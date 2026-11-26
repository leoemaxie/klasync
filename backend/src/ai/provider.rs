//! The seams between the application and any model vendor. A provider
//! implements only the capabilities it genuinely has; the gateway composes them.

use async_trait::async_trait;

use super::types::*;

#[async_trait]
pub trait TextProvider: Send + Sync {
    fn name(&self) -> &'static str;

    /// Ordered models to try for this work, cheapest-adequate first.
    fn models_for(&self, task: Task, tier: Tier) -> Vec<String>;

    /// How much lecture text one request can comfortably carry, in characters.
    /// Longer transcripts are condensed first (map-reduce on the cheap tier).
    fn context_chars(&self) -> usize {
        28_000
    }

    /// One attempt against one model. Retries, fallbacks, budgets and cost
    /// accounting are the gateway's job.
    async fn generate(&self, model: &str, request: &TextRequest) -> Result<TextOutput, AiError>;
}

#[async_trait]
pub trait Transcriber: Send + Sync {
    fn name(&self) -> &'static str;
    fn models_for(&self, duration_sec: Option<u32>) -> Vec<String>;
    async fn transcribe(
        &self,
        model: &str,
        request: &TranscribeRequest,
    ) -> Result<TextOutput, AiError>;
}

#[async_trait]
pub trait Embedder: Send + Sync {
    fn name(&self) -> &'static str;
    fn model(&self) -> String;
    fn dimensions(&self) -> usize;
    async fn embed(&self, items: &[EmbedItem], kind: EmbedKind) -> Result<Embeddings, AiError>;
}

/// Asynchronous, discounted execution of many text requests.
#[async_trait]
pub trait BatchProvider: Send + Sync {
    fn name(&self) -> &'static str;
    /// Models a batch for this tier would run on.
    fn model_for(&self, task: Task, tier: Tier) -> String;
    async fn submit(&self, model: &str, items: &[BatchItem]) -> Result<String, AiError>;
    async fn poll(&self, batch_id: &str) -> Result<BatchState, AiError>;
}
