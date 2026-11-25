//! Cost-aware, provider-neutral AI boundary.
//!
//! KLASYNC talks to OpenRouter here, while the rest of the application only
//! depends on `AiAdapter`. Model selection is task-specific and configurable;

use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::config::AppConfig;

pub type SharedAiAdapter = Arc<dyn AiAdapter>;

/// Amount provisionally held per in-flight request (USD micros).
const RESERVATION_MICROS: u64 = 1_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiWorkItem {
    pub job_id: Uuid,
    pub session_id: Uuid,
    pub job_type: String,
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiWorkResult {
    pub content: serde_json::Value,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Error)]
pub enum AiAdapterError {
    #[error("AI provider is not configured")]
    Unavailable,
    #[error("AI request exceeds the configured cost guard")]
    BudgetExceeded,
    #[error("AI provider rejected the request: {0}")]
    Rejected(String),
    #[error("AI provider request failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("AI response could not be decoded: {0}")]
    Decode(String),
}

#[async_trait]
pub trait AiAdapter: Send + Sync {
    async fn execute(&self, work: AiWorkItem) -> Result<AiWorkResult, AiAdapterError>;
    fn provider_name(&self) -> &'static str;
}

#[derive(Debug, Clone, Copy)]
enum TaskKind {
    Transcript,
    Summary,
    Flashcards,
    Explanation,
    QuestionAnswer,
    Other,
}

impl TaskKind {
    fn parse(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "transcribe" | "transcription" => Self::Transcript,
            "summarize" | "summary" => Self::Summary,
            "flashcards" | "flashcard" => Self::Flashcards,
            "explain" | "explanation" => Self::Explanation,
            "question_answer" | "question-answer" | "qa" => Self::QuestionAnswer,
            _ => Self::Other,
        }
    }
}

pub struct OpenRouterAdapter {
    client: Client,
    endpoint: String,
    api_key: String,
    transcription_model: String,
    summary_model: String,
    flashcard_model: String,
    explanation_model: String,
    qa_model: String,
    paid_fallback_model: Option<String>,
    allow_paid_fallback: bool,
    max_output_tokens: u32,
    max_cost_usd_micros: u64,
    spent_usd_micros: AtomicU64,
}

