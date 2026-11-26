//! OpenRouter adapter (OpenAI-compatible chat, transcription and embeddings).
//! Used when no Google key is configured, and as a last-resort fallback when
//! Gemini is unavailable.

use std::time::Duration;

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::{Client, Response, StatusCode};
use serde_json::{json, Value};

use super::provider::{Embedder, TextProvider, Transcriber};
use super::types::*;
use crate::config::AppConfig;

const PROVIDER: &str = "openrouter";

pub struct OpenRouterProvider {
    client: Client,
    endpoint: String,
    api_key: String,
    transcription_model: String,
    summary_model: String,
    flashcard_model: String,
    explanation_model: String,
    qa_model: String,
    embedding_model: String,
    paid_fallback_model: Option<String>,
    allow_paid_fallback: bool,
    max_output_tokens: u32,
    dimensions: usize,
}

impl OpenRouterProvider {
    pub fn from_config(config: &AppConfig) -> Option<Self> {
        Some(Self {
            client: super::client_builder(config)
                .timeout(Duration::from_secs(120))
                .build()
                .ok()?,
            endpoint: config.openrouter_base_url.trim_end_matches('/').to_owned(),
            api_key: config.openrouter_api_key.clone()?,
            transcription_model: config.openrouter_transcription_model.clone(),
            summary_model: config.openrouter_summary_model.clone(),
            flashcard_model: config.openrouter_flashcard_model.clone(),
            explanation_model: config.openrouter_explanation_model.clone(),
            qa_model: config.openrouter_qa_model.clone(),
            embedding_model: config.openrouter_embedding_model.clone(),
            paid_fallback_model: config.openrouter_paid_fallback_model.clone(),
            allow_paid_fallback: config.openrouter_allow_paid_fallback,
            max_output_tokens: config.ai_max_output_tokens,
            dimensions: config.embedding_dimensions,
        })
    }

    fn request(&self, path: &str) -> reqwest::RequestBuilder {
        self.client
            .post(format!("{}/{}", self.endpoint, path))
            .bearer_auth(&self.api_key)
            .header("HTTP-Referer", "https://klasync.triumphsystems.tech")
            .header("X-Title", "KLASYNC Assistive Learning")
            .header(reqwest::header::USER_AGENT, "klasync-api/0.1")
    }
}

async fn classify(model: &str, response: Response) -> AiError {
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    let message: String = serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|body| {
            body.pointer("/error/message")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or(text)
        .chars()
        .take(300)
        .collect();
    match status {
        StatusCode::TOO_MANY_REQUESTS => AiError::RateLimited(None),
        StatusCode::PAYMENT_REQUIRED => AiError::PaymentRequired(message),
        StatusCode::NOT_FOUND => AiError::ModelUnavailable(format!("{model}: {message}")),
        status if status.is_server_error() => AiError::Upstream(format!("{status} {message}")),
        status => AiError::Rejected(format!("{} {message}", status.as_u16())),
    }
}

/// Chat `content` is usually a string but some providers return `{text}` parts.
fn extract_text(content: &Value) -> String {
    match content {
        Value::String(text) => text.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(""),
        _ => String::new(),
    }
}

fn parse_usage(value: Option<&Value>) -> Usage {
    let Some(value) = value else {
        return Usage::default();
    };
    let number = |keys: &[&str]| {
        keys.iter()
            .find_map(|key| value.get(*key).and_then(Value::as_u64))
            .unwrap_or(0)
    };
    Usage {
        input_tokens: number(&["prompt_tokens", "input_tokens"]),
        output_tokens: number(&["completion_tokens", "output_tokens"]),
        cached_tokens: 0,
        thinking_tokens: 0,
        audio_input_tokens: 0,
        reported_cost_usd: value.get("cost").and_then(Value::as_f64),
    }
}

#[async_trait]
impl TextProvider for OpenRouterProvider {
    fn name(&self) -> &'static str {
        PROVIDER
    }

    fn models_for(&self, task: Task, _tier: Tier) -> Vec<String> {
        let primary = match task {
            Task::Flashcards => &self.flashcard_model,
            Task::Explain | Task::Quiz => &self.explanation_model,
            Task::Qa => &self.qa_model,
            _ => &self.summary_model,
        };
        let mut models = vec![primary.clone()];
        if self.allow_paid_fallback {
            if let Some(fallback) = &self.paid_fallback_model {
                if fallback != primary {
                    models.push(fallback.clone());
                }
            }
        }
        models
    }

    async fn generate(&self, model: &str, request: &TextRequest) -> Result<TextOutput, AiError> {
        if request.input.trim().is_empty() {
            return Err(AiError::NoInput(
                "there is no lecture text to process yet".to_owned(),
            ));
        }
        let system = if request.system.trim().is_empty() {
            "You are KLASYNC, an accessibility-first learning assistant. Be concise and accurate."
        } else {
            request.system.as_str()
        };
        let body = json!({
            "model": model,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": request.input},
            ],
            "max_tokens": request.max_output_tokens.unwrap_or(self.max_output_tokens),
            "temperature": request.temperature.unwrap_or(0.2),
        });
        let response = self.request("chat/completions").json(&body).send().await?;
        if !response.status().is_success() {
            return Err(classify(model, response).await);
        }
        let payload: Value = response
            .json()
            .await
            .map_err(|error| AiError::Decode(error.to_string()))?;
        // OpenRouter can answer 200 with an `error` object.
        if let Some(message) = payload.pointer("/error/message").and_then(Value::as_str) {
            return Err(AiError::Upstream(message.to_owned()));
        }
        let text = payload
            .pointer("/choices/0/message/content")
            .map(extract_text)
            .unwrap_or_default();
        if text.trim().is_empty() {
            return Err(AiError::InvalidOutput(
                "the model returned an empty response".to_owned(),
            ));
        }
        Ok(TextOutput {
            text,
            usage: parse_usage(payload.get("usage")),
            model: model.to_owned(),
            provider: PROVIDER,
            truncated: payload
                .pointer("/choices/0/finish_reason")
                .and_then(Value::as_str)
                == Some("length"),
        })
    }
}

