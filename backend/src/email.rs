/// KLASYNC transactional email module.
///
/// Provides transport-agnostic email sending via the `EmailSender` trait,
/// with pluggable backends (Resend API, unconfigured stub). All outgoing emails
/// are rendered through branded templates defined in the `templates` submodule.
pub mod escape;
pub mod layout;
pub mod templates;

use std::sync::Arc;

use async_trait::async_trait;
use reqwest::{header, Client, StatusCode};
use serde::Serialize;
use thiserror::Error;

use crate::config::AppConfig;

pub type SharedEmailSender = Arc<dyn EmailSender>;

#[derive(Debug, Clone)]
pub struct EmailMessage {
    pub to: String,
    pub subject: String,
    pub text: String,
    pub html: String,
    pub idempotency_key: String,
}

#[derive(Debug, Error)]
pub enum EmailError {
    #[error("email delivery is not configured")]
    Unavailable,
    #[error("email provider rejected the message: {0}")]
    Rejected(String),
    #[error("email request failed: {0}")]
    Transport(#[from] reqwest::Error),
}

#[async_trait]
pub trait EmailSender: Send + Sync {
    async fn send(&self, message: EmailMessage) -> Result<(), EmailError>;
    fn provider_name(&self) -> &'static str;
}

// ---------------------------------------------------------------------------
// Resend API transport
// ---------------------------------------------------------------------------

pub struct ResendEmailSender {
    client: Client,
    api_key: String,
    from: String,
}

impl ResendEmailSender {
    pub fn new(api_key: String, from: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            from,
        }
    }
}

#[derive(Serialize)]
struct ResendPayload<'a> {
    from: &'a str,
    to: Vec<&'a str>,
    subject: &'a str,
    text: &'a str,
    html: &'a str,
}

#[async_trait]
impl EmailSender for ResendEmailSender {
    async fn send(&self, message: EmailMessage) -> Result<(), EmailError> {
        let response = self
            .client
            .post("https://api.resend.com/emails")
            .header(header::AUTHORIZATION, format!("Bearer {}", self.api_key))
            .header(header::USER_AGENT, "klasync-api/0.1")
            .header("Idempotency-Key", message.idempotency_key)
            .json(&ResendPayload {
                from: &self.from,
                to: vec![&message.to],
                subject: &message.subject,
                text: &message.text,
                html: &message.html,
            })
            .send()
            .await?;
        if response.status() != StatusCode::OK && response.status() != StatusCode::CREATED {
            return Err(EmailError::Rejected(
                response.text().await.unwrap_or_default(),
            ));
        }
        Ok(())
    }

    fn provider_name(&self) -> &'static str {
        "resend"
    }
}

// ---------------------------------------------------------------------------
// Unconfigured stub
// ---------------------------------------------------------------------------

pub struct UnconfiguredEmailSender;

#[async_trait]
impl EmailSender for UnconfiguredEmailSender {
    async fn send(&self, _: EmailMessage) -> Result<(), EmailError> {
        Err(EmailError::Unavailable)
    }
    fn provider_name(&self) -> &'static str {
        "unconfigured"
    }
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

pub fn sender_from_config(config: &AppConfig) -> SharedEmailSender {
    if let (Some(api_key), Some(from)) = (&config.resend_api_key, &config.resend_from) {
        Arc::new(ResendEmailSender::new(api_key.clone(), from.clone()))
    } else {
        Arc::new(UnconfiguredEmailSender)
    }
}