impl OpenRouterAdapter {
    pub fn from_config(config: &AppConfig) -> Option<Self> {
        Some(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(90))
                .build()
                .ok()?,
            endpoint: config.openrouter_base_url.trim_end_matches('/').to_owned(),
            api_key: config.openrouter_api_key.clone()?,
            transcription_model: config.openrouter_transcription_model.clone(),
            summary_model: config.openrouter_summary_model.clone(),
            flashcard_model: config.openrouter_flashcard_model.clone(),
            explanation_model: config.openrouter_explanation_model.clone(),
            qa_model: config.openrouter_qa_model.clone(),
            paid_fallback_model: config.openrouter_paid_fallback_model.clone(),
            allow_paid_fallback: config.openrouter_allow_paid_fallback,
            max_output_tokens: config.ai_max_output_tokens,
            max_cost_usd_micros: (config.ai_max_cost_usd.max(0.0) * 1_000_000.0) as u64,
            spent_usd_micros: AtomicU64::new(0),
        })
    }

    fn model_for(&self, task: TaskKind) -> &str {
        match task {
            TaskKind::Transcript => &self.transcription_model,
            TaskKind::Summary => &self.summary_model,
            TaskKind::Flashcards => &self.flashcard_model,
            TaskKind::Explanation => &self.explanation_model,
            TaskKind::QuestionAnswer => &self.qa_model,
            TaskKind::Other => &self.summary_model,
        }
    }

    fn bearer(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request
            .bearer_auth(&self.api_key)
            .header("HTTP-Referer", "https://klasync.triumphsystems.tech")
            .header("X-Title", "KLASYNC Assistive Learning")
            .header(reqwest::header::USER_AGENT, "klasync-api/0.1")
    }

    /// Reserves a small amount before the call; `settle_cost` swaps the
    /// reservation for the real cost. Previously the reservation was never
    /// released, so after ~20 requests per process every AI call failed with
    /// `BudgetExceeded` until the server restarted.
    fn reserve_budget(&self) -> Result<(), AiAdapterError> {
        if self.max_cost_usd_micros == 0 {
            return Ok(());
        }
        loop {
            let current = self.spent_usd_micros.load(Ordering::Relaxed);
            if current >= self.max_cost_usd_micros {
                return Err(AiAdapterError::BudgetExceeded);
            }
            if self
                .spent_usd_micros
                .compare_exchange(
                    current,
                    current.saturating_add(RESERVATION_MICROS),
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                return Ok(());
            }
        }
    }

    fn settle_cost(&self, cost: f64) {
        let actual = (cost.max(0.0) * 1_000_000.0) as u64;
        // Remove the reservation, then add what was really spent.
        let _ = self
            .spent_usd_micros
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |value| {
                Some(value.saturating_sub(RESERVATION_MICROS))
            });
        self.spent_usd_micros.fetch_add(actual, Ordering::SeqCst);
    }

    fn release_reservation(&self) {
        let _ = self
            .spent_usd_micros
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |value| {
                Some(value.saturating_sub(RESERVATION_MICROS))
            });
    }

    async fn chat(&self, work: &AiWorkItem, model: &str) -> Result<AiWorkResult, AiAdapterError> {
        let input = work
            .input
            .get("prompt")
            .and_then(|value| value.as_str())
            .or_else(|| work.input.get("text").and_then(|value| value.as_str()))
            .unwrap_or("");
        if input.trim().is_empty() {
            return Err(AiAdapterError::Decode(
                "there is no lecture text to process yet".to_owned(),
            ));
        }
        let system = work
            .input
            .get("system")
            .and_then(|value| value.as_str())
            .unwrap_or(
            "You are KLASYNC, an accessibility-first learning assistant. Be concise and accurate.",
        );
        let max_tokens = work
            .input
            .get("max_tokens")
            .and_then(|value| value.as_u64())
            .map(|value| value as u32)
            .unwrap_or(self.max_output_tokens);
        let temperature = work
            .input
            .get("temperature")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.2);

        let mut models = vec![model.to_owned()];
        if self.allow_paid_fallback {
            if let Some(fallback) = &self.paid_fallback_model {
                if fallback != model {
                    models.push(fallback.clone());
                }
            }
        }

        let mut last_error = AiAdapterError::Unavailable;
        for candidate in models {
            match self
                .chat_once(&candidate, system, input, max_tokens, temperature)
                .await
            {
                Ok(result) => return Ok(result),
                Err(error) => {
                    tracing::warn!(model = %candidate, %error, "AI chat attempt failed");
                    last_error = error;
                }
            }
        }
        Err(last_error)
    }

    /// One model, with bounded retries for the transient failures that the
    /// shared free tier produces constantly (429 rate limits, 5xx, timeouts).
    async fn chat_once(
        &self,
        model: &str,
        system: &str,
        input: &str,
        max_tokens: u32,
        temperature: f64,
    ) -> Result<AiWorkResult, AiAdapterError> {
        let body = serde_json::json!({
            "model": model,
            "messages": [{"role":"system","content":system},{"role":"user","content":input}],
            "max_tokens": max_tokens,
            "temperature": temperature,
        });
        let backoff = [2u64, 6, 15];
        let mut attempt = 0usize;
        loop {
            let sent = self
                .bearer(
                    self.client
                        .post(format!("{}/chat/completions", self.endpoint))
                        .json(&body),
                )
                .send()
                .await;
            let response = match sent {
                Ok(response) => response,
                Err(error)
                    if attempt < backoff.len() && (error.is_timeout() || error.is_connect()) =>
                {
                    tokio::time::sleep(Duration::from_secs(backoff[attempt])).await;
                    attempt += 1;
                    continue;
                }
                Err(error) => return Err(error.into()),
            };
            let status = response.status();
            if (status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error())
                && attempt < backoff.len()
            {
                tokio::time::sleep(Duration::from_secs(backoff[attempt])).await;
                attempt += 1;
                continue;
            }
            if !status.is_success() {
                let text = response.text().await.unwrap_or_default();
                return Err(AiAdapterError::Rejected(format!(
                    "{} {}",
                    status.as_u16(),
                    text.chars().take(300).collect::<String>()
                )));
            }
            let payload: serde_json::Value = response
                .json()
                .await
                .map_err(|error| AiAdapterError::Decode(error.to_string()))?;
            // OpenRouter can answer 200 with an `error` object.
            if let Some(message) = payload
                .get("error")
                .and_then(|error| error.get("message"))
                .and_then(|message| message.as_str())
            {
                if attempt < backoff.len() {
                    tokio::time::sleep(Duration::from_secs(backoff[attempt])).await;
                    attempt += 1;
                    continue;
                }
                return Err(AiAdapterError::Rejected(message.to_owned()));
            }
            let usage: Option<Usage> = payload
                .get("usage")
                .and_then(|value| serde_json::from_value(value.clone()).ok());
            self.settle_cost(usage.as_ref().and_then(|u| u.cost).unwrap_or(0.0));
            let content = payload
                .pointer("/choices/0/message/content")
                .map(extract_text)
                .unwrap_or_default();
            if content.trim().is_empty() {
                if attempt < backoff.len() {
                    tokio::time::sleep(Duration::from_secs(backoff[attempt])).await;
                    attempt += 1;
                    // The reservation was already settled; re-reserve for the retry.
                    self.reserve_budget()?;
                    continue;
                }
                return Err(AiAdapterError::Decode(
                    "the model returned an empty response".to_owned(),
                ));
            }
            return Ok(AiWorkResult {
                content: serde_json::json!({"text": content}),
                metadata: serde_json::json!({"model": model, "usage": usage}),
            });
        }
    }

    async fn transcribe(&self, work: &AiWorkItem) -> Result<AiWorkResult, AiAdapterError> {
        let data = work
            .input
            .get("data")
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                AiAdapterError::Decode("transcription input requires base64 data".to_owned())
            })?;
        let format = work
            .input
            .get("format")
            .and_then(|value| value.as_str())
            .unwrap_or("wav");
        let body = serde_json::json!({"model":self.transcription_model,"input_audio":{"data":data,"format":format},"language":work.input.get("language").and_then(|v| v.as_str())});
        let response = self
            .bearer(
                self.client
                    .post(format!("{}/audio/transcriptions", self.endpoint))
                    .json(&body),
            )
            .send()
            .await?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(AiAdapterError::Rejected(format!(
                "{status} {}",
                text.chars().take(300).collect::<String>()
            )));
        }
        let payload: TranscriptionResponse = response
            .json()
            .await
            .map_err(|error| AiAdapterError::Decode(error.to_string()))?;
        self.settle_cost(
            payload
                .usage
                .as_ref()
                .and_then(|usage| usage.cost)
                .unwrap_or(0.0),
        );
        Ok(AiWorkResult {
            content: serde_json::json!({"text":payload.text}),
            metadata: serde_json::json!({"model":self.transcription_model,"usage":payload.usage}),
        })
    }
}

