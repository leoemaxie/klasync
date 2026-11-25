//! Google Gemini adapter (native `generateContent` API).
//!
//! The native API is used instead of the OpenAI-compatible layer because it is
//! the only way to get JSON mode, thinking control (thinking tokens are billed
//! as output), audio transcription, embeddings and batch from one client.

use std::time::Duration;

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use futures_util::{stream, StreamExt};
use reqwest::{Client, Response, StatusCode};
use serde_json::{json, Value};

use super::provider::{BatchProvider, Embedder, TextProvider, Transcriber};
use super::types::*;
use crate::config::AppConfig;

const PROVIDER: &str = "gemini";
/// Audio above this size goes through the Files API instead of inline base64.
const DEFAULT_INLINE_LIMIT: usize = 14 * 1024 * 1024;
/// `gemini-3.5-transcribe` accepts ~98k input tokens; audio is 25 tokens/s.
const TRANSCRIBE_MAX_SECONDS: u32 = 3_300;

pub struct GeminiProvider {
    client: Client,
    base: String,
    key: String,
    cheap: Vec<String>,
    mid: Vec<String>,
    strong: Vec<String>,
    transcribe_model: String,
    long_audio_model: String,
    embedding_model: String,
    dimensions: usize,
    inline_limit: usize,
}

impl GeminiProvider {
    pub fn from_config(config: &AppConfig) -> Option<Self> {
        let key = config.gemini_api_key.clone()?;
        Some(Self {
            client: super::client_builder(config)
                .timeout(Duration::from_secs(240))
                .build()
                .ok()?,
            base: config.gemini_base_url.clone(),
            key,
            cheap: config.gemini_models_cheap.clone(),
            mid: config.gemini_models_mid.clone(),
            strong: config.gemini_models_strong.clone(),
            transcribe_model: config.gemini_transcribe_model.clone(),
            long_audio_model: config.gemini_long_audio_model.clone(),
            embedding_model: config.gemini_embedding_model.clone(),
            dimensions: config.embedding_dimensions,
            inline_limit: DEFAULT_INLINE_LIMIT,
        })
    }

    #[cfg(test)]
    pub fn with_inline_limit(mut self, bytes: usize) -> Self {
        self.inline_limit = bytes;
        self
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base, path.trim_start_matches('/'))
    }

    fn post(&self, path: &str) -> reqwest::RequestBuilder {
        self.client
            .post(self.url(path))
            .header("x-goog-api-key", &self.key)
            .header(reqwest::header::USER_AGENT, "klasync-api/0.1")
    }

    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.client
            .get(self.url(path))
            .header("x-goog-api-key", &self.key)
            .header(reqwest::header::USER_AGENT, "klasync-api/0.1")
    }

    fn chain(&self, tier: Tier) -> &Vec<String> {
        match tier {
            Tier::Cheap => &self.cheap,
            Tier::Mid => &self.mid,
            Tier::Strong => &self.strong,
        }
    }
}

// ---- request / response shaping --------------------------------------------

/// `thinkingConfig` differs by model generation: 2.5 takes a token budget,
/// 3.x and 4 take a named level (and Gemini 4 rejects `minimal`).
fn thinking_config(model: &str, level: Thinking) -> Option<Value> {
    if model.starts_with("gemini-2.5") {
        let budget = match level {
            Thinking::Minimal => 0,
            Thinking::Low => 1024,
            Thinking::Medium => 8192,
            Thinking::High => -1,
        };
        return Some(json!({ "thinkingBudget": budget }));
    }
    if model.starts_with("gemini-3") || model.starts_with("gemini-4") {
        let name = match (level, model.starts_with("gemini-4")) {
            (Thinking::Minimal, true) => "low",
            (Thinking::Minimal, false) => "minimal",
            (Thinking::Low, _) => "low",
            (Thinking::Medium, _) => "medium",
            (Thinking::High, _) => "high",
        };
        return Some(json!({ "thinkingLevel": name }));
    }
    None
}

