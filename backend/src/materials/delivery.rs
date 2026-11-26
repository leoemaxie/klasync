//! Delivery channels: how a published material reaches people.
//!
//! * **in_app** and **link** are passive: publishing already makes the
//!   material visible in the student archive and downloadable, so these only
//!   leave a record in `material_deliveries`.
//! * **email** sends a link to every student enrolled in the course offering
//!   (the sender has no attachment support, so links are the only mode).
//!   Every send is logged per delivery with recipient and success counts.
//!
//! Deliveries are requested through [`request`], which writes the row and an
//! outbox event; the event relay runs [`process`] for deferred channels.

use std::time::Duration;

use async_trait::async_trait;
use serde_json::{json, Value};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use super::{
    registry::{
        registry, Audience, DeliveryChannel, DeliveryOutcome, DeliveryRequest, PluginError, Target,
    },
    store,
};
use crate::{
    email::{escape::escape_html, layout, EmailMessage},
    state::AppState,
};

pub struct InAppChannel;
pub struct LinkChannel;
pub struct EmailChannel;

#[async_trait]
impl DeliveryChannel for InAppChannel {
    fn id(&self) -> &'static str {
        "in_app"
    }
    fn label(&self) -> &'static str {
        "In-app archive"
    }
    async fn deliver(
        &self,
        _: &AppState,
        _: &DeliveryRequest,
    ) -> Result<DeliveryOutcome, PluginError> {
        Ok(DeliveryOutcome::default())
    }
}

#[async_trait]
impl DeliveryChannel for LinkChannel {
    fn id(&self) -> &'static str {
        "link"
    }
    fn label(&self) -> &'static str {
        "Download link"
    }
    async fn deliver(
        &self,
        _: &AppState,
        _: &DeliveryRequest,
    ) -> Result<DeliveryOutcome, PluginError> {
        Ok(DeliveryOutcome::default())
    }
}

#[derive(Debug, FromRow)]
struct Recipient {
    id: Uuid,
    email: String,
    display_name: String,
}

async fn recipients(
    pool: &PgPool,
    request: &DeliveryRequest,
) -> Result<Vec<Recipient>, PluginError> {
    let rows = match &request.audience {
        Audience::EnrolledRoster => {
            sqlx::query_as::<_, Recipient>(
                "select sa.id, sa.email, sa.display_name
                 from student_course_enrollments e
                 join student_accounts sa on sa.id = e.student_account_id
                 where e.course_id = $1 and sa.status = 'active'
                 order by sa.display_name",
            )
            .bind(request.course_id)
            .fetch_all(pool)
            .await
        }
        Audience::Students(ids) => {
            // Only students actually enrolled in this course, whatever was asked.
            sqlx::query_as::<_, Recipient>(
                "select sa.id, sa.email, sa.display_name
                 from student_course_enrollments e
                 join student_accounts sa on sa.id = e.student_account_id
                 where e.course_id = $1 and sa.status = 'active' and sa.id = any($2)
                 order by sa.display_name",
            )
            .bind(request.course_id)
            .bind(ids)
            .fetch_all(pool)
            .await
        }
    };
    rows.map_err(|e| PluginError::failed(e.to_string()))
}

/// Where a student opens the material in the app.
pub fn material_link(app_url: &str, target: Target) -> String {
    let base = app_url.trim_end_matches('/');
    match target {
        Target::Version(id) => format!("{base}/#/archive?material={id}"),
        Target::Book(id) => format!("{base}/#/archive?book={id}"),
    }
}

pub fn build_email(
    request: &DeliveryRequest,
    recipient_name: &str,
    link: &str,
    app_url: &str,
) -> (String, String, String) {
    let subject = format!("New lecture material: {}", request.title);
    let note = request
        .message
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty());
    let mut text = format!(
        "Hello {recipient_name},\n\nYour lecturer has published \"{}\".\n",
        request.title
    );
    let mut body = format!(
        "<h2>New lecture material</h2><p>Hello {},</p><p>Your lecturer has published <strong>{}</strong>.</p>",
        escape_html(recipient_name),
        escape_html(&request.title)
    );
    if let Some(note) = note {
        text.push_str(&format!("\nMessage from your lecturer:\n{note}\n"));
        body.push_str(&format!("<p><em>{}</em></p>", escape_html(note)));
    }
    text.push_str(&format!("\nRead or download it: {link}\n"));
    body.push_str(&format!(
        "<p><a class=\"btn\" href=\"{}\">Open the material</a></p><p class=\"muted\">You can read it in the app or export it as a PDF.</p>",
        escape_html(link)
    ));
    let html = layout::render(&subject, &body, app_url);
    (subject, text, html)
}

