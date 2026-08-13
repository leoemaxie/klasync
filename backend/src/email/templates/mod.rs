/// Transactional email templates for KLASYNC.
///
/// Each template is a plain Rust struct that implements `EmailTemplate`,
/// producing subject, plaintext body, and branded HTML content without
/// any external template engine dependency.
pub mod claim_confirm;
pub mod claim_verify;
pub mod password_reset;

use super::EmailMessage;

/// A typed transactional email template.
pub trait EmailTemplate: Send + Sync {
    /// Email subject line.
    fn subject(&self) -> String;
    /// Plain-text fallback body.
    fn text_body(&self) -> String;
    /// Inner HTML content (inserted into the branded layout wrapper).
    fn html_content(&self) -> String;
}

impl EmailMessage {
    /// Construct an `EmailMessage` from a typed template, wrapping the HTML
    /// content in the shared KLASYNC branded layout.
    pub fn from_template(
        to: impl Into<String>,
        idempotency_key: impl Into<String>,
        template: &impl EmailTemplate,
        app_url: &str,
    ) -> Self {
        Self {
            to: to.into(),
            subject: template.subject(),
            text: template.text_body(),
            html: super::layout::render(&template.subject(), &template.html_content(), app_url),
            idempotency_key: idempotency_key.into(),
        }
    }
}