/// The body of a `generateContent` request; also reused as one batch item.
pub fn build_generate_body(model: &str, request: &TextRequest, with_thinking: bool) -> Value {
    let mut config = serde_json::Map::new();
    if let Some(max) = request.max_output_tokens {
        config.insert("maxOutputTokens".into(), json!(max));
    }
    if request.json {
        config.insert("responseMimeType".into(), json!("application/json"));
    }
    // Newer models removed temperature/top_p/top_k, so only send it to 2.x.
    if let Some(temperature) = request.temperature {
        if model.starts_with("gemini-2.") {
            config.insert("temperature".into(), json!(temperature));
        }
    }
    if with_thinking {
        if let Some(thinking) = thinking_config(model, request.thinking) {
            config.insert("thinkingConfig".into(), thinking);
        }
    }
    let mut body = json!({
        "contents": [{ "role": "user", "parts": [{ "text": request.input }] }],
        "generationConfig": Value::Object(config),
    });
    if !request.system.trim().is_empty() {
        body["systemInstruction"] = json!({ "parts": [{ "text": request.system }] });
    }
    body
}

fn parse_usage(value: &Value) -> Usage {
    let number = |key: &str| value.get(key).and_then(Value::as_u64).unwrap_or(0);
    let prompt = number("promptTokenCount");
    let candidates = number("candidatesTokenCount");
    let thoughts = number("thoughtsTokenCount");
    let total = number("totalTokenCount");
    // Thinking tokens are billed as output but reported separately; the total
    // is the source of truth for what was billed.
    let mut output = candidates + thoughts;
    if total > prompt && total - prompt > output && candidates > 0 {
        output = total - prompt;
    }
    let audio = value
        .get("promptTokensDetails")
        .and_then(Value::as_array)
        .map(|details| {
            details
                .iter()
                .filter(|detail| detail.get("modality").and_then(Value::as_str) == Some("AUDIO"))
                .filter_map(|detail| detail.get("tokenCount").and_then(Value::as_u64))
                .sum()
        })
        .unwrap_or(0);
    Usage {
        input_tokens: prompt,
        output_tokens: output,
        cached_tokens: number("cachedContentTokenCount"),
        thinking_tokens: thoughts,
        audio_input_tokens: audio,
        reported_cost_usd: None,
    }
}

pub fn parse_generate_response(model: &str, payload: &Value) -> Result<TextOutput, AiError> {
    let candidate = payload.pointer("/candidates/0");
    let Some(candidate) = candidate else {
        let reason = payload
            .pointer("/promptFeedback/blockReason")
            .and_then(Value::as_str)
            .unwrap_or("no candidates returned");
        return Err(AiError::Rejected(format!("request blocked: {reason}")));
    };
    let text: String = candidate
        .pointer("/content/parts")
        .and_then(Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter(|part| part.get("thought").and_then(Value::as_bool) != Some(true))
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    let truncated = candidate.get("finishReason").and_then(Value::as_str) == Some("MAX_TOKENS");
    if text.trim().is_empty() {
        return Err(AiError::InvalidOutput(if truncated {
            "the model ran out of output tokens before answering".to_owned()
        } else {
            "the model returned an empty response".to_owned()
        }));
    }
    Ok(TextOutput {
        text,
        usage: payload
            .get("usageMetadata")
            .map(parse_usage)
            .unwrap_or_default(),
        model: payload
            .get("modelVersion")
            .and_then(Value::as_str)
            .unwrap_or(model)
            .to_owned(),
        provider: PROVIDER,
        truncated,
    })
}

fn retry_delay(body: &Value) -> Option<Duration> {
    body.pointer("/error/details")
        .and_then(Value::as_array)?
        .iter()
        .find_map(|detail| detail.get("retryDelay").and_then(Value::as_str))
        .and_then(|delay| delay.trim_end_matches('s').parse::<f64>().ok())
        .map(Duration::from_secs_f64)
}

async fn classify(model: &str, response: Response) -> AiError {
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    let body: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    let message = body
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or(text.as_str())
        .chars()
        .take(300)
        .collect::<String>();
    match status {
        StatusCode::TOO_MANY_REQUESTS => AiError::RateLimited(retry_delay(&body)),
        StatusCode::PAYMENT_REQUIRED => AiError::PaymentRequired(message),
        StatusCode::NOT_FOUND => AiError::ModelUnavailable(format!("{model}: {message}")),
        status if status.is_server_error() => AiError::Upstream(format!("{status} {message}")),
        status => AiError::Rejected(format!("{} {message}", status.as_u16())),
    }
}

fn is_thinking_rejection(error: &AiError) -> bool {
    matches!(error, AiError::Rejected(message)
        if message.to_ascii_lowercase().contains("thinking"))
}

#[async_trait]
impl TextProvider for GeminiProvider {
    fn name(&self) -> &'static str {
        PROVIDER
    }

    fn models_for(&self, _task: Task, tier: Tier) -> Vec<String> {
        self.chain(tier).clone()
    }

    // 1M-token windows: a 2-hour lecture (~90k characters) fits many times over.
    fn context_chars(&self) -> usize {
        600_000
    }

    async fn generate(&self, model: &str, request: &TextRequest) -> Result<TextOutput, AiError> {
        let mut with_thinking = true;
        loop {
            let body = build_generate_body(model, request, with_thinking);
            let response = self
                .post(&format!("v1beta/models/{model}:generateContent"))
                .json(&body)
                .send()
                .await?;
            if !response.status().is_success() {
                let error = classify(model, response).await;
                // A model that predates the thinking parameter: retry once without it.
                if with_thinking && is_thinking_rejection(&error) {
                    with_thinking = false;
                    continue;
                }
                return Err(error);
            }
            let payload: Value = response
                .json()
                .await
                .map_err(|error| AiError::Decode(error.to_string()))?;
            return parse_generate_response(model, &payload);
        }
    }
}

