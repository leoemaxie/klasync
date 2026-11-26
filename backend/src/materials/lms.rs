//! LMS push: a generic signed webhook.
//!
//! Learning-management systems differ too much to integrate one by one, so the
//! `webhook` delivery channel pushes a small JSON document to a URL the lecturer
//! configures per course (the plug-in setting's `config`: `{"url", "secret"}`).
//! An LMS connector, Zapier/Make flow or a custom script turns that into a Canvas
//! page, a Moodle resource, and so on. The body is signed with HMAC-SHA256 so the
//! receiver can verify it came from Klasync, and it carries 24-hour signed
//! download links for the PDF/DOCX.
//!
//! Safety: only `https` URLs (set `MATERIALS_WEBHOOK_ALLOW_HTTP=1` for local
//! development) and never private, loopback or link-local addresses (override
//! with `MATERIALS_WEBHOOK_ALLOW_PRIVATE=1` for local development), so a course
//! setting cannot be used to probe the server's internal network.

use std::{net::IpAddr, time::Duration};

use async_trait::async_trait;
use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::Sha256;

use super::{
    delivery::material_link,
    registry::{
        registry, DeliveryChannel, DeliveryOutcome, DeliveryRequest, PluginError, PluginKind,
        Target,
    },
    service,
};
use crate::state::AppState;

pub struct WebhookChannel;

const LINK_TTL: Duration = Duration::from_secs(24 * 3600);
const TIMEOUT: Duration = Duration::from_secs(15);

fn env_flag(name: &str) -> bool {
    matches!(std::env::var(name).as_deref(), Ok("1") | Ok("true"))
}

/// Is this address one a webhook must never reach?
pub fn is_internal(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.octets()[0] == 0
                // Carrier-grade NAT 100.64.0.0/10
                || (v4.octets()[0] == 100 && (64..128).contains(&v4.octets()[1]))
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || (v6.segments()[0] & 0xfe00) == 0xfc00 // unique local
                || (v6.segments()[0] & 0xffc0) == 0xfe80 // link local
                || v6.to_ipv4_mapped().map(|v4| is_internal(IpAddr::V4(v4))).unwrap_or(false)
        }
    }
}

/// Checks the shape of a webhook URL (scheme, host, no credentials).
pub fn validate_url_shape(raw: &str) -> Result<reqwest::Url, String> {
    let url = reqwest::Url::parse(raw.trim()).map_err(|_| "the webhook URL is not valid")?;
    let allow_http = env_flag("MATERIALS_WEBHOOK_ALLOW_HTTP");
    match url.scheme() {
        "https" => {}
        "http" if allow_http => {}
        _ => return Err("the webhook URL must start with https://".to_owned()),
    }
    if url.host_str().is_none() {
        return Err("the webhook URL has no host".to_owned());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("the webhook URL must not contain credentials".to_owned());
    }
    Ok(url)
}

/// Shape check plus DNS resolution: every address must be public.
pub async fn validate_destination(raw: &str) -> Result<reqwest::Url, String> {
    let url = validate_url_shape(raw)?;
    if env_flag("MATERIALS_WEBHOOK_ALLOW_PRIVATE") {
        return Ok(url);
    }
    let host = url.host_str().unwrap_or_default().to_owned();
    let port = url.port_or_known_default().unwrap_or(443);
    let addrs: Vec<_> = tokio::net::lookup_host((host.as_str(), port))
        .await
        .map_err(|_| "the webhook host could not be resolved".to_owned())?
        .collect();
    if addrs.is_empty() || addrs.iter().any(|a| is_internal(a.ip())) {
        return Err("the webhook host is not a public address".to_owned());
    }
    Ok(url)
}

/// `sha256=<hex>` HMAC of the body.
pub fn sign(secret: &str, body: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("any key length");
    mac.update(body);
    let bytes = mac.finalize().into_bytes();
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!("sha256={hex}")
}

