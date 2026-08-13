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

    fn is_light(self) -> bool {
        matches!(
            self,
            Self::Summary | Self::Flashcards | Self::Explanation | Self::QuestionAnswer
        )
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
                    current.saturating_add(1_000),
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                return Ok(());
            }
        }
    }

    fn record_cost(&self, cost: f64) {
        self.spent_usd_micros
            .fetch_add((cost.max(0.0) * 1_000_000.0) as u64, Ordering::SeqCst);
    }

    async fn chat(&self, work: &AiWorkItem, model: &str) -> Result<AiWorkResult, AiAdapterError> {
        let input = work
            .input
            .get("prompt")
            .and_then(|value| value.as_str())
            .or_else(|| work.input.get("text").and_then(|value| value.as_str()))
            .unwrap_or("");
        let system = work
            .input
            .get("system")
            .and_then(|value| value.as_str())
            .unwrap_or(
            "You are KLASYNC, an accessibility-first learning assistant. Be concise and accurate.",
        );
        let body = serde_json::json!({
            "model": model,
            "messages": [{"role":"system","content":system},{"role":"user","content":input}],
            "max_tokens": self.max_output_tokens,
            "temperature": 0.2,
        });
        let response = self
            .bearer(
                self.client
                    .post(format!("{}/chat/completions", self.endpoint))
                    .json(&body),
            )
            .send()
            .await?;
        if response.status() == StatusCode::TOO_MANY_REQUESTS && self.allow_paid_fallback {
            if let Some(fallback) = &self.paid_fallback_model {
                return self.chat_with_model(work, fallback).await;
            }
        }
        if !response.status().is_success() {
            return Err(AiAdapterError::Rejected(
                response.text().await.unwrap_or_default(),
            ));
        }
        let payload: ChatResponse = response
            .json()
            .await
            .map_err(|error| AiAdapterError::Decode(error.to_string()))?;
        self.record_cost(
            payload
                .usage
                .as_ref()
                .and_then(|usage| usage.cost)
                .unwrap_or(0.0),
        );
        let content = payload
            .choices
            .first()
            .and_then(|choice| choice.message.content.clone())
            .unwrap_or_default();
        Ok(AiWorkResult {
            content: serde_json::json!({"text": content}),
            metadata: serde_json::json!({"model": model, "usage": payload.usage}),
        })
    }

    async fn chat_with_model(
        &self,
        work: &AiWorkItem,
        model: &str,
    ) -> Result<AiWorkResult, AiAdapterError> {
        // Keep the fallback explicit and bounded; this path is never reached unless
        // OPENROUTER_ALLOW_PAID_FALLBACK=true.
        let input = work
            .input
            .get("prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let body = serde_json::json!({"model":model,"messages":[{"role":"user","content":input}],"max_tokens":self.max_output_tokens,"temperature":0.2});
        let response = self
            .bearer(
                self.client
                    .post(format!("{}/chat/completions", self.endpoint))
                    .json(&body),
            )
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(AiAdapterError::Rejected(
                response.text().await.unwrap_or_default(),
            ));
        }
        let payload: ChatResponse = response
            .json()
            .await
            .map_err(|error| AiAdapterError::Decode(error.to_string()))?;
        self.record_cost(
            payload
                .usage
                .as_ref()
                .and_then(|usage| usage.cost)
                .unwrap_or(0.0),
        );
        let content = payload
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();
        Ok(AiWorkResult {
            content: serde_json::json!({"text": content}),
            metadata: serde_json::json!({"model": model, "usage": payload.usage}),
        })
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
            return Err(AiAdapterError::Rejected(
                response.text().await.unwrap_or_default(),
            ));
        }
        let payload: TranscriptionResponse = response
            .json()
            .await
            .map_err(|error| AiAdapterError::Decode(error.to_string()))?;
        self.record_cost(
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
        if task.is_light() && self.model_for(task) != "openrouter/free" {
            // A configured model is respected, but the defaults in AppConfig are
            // free. This branch is intentionally observable in metadata.
        }
        match task {
            TaskKind::Transcript => self.transcribe(&work).await,
            _ => self.chat(&work, self.model_for(task)).await,
        }
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

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(default)]
struct ChatResponse {
    #[serde(default)]
    choices: Vec<ChatChoice>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(default)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(default)]
struct ChatMessage {
    content: Option<String>,
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