// ---- transcription ----------------------------------------------------------

fn normalise_mime(mime: &str) -> String {
    let base = mime
        .split(';')
        .next()
        .unwrap_or(mime)
        .trim()
        .to_ascii_lowercase();
    match base.as_str() {
        "audio/x-wav" | "audio/wave" | "audio/vnd.wave" => "audio/wav".to_owned(),
        "audio/mpeg3" | "audio/x-mpeg-3" | "audio/mpeg" => "audio/mp3".to_owned(),
        "audio/x-m4a" | "audio/m4a" => "audio/aac".to_owned(),
        "" => "audio/webm".to_owned(),
        other => other.to_owned(),
    }
}

impl GeminiProvider {
    /// Uploads large audio through the resumable Files API and waits until the
    /// file is processed. Returns the file URI to reference in a request.
    async fn upload_file(&self, bytes: &[u8], mime: &str) -> Result<String, AiError> {
        let start = self
            .client
            .post(self.url("upload/v1beta/files"))
            .header("x-goog-api-key", &self.key)
            .header("X-Goog-Upload-Protocol", "resumable")
            .header("X-Goog-Upload-Command", "start")
            .header(
                "X-Goog-Upload-Header-Content-Length",
                bytes.len().to_string(),
            )
            .header("X-Goog-Upload-Header-Content-Type", mime)
            .json(&json!({ "file": { "display_name": "klasync-lecture-audio" } }))
            .send()
            .await?;
        if !start.status().is_success() {
            return Err(classify("files", start).await);
        }
        let upload_url = start
            .headers()
            .get("x-goog-upload-url")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
            .ok_or_else(|| AiError::Decode("file upload did not return an upload URL".into()))?;
        let finish = self
            .client
            .post(&upload_url)
            .header("Content-Length", bytes.len().to_string())
            .header("X-Goog-Upload-Offset", "0")
            .header("X-Goog-Upload-Command", "upload, finalize")
            .body(bytes.to_vec())
            .send()
            .await?;
        if !finish.status().is_success() {
            return Err(classify("files", finish).await);
        }
        let payload: Value = finish
            .json()
            .await
            .map_err(|error| AiError::Decode(error.to_string()))?;
        let file = payload.get("file").unwrap_or(&payload);
        let uri = file
            .get("uri")
            .and_then(Value::as_str)
            .ok_or_else(|| AiError::Decode("uploaded file has no URI".into()))?
            .to_owned();
        let name = file.get("name").and_then(Value::as_str).map(str::to_owned);
        let mut state = file
            .get("state")
            .and_then(Value::as_str)
            .unwrap_or("ACTIVE")
            .to_owned();
        for _ in 0..30 {
            if state == "ACTIVE" {
                return Ok(uri);
            }
            if state == "FAILED" {
                return Err(AiError::Rejected(
                    "the audio file could not be processed".into(),
                ));
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
            let Some(name) = &name else { break };
            let current: Value = self
                .get(&format!("v1beta/{name}"))
                .send()
                .await?
                .json()
                .await
                .map_err(|error| AiError::Decode(error.to_string()))?;
            state = current
                .get("state")
                .and_then(Value::as_str)
                .unwrap_or("ACTIVE")
                .to_owned();
        }
        Ok(uri)
    }
}

#[async_trait]
impl Transcriber for GeminiProvider {
    fn name(&self) -> &'static str {
        PROVIDER
    }

