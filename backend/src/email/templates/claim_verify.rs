use super::EmailTemplate;
use crate::email::escape::escape_html;

pub struct ClaimVerifyTemplate {
    pub code: String,
    pub expires_minutes: u32,
}

impl EmailTemplate for ClaimVerifyTemplate {
    fn subject(&self) -> String {
        "Verify your KLASYNC lecture claim".to_owned()
    }

    fn text_body(&self) -> String {
        format!(
            "Your KLASYNC verification code is {}.\nIt expires in {} minutes.",
            self.code, self.expires_minutes
        )
    }

    fn html_content(&self) -> String {
        let safe_code = escape_html(&self.code);
        format!(
            r#"<h2>Verify Your Lecture Claim</h2>
<p>Use the verification code below to confirm your lecture participation:</p>
<div class="code-box">{safe_code}</div>
<p class="muted">This code expires in {expires} minutes. If you did not request this, you can safely ignore this email.</p>"#,
            safe_code = safe_code,
            expires = self.expires_minutes
        )
    }
}
