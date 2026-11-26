//! Provider-neutral AI layer.
//!
//! * `types`      — tiers, tasks, requests, usage, errors (no vendor knowledge)
//! * `provider`   — the capability traits a vendor can implement
//! * `gemini`     — Google Gemini (native API): text, transcription, embeddings, batch
//! * `openrouter` — OpenRouter: text, transcription, embeddings
//! * `gateway`    — routing, retries, escalation, pricing and usage logging
//!
//! Everything else in the application talks only to [`AiGateway`].
//! Selection happens once, in [`gateway_from_config`]: a Google API key routes
//! to Gemini; otherwise OpenRouter is used (and stays available as a fallback).

pub mod gateway;
pub mod gemini;
pub mod openrouter;
pub mod pricing;
pub mod provider;
pub mod types;

use std::sync::Arc;

use sqlx::PgPool;

use crate::config::AppConfig;

pub use gateway::{AiGateway, GatewayParts};
pub use pricing::PriceTable;
pub use provider::{BatchProvider, Embedder, TextProvider, Transcriber};
pub use types::*;

pub type SharedAi = Arc<AiGateway>;

/// A reqwest client builder that also trusts `HTTP_EXTRA_CA_FILE`, so the
/// service works behind TLS-inspecting proxies. Bad bundles are logged, not fatal.
pub fn client_builder(config: &AppConfig) -> reqwest::ClientBuilder {
    let mut builder = reqwest::Client::builder();
    if let Some(path) = &config.http_extra_ca_file {
        match std::fs::read(path)
            .map_err(|error| error.to_string())
            .and_then(|pem| {
                reqwest::Certificate::from_pem_bundle(&pem).map_err(|error| error.to_string())
            }) {
            Ok(certificates) => {
                for certificate in certificates {
                    builder = builder.add_root_certificate(certificate);
                }
            }
            Err(error) => tracing::warn!(%path, %error, "Could not load HTTP_EXTRA_CA_FILE"),
        }
    }
    builder
}

pub fn gateway_from_config(config: &AppConfig, pool: Option<PgPool>) -> SharedAi {
    let wants_gemini = matches!(config.ai_provider.as_str(), "auto" | "gemini");
    let wants_openrouter = matches!(
        config.ai_provider.as_str(),
        "auto" | "openrouter" | "gemini"
    );

    let gemini = if wants_gemini {
        gemini::GeminiProvider::from_config(config).map(Arc::new)
    } else {
        None
    };
    if config.ai_provider == "gemini" && gemini.is_none() {
        tracing::warn!("AI_PROVIDER=gemini but no GEMINI_API_KEY/GOOGLE_API_KEY is set");
    }
    // OpenRouter is the primary only when there is no Gemini; with both it is
    // appended as a last-resort fallback for text.
    let openrouter = if wants_openrouter {
        openrouter::OpenRouterProvider::from_config(config).map(Arc::new)
    } else {
        None
    };

    let mut text: Vec<Arc<dyn TextProvider>> = Vec::new();
    if let Some(gemini) = &gemini {
        text.push(gemini.clone());
    }
    if let Some(openrouter) = &openrouter {
        text.push(openrouter.clone());
    }
    let transcriber: Option<Arc<dyn Transcriber>> = match (&gemini, &openrouter) {
        (Some(gemini), _) => Some(gemini.clone()),
        (None, Some(openrouter)) => Some(openrouter.clone()),
        _ => None,
    };
    let embedder: Option<Arc<dyn Embedder>> = match (&gemini, &openrouter) {
        (Some(gemini), _) => Some(gemini.clone()),
        (None, Some(openrouter)) => Some(openrouter.clone()),
        _ => None,
    };
    let batch: Option<Arc<dyn BatchProvider>> = match (&gemini, config.ai_batch_enabled) {
        (Some(gemini), true) => Some(gemini.clone()),
        _ => None,
    };

    tracing::info!(
        text = ?text.iter().map(|p| p.name()).collect::<Vec<_>>(),
        embeddings = embedder.is_some(),
        transcription = transcriber.is_some(),
        batch = batch.is_some(),
        "AI gateway configured"
    );
    Arc::new(AiGateway::new(GatewayParts {
        text,
        transcriber,
        embedder,
        batch,
        prices: PriceTable::defaults().with_overrides(config.ai_pricing_overrides.as_deref()),
        pool,
        daily_budget_usd: config.ai_daily_budget_usd,
    }))
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
}