#[async_trait]
impl AiAdapter for OpenRouterAdapter {
    async fn execute(&self, work: AiWorkItem) -> Result<AiWorkResult, AiAdapterError> {
        self.reserve_budget()?;
        let task = TaskKind::parse(&work.job_type);
        let result = match task {
            TaskKind::Transcript => self.transcribe(&work).await,
            _ => self.chat(&work, self.model_for(task)).await,
        };
        if result.is_err() {
            self.release_reservation();
        }
        result
    }

    fn provider_name(&self) -> &'static str {
        "openrouter"
    }
}

pub struct UnconfiguredAiAdapter;

#[async_trait]
impl AiAdapter for UnconfiguredAiAdapter {
    async fn execute(&self, _: AiWorkItem) -> Result<AiWorkResult, AiAdapterError> {
        Err(AiAdapterError::Unavailable)
    }
    fn provider_name(&self) -> &'static str {
        "unconfigured"
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
struct Usage {
    cost: Option<f64>,
    total_tokens: Option<u64>,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(default)]
struct TranscriptionResponse {
    text: String,
    usage: Option<Usage>,
}

pub fn adapter_from_config(config: &AppConfig) -> SharedAiAdapter {
    OpenRouterAdapter::from_config(config)
        .map(|adapter| Arc::new(adapter) as SharedAiAdapter)
        .unwrap_or_else(|| Arc::new(UnconfiguredAiAdapter))
}

/// Converts raw audio bytes to the request shape expected by OpenRouter's
/// transcription endpoint. Keeping this helper here prevents handlers from
/// depending on OpenRouter-specific encoding details.
pub fn transcription_input(
    audio: &[u8],
    format: &str,
    language: Option<&str>,
) -> serde_json::Value {
    serde_json::json!({"data": STANDARD.encode(audio), "format": format, "language": language})
}

/// Chat `content` is usually a string but some providers return an array of
/// `{type:"text", text}` parts.
fn extract_text(content: &serde_json::Value) -> String {
    match content {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Array(parts) => parts
            .iter()
            .filter_map(|part| part.get("text").and_then(|text| text.as_str()))
            .collect::<Vec<_>>()
            .join(""),
        _ => String::new(),
    }
}

/// Models routinely wrap JSON in markdown fences or add a sentence before it.
/// Returns the first complete JSON array/object found in `raw`.
pub fn extract_json(raw: &str) -> Option<serde_json::Value> {
    let trimmed = raw.trim();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        return Some(value);
    }
    for (open, close) in [('[', ']'), ('{', '}')] {
        if let (Some(start), Some(end)) = (trimmed.find(open), trimmed.rfind(close)) {
            if end > start {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&trimmed[start..=end])
                {
                    return Some(value);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_fenced_and_prefixed_json() {
        let fenced = "```json\n[{\"a\":1}]\n```";
        assert!(extract_json(fenced).unwrap().is_array());
        let prefixed = "Here you go: [{\"a\":1},{\"a\":2}] hope it helps";
        assert_eq!(extract_json(prefixed).unwrap().as_array().unwrap().len(), 2);
        assert!(extract_json("no json here").is_none());
    }

    #[test]
    fn extracts_text_parts() {
        let parts = serde_json::json!([{"type":"text","text":"a"},{"type":"text","text":"b"}]);
        assert_eq!(extract_text(&parts), "ab");
    }
}