#[async_trait]
impl DeliveryChannel for EmailChannel {
    fn id(&self) -> &'static str {
        "email"
    }
    fn label(&self) -> &'static str {
        "Email"
    }
    fn deferred(&self) -> bool {
        true
    }
    async fn deliver(
        &self,
        state: &AppState,
        request: &DeliveryRequest,
    ) -> Result<DeliveryOutcome, PluginError> {
        let pool = state.db_pool();
        let people = recipients(pool, request).await?;
        let link = material_link(&state.config.public_app_url, request.target);
        let mut delivered = 0;
        let mut last_error: Option<String> = None;
        for person in &people {
            let (subject, text, html) = build_email(
                request,
                &person.display_name,
                &link,
                &state.config.public_app_url,
            );
            let message = EmailMessage {
                to: person.email.clone(),
                subject,
                text,
                html,
                idempotency_key: format!("material-{}-{}", request.delivery_id, person.id),
            };
            match state.mailer.send(message).await {
                Ok(()) => delivered += 1,
                Err(error) => {
                    tracing::warn!(%error, delivery = %request.delivery_id, "Material email failed");
                    last_error = Some(error.to_string());
                }
            }
            // Stay under typical provider rate limits.
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        if delivered == 0 && !people.is_empty() {
            return Err(PluginError::Failed(
                last_error.unwrap_or_else(|| "no email could be sent".to_owned()),
            ));
        }
        Ok(DeliveryOutcome {
            recipients: people.len(),
            delivered,
        })
    }
}

// ---- requests and processing -------------------------------------------------------------

/// Records a delivery and queues it. Passive channels are marked sent at once.
#[allow(clippy::too_many_arguments)]
pub async fn request(
    pool: &PgPool,
    channel_id: &str,
    target: Target,
    course_id: Uuid,
    title: &str,
    audience: &Audience,
    message: Option<&str>,
    created_by: Uuid,
) -> Result<Uuid, String> {
    let channel = registry()
        .delivery_channel(channel_id)
        .ok_or_else(|| format!("unknown delivery channel {channel_id}"))?;
    let id = Uuid::now_v7();
    let (version_id, book_version_id) = match target {
        Target::Version(v) => (Some(v), None),
        Target::Book(b) => (None, Some(b)),
    };
    let audience_json = match audience {
        Audience::EnrolledRoster => {
            json!({"kind": "enrolled_roster", "course_id": course_id, "title": title, "message": message})
        }
        Audience::Students(ids) => {
            json!({"kind": "students", "course_id": course_id, "students": ids, "title": title, "message": message})
        }
    };
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    sqlx::query(
        "insert into material_deliveries (id, version_id, book_version_id, channel, audience, status, created_by, sent_at)
         values ($1, $2, $3, $4, $5, $6, $7, case when $8 then now() end)",
    )
    .bind(id)
    .bind(version_id)
    .bind(book_version_id)
    .bind(channel_id)
    .bind(&audience_json)
    .bind(if channel.deferred() { "queued" } else { "sent" })
    .bind(created_by)
    .bind(!channel.deferred())
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    if channel.deferred() {
        store::emit(
            &mut *tx,
            "material.delivery_requested",
            json!({"delivery_id": id, "channel": channel_id}),
        )
        .await
        .map_err(|e| e.to_string())?;
    }
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(id)
}

#[derive(Debug, FromRow)]
struct DeliveryRow {
    version_id: Option<Uuid>,
    book_version_id: Option<Uuid>,
    channel: String,
    audience: Value,
}