#[async_trait]
impl DeliveryChannel for WebhookChannel {
    fn id(&self) -> &'static str {
        "webhook"
    }
    fn label(&self) -> &'static str {
        "LMS / webhook"
    }
    fn deferred(&self) -> bool {
        true
    }
    async fn deliver(
        &self,
        state: &AppState,
        request: &DeliveryRequest,
    ) -> Result<DeliveryOutcome, PluginError> {
        let (enabled, config) = super::registry::resolve_setting(
            state.db_pool(),
            PluginKind::Delivery,
            "webhook",
            Some(request.course_id),
        )
        .await
        .unwrap_or((true, json!({})));
        if !enabled {
            return Err(PluginError::failed(
                "the LMS webhook is turned off for this course",
            ));
        }
        let url_text = config
            .get("url")
            .and_then(Value::as_str)
            .filter(|u| !u.trim().is_empty())
            .ok_or_else(|| PluginError::failed("no LMS webhook URL is set for this course"))?;
        let url = validate_destination(url_text)
            .await
            .map_err(PluginError::failed)?;
        let secret = config.get("secret").and_then(Value::as_str).unwrap_or("");

        // 24-hour links to the handout files, when storage can sign.
        let mut files = serde_json::Map::new();
        for format in ["pdf", "docx", "epub"] {
            if !registry().renderer(format).is_some_and(|r| r.available()) {
                continue;
            }
            if let Ok(file) = service::export(state, request.target, format, None).await {
                if let Some(link) = service::export_url_for(state, &file, LINK_TTL).await {
                    files.insert(
                        format.to_owned(),
                        json!({"url": link, "filename": file.filename}),
                    );
                }
            }
        }
        let (kind, id) = match request.target {
            Target::Version(id) => ("lecture", id),
            Target::Book(id) => ("book", id),
        };
        let payload = json!({
            "event": "material.published",
            "kind": kind,
            "id": id,
            "title": request.title,
            "message": request.message,
            "course_id": request.course_id,
            "app_url": material_link(&state.config.public_app_url, request.target),
            "files": files,
            "link_expires_in": LINK_TTL.as_secs(),
        });
        let body = serde_json::to_vec(&payload).map_err(|e| PluginError::failed(e.to_string()))?;

        let client = reqwest::Client::builder()
            .timeout(TIMEOUT)
            // A redirect could point at an internal address we never checked.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| PluginError::failed(e.to_string()))?;
        let mut call = client
            .post(url)
            .header("content-type", "application/json")
            .header("x-klasync-event", "material.published")
            .header("x-klasync-delivery", request.delivery_id.to_string());
        if !secret.is_empty() {
            call = call.header("x-klasync-signature", sign(secret, &body));
        }
        let response =
            call.body(body).send().await.map_err(|e| {
                PluginError::failed(format!("the webhook could not be reached: {e}"))
            })?;
        if !response.status().is_success() {
            return Err(PluginError::failed(format!(
                "the webhook answered {}",
                response.status()
            )));
        }
        Ok(DeliveryOutcome {
            recipients: 1,
            delivered: 1,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_internal_addresses() {
        for ip in [
            "127.0.0.1",
            "10.1.2.3",
            "192.168.0.9",
            "169.254.169.254",
            "172.16.5.5",
            "100.64.0.1",
            "0.0.0.0",
            "::1",
            "fd00::1",
            "fe80::1",
            "::ffff:10.0.0.1",
        ] {
            assert!(is_internal(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["8.8.8.8", "93.184.216.34", "2606:4700::1111"] {
            assert!(!is_internal(ip.parse().unwrap()), "{ip}");
        }
    }

    #[test]
    fn url_shape_rules() {
        assert!(validate_url_shape("https://lms.example.edu/hooks/abc").is_ok());
        assert!(validate_url_shape("http://lms.example.edu/x").is_err());
        assert!(validate_url_shape("ftp://x.example.edu").is_err());
        assert!(validate_url_shape("https://user:pw@lms.example.edu/").is_err());
        assert!(validate_url_shape("not a url").is_err());
    }

    #[test]
    fn signature_matches_a_known_vector() {
        // RFC 4231 test case 2.
        assert_eq!(
            sign("Jefe", b"what do ya want for nothing?"),
            "sha256=5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[tokio::test]
    async fn loopback_destinations_are_refused() {
        let error = validate_destination("https://127.0.0.1/hook")
            .await
            .unwrap_err();
        assert!(error.contains("public"), "{error}");
    }
}