#[async_trait]
impl Transcriber for OpenRouterProvider {
    fn name(&self) -> &'static str {
        PROVIDER
    }

    fn models_for(&self, _duration_sec: Option<u32>) -> Vec<String> {
        vec![self.transcription_model.clone()]
    }

    async fn transcribe(
        &self,
        model: &str,
        request: &TranscribeRequest,
    ) -> Result<TextOutput, AiError> {
        let format = request
            .mime
            .split(';')
            .next()
            .and_then(|mime| mime.split('/').nth(1))
            .map(|subtype| match subtype.trim() {
                "mpeg" | "mpeg3" => "mp3",
                "x-m4a" | "mp4" | "aac" => "m4a",
                "x-wav" | "wave" => "wav",
                other => other,
            })
            .unwrap_or("wav")
            .to_owned();
        let body = json!({
            "model": model,
            "input_audio": {"data": STANDARD.encode(&request.audio), "format": format},
            "language": request.language.as_deref().unwrap_or("en"),
        });
        let response = self
            .request("audio/transcriptions")
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
        let text = payload
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        Ok(TextOutput {
            text,
            usage: parse_usage(payload.get("usage")),
            model: model.to_owned(),
            provider: PROVIDER,
            truncated: false,
        })
    }
}

#[async_trait]
impl Embedder for OpenRouterProvider {
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
        let mut vectors = Vec::with_capacity(items.len());
        let mut usage = Usage::default();
        for batch in items.chunks(64) {
            let inputs: Vec<String> = batch
                .iter()
                .map(|item| match (kind, &item.title) {
                    (EmbedKind::Document, Some(title)) => format!("{title}\n\n{}", item.text),
                    _ => item.text.clone(),
                })
                .collect();
            let body = json!({
                "model": self.embedding_model,
                "input": inputs,
                "dimensions": self.dimensions,
            });
            let response = self.request("embeddings").json(&body).send().await?;
            if !response.status().is_success() {
                return Err(classify(&self.embedding_model, response).await);
            }
            let payload: Value = response
                .json()
                .await
                .map_err(|error| AiError::Decode(error.to_string()))?;
            let data = payload
                .get("data")
                .and_then(Value::as_array)
                .ok_or_else(|| AiError::Decode("embedding response had no data".into()))?;
            for row in data {
                let values: Vec<f32> = row
                    .get("embedding")
                    .and_then(Value::as_array)
                    .map(|values| {
                        values
                            .iter()
                            .filter_map(|v| v.as_f64().map(|f| f as f32))
                            .collect()
                    })
                    .unwrap_or_default();
                if values.len() != self.dimensions {
                    return Err(AiError::Decode(format!(
                        "expected {} dimensions, the model returned {}",
                        self.dimensions,
                        values.len()
                    )));
                }
                vectors.push(values);
            }
            usage.input_tokens += parse_usage(payload.get("usage")).input_tokens;
        }
        Ok(Embeddings {
            vectors,
            model: self.embedding_model.clone(),
            dimensions: self.dimensions,
            usage,
        })
    }
}
