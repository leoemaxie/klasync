use super::EmailTemplate;
use crate::email::escape::escape_html;

pub struct PasswordResetTemplate {
    pub reset_url: String,
    pub expires_minutes: u32,
}

impl EmailTemplate for PasswordResetTemplate {
    fn subject(&self) -> String {
        "Reset your KLASYNC password".to_owned()
    }

    fn text_body(&self) -> String {
        format!(
            "Reset your KLASYNC password by visiting:\n{}\n\nThis link expires in {} minutes.\nIf you did not request this, you can safely ignore this email.",
            self.reset_url, self.expires_minutes
        )
    }

    fn html_content(&self) -> String {
        let safe_url = escape_html(&self.reset_url);
        format!(
            r#"<h2>Password Reset Request</h2>
<p>We received a request to reset the password for your KLASYNC account.</p>
<p><a href="{safe_url}" class="btn">Reset Password</a></p>
<p class="muted">Or copy and paste this URL into your browser:<br><span class="link-fallback">{safe_url}</span></p>
<p class="muted">This link expires in {expires} minutes. If you did not request a password reset, you can safely ignore this email.</p>"#,
            safe_url = safe_url,
            expires = self.expires_minutes
        )
    }
}