    fn models_for(&self, duration_sec: Option<u32>) -> Vec<String> {
        // The dedicated transcribe model holds ~55 minutes; anything longer (or
        // when it fails) falls to a long-context general model.
        match duration_sec {
            Some(seconds) if seconds > TRANSCRIBE_MAX_SECONDS => {
                vec![self.long_audio_model.clone()]
            }
            _ => vec![self.transcribe_model.clone(), self.long_audio_model.clone()],
        }
    }

    async fn transcribe(
        &self,
        model: &str,
        request: &TranscribeRequest,
    ) -> Result<TextOutput, AiError> {
        let mime = normalise_mime(&request.mime);
        let audio_part = if request.audio.len() <= self.inline_limit {
            json!({ "inlineData": { "mimeType": mime, "data": STANDARD.encode(&request.audio) } })
        } else {
            let uri = self.upload_file(&request.audio, &mime).await?;
            json!({ "fileData": { "mimeType": mime, "fileUri": uri } })
        };
        let language = request.language.as_deref().unwrap_or("English");
        let instruction = if request.timestamps {
            format!(
                "Transcribe this university lecture recording verbatim in {language}. \
                 Output plain text only. Start a new paragraph roughly every 30 seconds and begin each \
                 paragraph with its start time in [mm:ss] format, for example [00:00] or [00:30]. \
                 Do not add commentary, headings or speaker labels."
            )
        } else {
            format!(
                "Transcribe this audio verbatim in {language}. Output the spoken words only, as plain \
                 text, with no timestamps, commentary or speaker labels."
            )
        };
        let body = json!({
            "contents": [{ "role": "user", "parts": [audio_part, { "text": instruction }] }],
            "generationConfig": { "maxOutputTokens": 32000 },
        });
        let response = self
            .post(&format!("v1beta/models/{model}:generateContent"))
            .json(&body)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(classify(model, response).await);
        }
        let payload: Value = response
            .json()
            .await
            .map_err(|error| AiError::Decode(error.to_string()))?;
        parse_generate_response(model, &payload)
    }
}

// ---- embeddings -------------------------------------------------------------

fn l2_normalise(vector: &mut [f32]) {
    let norm = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        vector.iter_mut().for_each(|v| *v /= norm);
    }
}

#[async_trait]
impl Embedder for GeminiProvider {
    fn name(&self) -> &'static str {
        PROVIDER
    }

    fn model(&self) -> String {
        self.embedding_model.clone()
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    async fn embed(&self, items: &[EmbedItem], kind: EmbedKind) -> Result<Embeddings, AiError> {
        // `gemini-embedding-2` has no task_type; the task goes in the text.
        let legacy = self.embedding_model.ends_with("-001");
        let model = self.embedding_model.clone();
        let dimensions = self.dimensions;
        let results = stream::iter(items.iter().cloned().enumerate())
            .map(|(index, item)| {
                let model = model.clone();
                async move {
                    let text = if legacy {
                        item.text.clone()
                    } else {
                        match kind {
                            EmbedKind::Document => format!(
                                "title: {} | text: {}",
                                item.title.as_deref().unwrap_or("none"),
                                item.text
                            ),
                            EmbedKind::Query => {
                                format!("task: question answering | query: {}", item.text)
                            }
                        }
                    };
                    let mut body = json!({
                        "content": { "parts": [{ "text": text }] },
                        "outputDimensionality": dimensions,
                    });
                    if legacy {
                        body["taskType"] = json!(match kind {
                            EmbedKind::Document => "RETRIEVAL_DOCUMENT",
                            EmbedKind::Query => "QUESTION_ANSWERING",
                        });
                        if let (EmbedKind::Document, Some(title)) = (kind, &item.title) {
                            body["title"] = json!(title);
                        }
                    }
                    let mut attempt = 0;
                    loop {
                        let response = self
                            .post(&format!("v1beta/models/{model}:embedContent"))
                            .json(&body)
                            .send()
                            .await;
                        let outcome: Result<(Vec<f32>, u64), AiError> = match response {
                            Ok(response) if response.status().is_success() => {
                                let payload: Value = response
                                    .json()
                                    .await
                                    .map_err(|error| AiError::Decode(error.to_string()))?;
                                let values: Vec<f32> = payload
                                    .pointer("/embedding/values")
                                    .and_then(Value::as_array)
                                    .map(|values| {
                                        values
                                            .iter()
                                            .filter_map(|v| v.as_f64().map(|f| f as f32))
                                            .collect()
                                    })
                                    .unwrap_or_default();
                                let tokens = payload
                                    .pointer("/usageMetadata/promptTokenCount")
                                    .and_then(Value::as_u64)
                                    .unwrap_or(0);
                                Ok((values, tokens))
                            }
                            Ok(response) => Err(classify(&model, response).await),
                            Err(error) => Err(error.into()),
                        };
                        match outcome {
                            Err(error) if error.is_transient() && attempt < 3 => {
                                attempt += 1;
                                let pause = match &error {
                                    AiError::RateLimited(Some(delay)) => *delay,
                                    _ => Duration::from_secs(2u64.pow(attempt)),
                                };
                                tokio::time::sleep(pause.min(Duration::from_secs(20))).await;
                            }
                            other => return other.map(|value| (index, value)),
                        }
                    }
                }
            })
            .buffer_unordered(6)
            .collect::<Vec<_>>()
            .await;

        let mut vectors = vec![Vec::new(); items.len()];
        let mut tokens = 0u64;
        for result in results {
            let (index, (mut values, used)) = result?;
            if values.len() != dimensions {
                return Err(AiError::Decode(format!(
                    "expected {dimensions} dimensions, the model returned {}",
                    values.len()
                )));
            }
            l2_normalise(&mut values);
            vectors[index] = values;
            tokens += used;
        }
        Ok(Embeddings {
            vectors,
            model,
            dimensions,
            usage: Usage {
                input_tokens: tokens,
                ..Usage::default()
            },
        })
    }
}