fn audience_from_json(value: &Value) -> Audience {
    match value.get("kind").and_then(Value::as_str) {
        Some("students") => Audience::Students(
            value
                .get("students")
                .and_then(Value::as_array)
                .map(|ids| {
                    ids.iter()
                        .filter_map(|v| v.as_str().and_then(|s| Uuid::parse_str(s).ok()))
                        .collect()
                })
                .unwrap_or_default(),
        ),
        _ => Audience::EnrolledRoster,
    }
}

/// Runs one queued delivery through its channel and records the outcome.
/// Claiming is atomic, so concurrent relays never send twice.
pub async fn process(state: &AppState, delivery_id: Uuid) -> Result<(), String> {
    let pool = state.db_pool();
    let row = sqlx::query_as::<_, DeliveryRow>(
        "update material_deliveries set status = 'sending'
         where id = $1 and status = 'queued'
         returning version_id, book_version_id, channel, audience",
    )
    .bind(delivery_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;
    let Some(row) = row else {
        return Ok(());
    };
    let target = match (row.version_id, row.book_version_id) {
        (Some(v), _) => Target::Version(v),
        (None, Some(b)) => Target::Book(b),
        _ => return Err("a delivery needs a target".to_owned()),
    };
    let course_id = row
        .audience
        .get("course_id")
        .and_then(Value::as_str)
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| "the delivery has no course".to_owned())?;
    let request = DeliveryRequest {
        delivery_id,
        target,
        course_id,
        title: row
            .audience
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("Lecture material")
            .to_owned(),
        audience: audience_from_json(&row.audience),
        message: row
            .audience
            .get("message")
            .and_then(Value::as_str)
            .map(str::to_owned),
    };
    let outcome = match registry().delivery_channel(&row.channel) {
        Some(channel) => channel.deliver(state, &request).await,
        None => Err(PluginError::failed("the channel is no longer installed")),
    };
    match outcome {
        Ok(outcome) => {
            let status = if outcome.recipients == 0 || outcome.delivered == outcome.recipients {
                "sent"
            } else {
                "partial"
            };
            sqlx::query(
                "update material_deliveries set status = $2, recipients = $3, delivered = $4, sent_at = now() where id = $1",
            )
            .bind(delivery_id)
            .bind(status)
            .bind(outcome.recipients as i32)
            .bind(outcome.delivered as i32)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        Err(error) => {
            sqlx::query(
                "update material_deliveries set status = 'failed', error = $2 where id = $1",
            )
            .bind(delivery_id)
            .bind(error.to_string().chars().take(300).collect::<String>())
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(message: Option<&str>) -> DeliveryRequest {
        DeliveryRequest {
            delivery_id: Uuid::nil(),
            target: Target::Version(Uuid::nil()),
            course_id: Uuid::nil(),
            title: "Heat <transfer> & more".into(),
            audience: Audience::EnrolledRoster,
            message: message.map(str::to_owned),
        }
    }

    #[test]
    fn email_escapes_and_includes_lecturer_note() {
        let (subject, text, html) = build_email(
            &request(Some("Read <b>this</b> first")),
            "Ada <script>",
            "https://app.example/#/archive?material=1",
            "https://app.example",
        );
        assert_eq!(subject, "New lecture material: Heat <transfer> & more");
        assert!(text.contains("Read <b>this</b> first"));
        assert!(text.contains("https://app.example/#/archive?material=1"));
        assert!(html.contains("Ada &lt;script&gt;"));
        assert!(html.contains("Heat &lt;transfer&gt; &amp; more"));
        assert!(html.contains("Read &lt;b&gt;this&lt;/b&gt; first"));
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn links_target_versions_and_books() {
        let id = Uuid::nil();
        assert_eq!(
            material_link("https://a.b/", Target::Version(id)),
            format!("https://a.b/#/archive?material={id}")
        );
        assert!(material_link("https://a.b", Target::Book(id)).contains("?book="));
    }

    #[test]
    fn audience_round_trips() {
        let id = Uuid::now_v7();
        let value = json!({"kind": "students", "students": [id.to_string(), "bad"]});
        match audience_from_json(&value) {
            Audience::Students(ids) => assert_eq!(ids, vec![id]),
            _ => panic!("expected students"),
        }
        assert!(matches!(
            audience_from_json(&json!({"kind": "enrolled_roster"})),
            Audience::EnrolledRoster
        ));
    }
}
