use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct AuditEvent<'a> {
    pub event_type: &'a str,
    pub metadata: serde_json::Value,
}

pub async fn record_session_event(
    pool: &PgPool,
    session_id: Uuid,
    actor_id: Option<Uuid>,
    actor_role: Option<&str>,
    event: AuditEvent<'_>,
) {
    if let Err(error) = sqlx::query(
        "insert into session_audit_events (session_id, actor_id, actor_role, event_type, metadata) values ($1, $2, $3::account_role, $4, $5)",
    )
    .bind(session_id)
    .bind(actor_id)
    .bind(actor_role)
    .bind(event.event_type)
    .bind(event.metadata)
    .execute(pool)
    .await
    {
        tracing::error!(
            %error,
            %session_id,
            event_type = %event.event_type,
            "Failed to record session audit event"
        );
    }

}
