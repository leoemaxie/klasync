use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    api::error::ApiError,
    api::handlers::sessions::database_session_by_code,
    audit::{self, AuditEvent},
    auth::guard::AuthenticatedLecturer,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct LifecyclePatch {
    pub title: Option<String>,
    pub scheduled_start_at: Option<DateTime<Utc>>,
    pub timezone: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct LifecycleView {
    pub id: Uuid,
    pub title: String,
    pub status: String,
    pub scheduled_start_at: Option<DateTime<Utc>>,
    pub timezone: String,
    pub archived_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub reopen_count: i32,
}

pub async fn update(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(short_code): Path<String>,
    Json(input): Json<LifecyclePatch>,
) -> Result<Json<LifecycleView>, ApiError> {
    if input
        .title
        .as_deref()
        .map(str::trim)
        .is_some_and(str::is_empty)
    {
        return Err(ApiError::bad_request("Session title is required"));
    }
    if input
        .timezone
        .as_deref()
        .is_some_and(|zone| zone.trim().is_empty())
    {
        return Err(ApiError::bad_request("Timezone is required"));
    }
    let pool = state
        .production_database()
        .ok_or_else(|| ApiError::service_unavailable())?;
    let session = database_session_by_code(pool, &short_code).await?;
    ensure_owner(pool, session.id, lecturer.id).await?;
    let view = sqlx::query_as::<_, LifecycleView>(
        "update lecture_sessions set title = coalesce($1, title), scheduled_start_at = coalesce($2, scheduled_start_at), timezone = coalesce($3, timezone), status = case when $2 is not null then 'scheduled' else status end where id = $4 and deleted_at is null returning id, title, status::text, scheduled_start_at, timezone, archived_at, deleted_at, reopen_count",
    )
    .bind(input.title.map(|value| value.trim().to_owned()))
    .bind(input.scheduled_start_at)
    .bind(input.timezone.map(|value| value.trim().to_owned()))
    .bind(session.id)
    .fetch_one(pool).await
    .map_err(|_| ApiError::service_unavailable())?;
    audit::record_session_event(
        pool,
        session.id,
        Some(lecturer.id),
        Some("lecturer"),
        AuditEvent {
            event_type: "session_updated",
            metadata: serde_json::to_value(&view).unwrap_or_default(),
        },
    )
    .await;
    Ok(Json(view))
}

pub async fn archive(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(short_code): Path<String>,
) -> Result<Json<LifecycleView>, ApiError> {
    transition(&state, lecturer.id, &short_code, "archive")
        .await
        .map(Json)
}

pub async fn reopen(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(short_code): Path<String>,
) -> Result<Json<LifecycleView>, ApiError> {
    transition(&state, lecturer.id, &short_code, "reopen")
        .await
        .map(Json)
}

pub async fn remove(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(short_code): Path<String>,
) -> Result<StatusCode, ApiError> {
    let pool = state
        .production_database()
        .ok_or_else(|| ApiError::service_unavailable())?;
    let session = database_session_by_code(pool, &short_code).await?;
    ensure_owner(pool, session.id, lecturer.id).await?;
    let result = sqlx::query("update lecture_sessions set deleted_at = now() where id = $1 and status = 'ended' and deleted_at is null")
        .bind(session.id).execute(pool).await
        .map_err(|_| ApiError::service_unavailable())?;
    if result.rows_affected() == 0 {
        return Err(ApiError::conflict(
            "Sessions must be ended before they can be deleted",
        ));
    }
    audit::record_session_event(
        pool,
        session.id,
        Some(lecturer.id),
        Some("lecturer"),
        AuditEvent {
            event_type: "session_deleted",
            metadata: serde_json::json!({}),
        },
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

async fn transition(
    state: &AppState,
    lecturer_id: Uuid,
    short_code: &str,
    action: &str,
) -> Result<LifecycleView, ApiError> {
    let pool = state
        .production_database()
        .ok_or_else(|| ApiError::service_unavailable())?;
    let session = database_session_by_code(pool, short_code).await?;
    ensure_owner(pool, session.id, lecturer_id).await?;
    let (query, event) = match action {
        "archive" => (
            "update lecture_sessions set archived_at = now() where id = $1 and status = 'ended' and archived_at is null and deleted_at is null returning id, title, status::text, scheduled_start_at, timezone, archived_at, deleted_at, reopen_count",
            "session_archived",
        ),
        "reopen" => (
            "update lecture_sessions set status = 'live', started_at = now(), ended_at = null, archived_at = null, reopen_count = reopen_count + 1 where id = $1 and status = 'ended' and deleted_at is null returning id, title, status::text, scheduled_start_at, timezone, archived_at, deleted_at, reopen_count",
            "session_reopened",
        ),
        _ => return Err(ApiError::bad_request("Invalid session lifecycle action")),
    };
    let view = sqlx::query_as::<_, LifecycleView>(query)
        .bind(session.id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::service_unavailable())?
        .ok_or_else(|| {
            ApiError::conflict("Session cannot be transitioned from its current status")
        })?;
    audit::record_session_event(
        pool,
        session.id,
        Some(lecturer_id),
        Some("lecturer"),
        AuditEvent {
            event_type: event,
            metadata: serde_json::to_value(&view).unwrap_or_default(),
        },
    )
    .await;
    Ok(view)
}

async fn ensure_owner(
    pool: &sqlx::PgPool,
    session_id: Uuid,
    lecturer_id: Uuid,
) -> Result<(), ApiError> {
    let owns: bool = sqlx::query_scalar(
        "select exists(select 1 from lecture_sessions where id = $1 and lecturer_id = $2)",
    )
    .bind(session_id)
    .bind(lecturer_id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?;
    if !owns {
        return Err(ApiError::not_found("Session not found"));
    }
    Ok(())
}