// ---- batch ------------------------------------------------------------------

#[async_trait]
impl BatchProvider for GeminiProvider {
    fn name(&self) -> &'static str {
        PROVIDER
    }

    fn model_for(&self, _task: Task, tier: Tier) -> String {
        self.chain(tier).first().cloned().unwrap_or_default()
    }

    async fn submit(&self, model: &str, items: &[BatchItem]) -> Result<String, AiError> {
        let requests: Vec<Value> = items
            .iter()
            .map(|item| {
                json!({
                    "request": build_generate_body(model, &item.request, true),
                    "metadata": { "key": item.key },
                })
            })
            .collect();
        let body = json!({
            "batch": {
                "display_name": format!("klasync-{}", chrono::Utc::now().format("%Y%m%d-%H%M%S")),
                "input_config": { "requests": { "requests": requests } },
            }
        });
        let response = self
            .post(&format!("v1beta/models/{model}:batchGenerateContent"))
            .json(&body)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(classify(model, response).await);
        }
        let payload: Value = response
            .json()
            .await
            .map_err(|error| AiError::Decode(error.to_string()))?;
        payload
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| AiError::Decode("batch response had no name".into()))
    }

    async fn poll(&self, batch_id: &str) -> Result<BatchState, AiError> {
        let response = self.get(&format!("v1beta/{batch_id}")).send().await?;
        if !response.status().is_success() {
            return Err(classify("batch", response).await);
        }
        let payload: Value = response
            .json()
            .await
            .map_err(|error| AiError::Decode(error.to_string()))?;
        parse_batch_status(&payload)
    }
}

