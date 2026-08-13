/// Shared branded HTML email layout for all KLASYNC transactional emails.
use super::escape::escape_html;

/// Wraps inner template HTML content in the full branded document shell.
pub fn render(title: &str, body_content: &str, app_url: &str) -> String {
    let safe_title = escape_html(title);
    let safe_app_url = escape_html(app_url);
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1.0">
<title>{safe_title}</title>
<style>
body{{margin:0;padding:0;background-color:#0d0e11;font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,Helvetica,Arial,sans-serif;color:#f3f4f6;-webkit-text-size-adjust:100%}}
.wrapper{{width:100%;table-layout:fixed;background-color:#0d0e11;padding:40px 16px}}
.main{{max-width:560px;margin:0 auto;background-color:#16181d;border:1px solid #272a30;border-radius:12px;overflow:hidden}}
.header{{padding:24px 32px;border-bottom:1px solid #272a30;background-color:#121418}}
.logo{{display:inline-block;font-size:13px;font-weight:700;letter-spacing:0.1em;color:#c0e060;text-transform:uppercase;text-decoration:none}}
.body{{padding:32px;font-size:15px;line-height:1.6;color:#d1d5db}}
.body h2{{color:#ffffff;margin:0 0 16px;font-size:20px;font-weight:600}}
.body p{{margin:0 0 14px}}
.btn{{display:inline-block;background-color:#c0e060;color:#0d0e11;font-weight:600;font-size:15px;padding:12px 28px;border-radius:8px;text-decoration:none;margin:8px 0 16px}}
.code-box{{display:inline-block;background-color:#0d0e11;border:1px solid #374151;font-family:"Courier New",Courier,monospace;font-size:28px;font-weight:700;letter-spacing:0.25em;color:#c0e060;padding:14px 24px;border-radius:8px;margin:12px 0}}
.muted{{font-size:13px;color:#6b7280}}
.link-fallback{{word-break:break-all;color:#9ca3af}}
.footer{{padding:20px 32px;border-top:1px solid #272a30;background-color:#121418;font-size:12px;color:#6b7280;text-align:center}}
.footer a{{color:#9ca3af;text-decoration:none}}
@media only screen and (max-width:600px){{
  .wrapper{{padding:20px 8px}}
  .header,.body,.footer{{padding-left:20px;padding-right:20px}}
}}
</style>
</head>
<body>
<div class="wrapper">
<div class="main">
<div class="header">
<span class="logo">KLASYNC // Lecture Accessibility</span>
</div>
<div class="body">
{body_content}
</div>
<div class="footer">
<p>This is an automated message from the KLASYNC platform.</p>
<p><a href="{safe_app_url}">{safe_app_url}</a></p>
</div>
</div>
</div>
</body>
</html>"#
    )
}
