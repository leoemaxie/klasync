//! Provider-neutral boundary for KLASYNC learning intelligence.
//!
//! API handlers create durable `ai_jobs`; this module knows nothing about a
//! particular model vendor. A separately deployable gateway can use any model,
//! local inference runtime, or orchestration service while preserving this
//! stable contract.

use std::sync::Arc;

use async_trait::async_trait;
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
    /// Examples: `transcribe`, `summarize`, `flashcards`, `explain`, `question_answer`.
    pub job_type: String,
    /// Persisted resource keys or transcript references; never raw client secrets.
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiWorkResult {
    pub content: serde_json::Value,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Error)]
pub enum AiAdapterError {
    #[error("AI delivery is not configured")]
    Unavailable,
    #[error("AI gateway rejected the request: {0}")]
    Rejected(String),
    #[error("AI gateway request failed: {0}")]
    Transport(#[from] reqwest::Error),
}

#[async_trait]
pub trait AiAdapter: Send + Sync {
    async fn execute(&self, work: AiWorkItem) -> Result<AiWorkResult, AiAdapterError>;
    fn provider_name(&self) -> &'static str;
}

/// A small HTTP contract intended for an organisation-owned AI gateway. It is
/// deliberately model-agnostic so switching providers does not change routes,
/// database jobs, or frontend payloads.
pub struct HttpAiGateway {
    client: Client,
    endpoint: String,
    api_key: Option<String>,
}

impl HttpAiGateway {
    pub fn new(endpoint: String, api_key: Option<String>) -> Self {
        Self { client: Client::new(), endpoint: endpoint.trim_end_matches('/').to_owned(), api_key }
    }
}

#[async_trait]
impl AiAdapter for HttpAiGateway {
    async fn execute(&self, work: AiWorkItem) -> Result<AiWorkResult, AiAdapterError> {
        let mut request = self.client.post(format!("{}/v1/jobs", self.endpoint)).json(&work);
        if let Some(api_key) = &self.api_key {
            request = request.bearer_auth(api_key);
        }
        let response = request.send().await?;
        if response.status() != StatusCode::OK && response.status() != StatusCode::ACCEPTED {
            return Err(AiAdapterError::Rejected(response.text().await.unwrap_or_default()));
        }
        Ok(response.json::<AiWorkResult>().await?)
    }

    fn provider_name(&self) -> &'static str { "http-ai-gateway" }
}

pub struct UnconfiguredAiAdapter;

#[async_trait]
impl AiAdapter for UnconfiguredAiAdapter {
    async fn execute(&self, _: AiWorkItem) -> Result<AiWorkResult, AiAdapterError> {
        Err(AiAdapterError::Unavailable)
    }

    fn provider_name(&self) -> &'static str { "unconfigured" }
}

pub fn adapter_from_config(config: &AppConfig) -> SharedAiAdapter {
    config.ai_gateway_url.as_ref().map(|endpoint| {
        Arc::new(HttpAiGateway::new(endpoint.clone(), config.ai_gateway_api_key.clone())) as SharedAiAdapter
    }).unwrap_or_else(|| Arc::new(UnconfiguredAiAdapter))
}
