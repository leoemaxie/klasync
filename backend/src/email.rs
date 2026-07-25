use std::{path::PathBuf, sync::Arc};

use async_trait::async_trait;
use chrono::Utc;
use reqwest::{header, Client, StatusCode};
use serde::Serialize;
use thiserror::Error;
use uuid::Uuid;

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
    #[error("Resend rejected the message: {0}")]
    Rejected(String),
    #[error("email request failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("development outbox failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("development outbox serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[async_trait]
pub trait EmailSender: Send + Sync {
    async fn send(&self, message: EmailMessage) -> Result<(), EmailError>;
    fn provider_name(&self) -> &'static str;
}

pub struct ResendEmailSender {
    client: Client,
    api_key: String,
    from: String,
}

impl ResendEmailSender {
    pub fn new(api_key: String, from: String) -> Self {
        Self { client: Client::new(), api_key, from }
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
        let response = self.client
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
            return Err(EmailError::Rejected(response.text().await.unwrap_or_default()));
        }
        Ok(())
    }

    fn provider_name(&self) -> &'static str { "resend" }
}

/// Keeps local onboarding usable without accidentally sending test mail.
pub struct DevelopmentOutbox { root: PathBuf }

impl DevelopmentOutbox {
    pub fn new(root: impl Into<PathBuf>) -> Self { Self { root: root.into() } }
}

#[async_trait]
impl EmailSender for DevelopmentOutbox {
    async fn send(&self, message: EmailMessage) -> Result<(), EmailError> {
        tokio::fs::create_dir_all(&self.root).await?;
        let payload = serde_json::json!({
            "to": message.to, "subject": message.subject, "text": message.text,
            "html": message.html, "idempotency_key": message.idempotency_key,
            "created_at": Utc::now(),
        });
        let file = self.root.join(format!("{}-{}.json", Utc::now().timestamp_millis(), Uuid::now_v7()));
        tokio::fs::write(file, serde_json::to_vec_pretty(&payload)?).await?;
        Ok(())
    }

    fn provider_name(&self) -> &'static str { "development-outbox" }
}

pub struct UnconfiguredEmailSender;

#[async_trait]
impl EmailSender for UnconfiguredEmailSender {
    async fn send(&self, _: EmailMessage) -> Result<(), EmailError> { Err(EmailError::Unavailable) }
    fn provider_name(&self) -> &'static str { "unconfigured" }
}

pub fn sender_from_config(config: &AppConfig) -> SharedEmailSender {
    if let (Some(api_key), Some(from)) = (&config.resend_api_key, &config.resend_from) {
        Arc::new(ResendEmailSender::new(api_key.clone(), from.clone()))
    } else if let Some(directory) = &config.password_reset_outbox_dir {
        Arc::new(DevelopmentOutbox::new(directory))
    } else {
        Arc::new(UnconfiguredEmailSender)
    }
}

pub fn password_reset_message(config: &AppConfig, email: String, token: &str) -> EmailMessage {
    let reset_url = format!("{}/reset-password?token={token}", config.public_app_url);
    EmailMessage {
        to: email,
        subject: "Reset your KLASYNC password".to_owned(),
        text: format!("Reset your KLASYNC password: {reset_url}\n\nThis link expires shortly."),
        html: format!("<p>Reset your KLASYNC password.</p><p><a href=\"{reset_url}\">Reset password</a></p><p>This link expires shortly.</p>"),
        idempotency_key: format!("password-reset-{token}"),
    }
}
