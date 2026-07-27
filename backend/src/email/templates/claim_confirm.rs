use super::EmailTemplate;

pub struct ClaimConfirmTemplate;

impl EmailTemplate for ClaimConfirmTemplate {
    fn subject(&self) -> String {
        "KLASYNC lecture claim confirmed".to_owned()
    }

    fn text_body(&self) -> String {
        "Your lecture participation has been linked to your KLASYNC account.".to_owned()
    }

    fn html_content(&self) -> String {
        r#"<h2>Claim Confirmed</h2>
<p>Your lecture participation has been successfully linked to your KLASYNC account.</p>
<p>You can now access lecture materials, captions, and attendance records from your dashboard.</p>"#
            .to_owned()
    }
}