pub fn parse_batch_status(payload: &Value) -> Result<BatchState, AiError> {
    let state = payload
        .pointer("/metadata/state")
        .or_else(|| payload.get("state"))
        .and_then(Value::as_str)
        .unwrap_or("BATCH_STATE_RUNNING");
    match state {
        "BATCH_STATE_PENDING" | "BATCH_STATE_RUNNING" => Ok(BatchState::Running),
        "BATCH_STATE_CANCELLED" => Ok(BatchState::Cancelled),
        "BATCH_STATE_EXPIRED" => Ok(BatchState::Expired),
        "BATCH_STATE_FAILED" => Ok(BatchState::Failed(
            payload
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or("the batch failed")
                .to_owned(),
        )),
        "BATCH_STATE_SUCCEEDED" => {
            let response = payload.get("response").unwrap_or(payload);
            let inlined = response
                .pointer("/inlinedResponses/inlinedResponses")
                .or_else(|| response.get("inlinedResponses"))
                .or_else(|| response.pointer("/inlined_responses/inlined_responses"))
                .and_then(Value::as_array)
                .ok_or_else(|| AiError::Decode("batch finished without inline responses".into()))?;
            let model = payload
                .pointer("/metadata/model")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim_start_matches("models/")
                .to_owned();
            let results = inlined
                .iter()
                .map(|item| {
                    let key = item
                        .pointer("/metadata/key")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    let outcome = match item.get("response") {
                        Some(response) => parse_generate_response(&model, response),
                        None => Err(AiError::Rejected(
                            item.pointer("/error/message")
                                .and_then(Value::as_str)
                                .unwrap_or("batch item failed")
                                .to_owned(),
                        )),
                    };
                    (key, outcome)
                })
                .collect();
            Ok(BatchState::Succeeded(results))
        }
        other => Err(AiError::Decode(format!("unknown batch state {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thinking_config_follows_model_generation() {
        assert_eq!(
            thinking_config("gemini-2.5-flash", Thinking::Minimal),
            Some(json!({"thinkingBudget": 0}))
        );
        assert_eq!(
            thinking_config("gemini-3.5-flash-lite", Thinking::Minimal),
            Some(json!({"thinkingLevel": "minimal"}))
        );
        // Gemini 4 rejects `minimal`.
        assert_eq!(
            thinking_config("gemini-4-hydrogen", Thinking::Minimal),
            Some(json!({"thinkingLevel": "low"}))
        );
        assert_eq!(thinking_config("gemma-4-31b-it", Thinking::Low), None);
    }

    #[test]
    fn usage_bills_thinking_as_output() {
        let usage = parse_usage(&json!({
            "promptTokenCount": 13, "candidatesTokenCount": 221,
            "thoughtsTokenCount": 560, "totalTokenCount": 794,
            "promptTokensDetails": [{"modality":"AUDIO","tokenCount":10}]
        }));
        assert_eq!(usage.input_tokens, 13);
        assert_eq!(usage.output_tokens, 781);
        assert_eq!(usage.thinking_tokens, 560);
        assert_eq!(usage.audio_input_tokens, 10);
    }

    #[test]
    fn parses_text_and_skips_thought_parts() {
        let payload = json!({"candidates":[{"content":{"parts":[
            {"text":"reasoning","thought":true},{"text":"answer"}]},"finishReason":"STOP"}],
            "usageMetadata":{"promptTokenCount":1,"candidatesTokenCount":1,"totalTokenCount":2}});
        let out = parse_generate_response("m", &payload).unwrap();
        assert_eq!(out.text, "answer");
        assert!(!out.truncated);
    }

    #[test]
    fn truncated_empty_answer_is_invalid_output() {
        let payload = json!({"candidates":[{"content":{"parts":[]},"finishReason":"MAX_TOKENS"}]});
        assert!(matches!(
            parse_generate_response("m", &payload),
            Err(AiError::InvalidOutput(_))
        ));
    }

    #[test]
    fn batch_results_are_keyed() {
        let payload = json!({"metadata":{"state":"BATCH_STATE_SUCCEEDED","model":"models/gemini-3.5-flash-lite"},
          "response":{"inlinedResponses":{"inlinedResponses":[
            {"metadata":{"key":"a"},"response":{"candidates":[{"content":{"parts":[{"text":"x"}]}}]}},
            {"metadata":{"key":"b"},"error":{"message":"bad"}}]}}});
        match parse_batch_status(&payload).unwrap() {
            BatchState::Succeeded(items) => {
                assert_eq!(items.len(), 2);
                assert!(items[0].1.is_ok());
                assert!(items[1].1.is_err());
            }
            _ => panic!("expected success"),
        }
    }

    #[test]
    fn mime_types_are_normalised() {
        assert_eq!(normalise_mime("audio/webm;codecs=opus"), "audio/webm");
        assert_eq!(normalise_mime("audio/x-wav"), "audio/wav");
        assert_eq!(normalise_mime("audio/mpeg"), "audio/mp3");
    }

    /// Live: audio above the inline limit goes through the Files API.
    /// `GEMINI_API_KEY=... cargo test --lib files_api -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn files_api_upload_path_transcribes_large_audio() {
        let config = crate::config::AppConfig::from_env();
        let provider = GeminiProvider::from_config(&config)
            .expect("GEMINI_API_KEY must be set")
            .with_inline_limit(1024);
        let audio =
            std::fs::read(std::env::var("GEMINI_TEST_WAV").unwrap_or("/tmp/lecture.wav".into()))
                .expect("test audio");
        assert!(audio.len() > 1024);
        let out = Transcriber::transcribe(
            &provider,
            "gemini-3.5-transcribe",
            &TranscribeRequest {
                audio,
                mime: "audio/wav".into(),
                language: None,
                duration_sec: None,
                timestamps: false,
                session_id: None,
                job_id: None,
            },
        )
        .await
        .expect("transcription via Files API");
        println!("{}", out.text);
        assert!(out.text.to_lowercase().contains("photosynthesis"));
    }
}
