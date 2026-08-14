use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::{error::ApiError, handlers::sessions::database_session_by_code},
    audit::{self, AuditEvent},
    auth::guard::AuthenticatedLecturer,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct LiveControlPatch {
    pub captions_paused: Option<bool>,
    pub audio_ingestion_active: Option<bool>,
    pub late_join_policy: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct LiveControlState {
    pub session_id: Uuid,
    pub captions_paused: bool,
    pub audio_ingestion_active: bool,
    pub late_join_policy: String,
}

pub async fn update(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(short_code): Path<String>,
    Json(input): Json<LiveControlPatch>,
) -> Result<Json<LiveControlState>, ApiError> {
    if let Some(policy) = &input.late_join_policy {
        if !["allowed", "roster_only", "closed"].contains(&policy.as_str()) {
            return Err(ApiError::bad_request("Invalid late join policy specified"));
        }
    }
    let pool = state.db_pool();
    let session = database_session_by_code(pool, &short_code).await?;
    ensure_owner(pool, session.id, lecturer.id).await?;
    let current = sqlx::query_as!(
        LiveControlState,
        r#"insert into session_live_controls (session_id, captions_paused, audio_ingestion_active, late_join_policy, updated_by)
         values ($1, coalesce($2, false), coalesce($3, false), coalesce($4, 'allowed'), $5)
         on conflict (session_id) do update set
           captions_paused = coalesce($2, session_live_controls.captions_paused),
           audio_ingestion_active = coalesce($3, session_live_controls.audio_ingestion_active),
           late_join_policy = coalesce($4, session_live_controls.late_join_policy),
           updated_by = $5, updated_at = now()
         returning session_id, captions_paused as "captions_paused!", audio_ingestion_active as "audio_ingestion_active!", late_join_policy as "late_join_policy!""#,
        session.id,
        input.captions_paused,
        input.audio_ingestion_active,
        input.late_join_policy,
        lecturer.id
    )
    .fetch_one(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to update session live controls");
        ApiError::service_unavailable()
    })?;
    audit::record_session_event(
        pool,
        session.id,
        Some(lecturer.id),
        Some("lecturer"),
        AuditEvent {
            event_type: "live_controls_updated",
            metadata: serde_json::to_value(&current).unwrap_or_default(),
        },
    )
    .await;
    Ok(Json(current))
}

pub async fn pause_captions(
    state: State<AppState>,
    lecturer: AuthenticatedLecturer,
    path: Path<String>,
) -> Result<Json<LiveControlState>, ApiError> {
    update(
        state,
        lecturer,
        path,
        Json(LiveControlPatch {
            captions_paused: Some(true),
            audio_ingestion_active: None,
            late_join_policy: None,
        }),
    )
    .await
}

pub async fn resume_captions(
    state: State<AppState>,
    lecturer: AuthenticatedLecturer,
    path: Path<String>,
) -> Result<Json<LiveControlState>, ApiError> {
    update(
        state,
        lecturer,
        path,
        Json(LiveControlPatch {
            captions_paused: Some(false),
            audio_ingestion_active: None,
            late_join_policy: None,
        }),
    )
    .await
}

pub async fn start_audio(
    state: State<AppState>,
    lecturer: AuthenticatedLecturer,
    path: Path<String>,
) -> Result<Json<LiveControlState>, ApiError> {
    update(
        state,
        lecturer,
        path,
        Json(LiveControlPatch {
            captions_paused: None,
            audio_ingestion_active: Some(true),
            late_join_policy: None,
        }),
    )
    .await
}

pub async fn stop_audio(
    state: State<AppState>,
    lecturer: AuthenticatedLecturer,
    path: Path<String>,
) -> Result<Json<LiveControlState>, ApiError> {
    update(
        state,
        lecturer,
        path,
        Json(LiveControlPatch {
            captions_paused: None,
            audio_ingestion_active: Some(false),
            late_join_policy: None,
        }),
    )
    .await
}

pub async fn participant_action(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path((short_code, participant_id, action)): Path<(String, Uuid, String)>,
) -> Result<StatusCode, ApiError> {
    if action != "mute" && action != "remove" {
        return Err(ApiError::bad_request(
            "Invalid participant moderation action",
        ));
    }
    let pool = state.db_pool();
    let session = database_session_by_code(pool, &short_code).await?;
    ensure_owner(pool, session.id, lecturer.id).await?;
    let rows_affected = if action == "mute" {
        sqlx::query!(
            "update session_participants set muted_at = coalesce(muted_at, now()) where id = $1 and session_id = $2",
            participant_id,
            session.id
        )
        .execute(pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, %action, "Failed to perform participant action");
            ApiError::service_unavailable()
        })?
        .rows_affected()
    } else {
        sqlx::query!(
            "update session_participants set removed_at = coalesce(removed_at, now()), removal_reason = 'lecturer_action' where id = $1 and session_id = $2",
            participant_id,
            session.id
        )
        .execute(pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, %action, "Failed to perform participant action");
            ApiError::service_unavailable()
        })?
        .rows_affected()
    };
    if rows_affected == 0 {
        return Err(ApiError::not_found("Participant not found"));
    }
    audit::record_session_event(
        pool,
        session.id,
        Some(lecturer.id),
        Some("lecturer"),
        AuditEvent {
            event_type: "participant_action",
            metadata: serde_json::json!({"participant_id": participant_id, "action": action}),
        },
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn moderate_caption(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path((short_code, caption_id)): Path<(String, Uuid)>,
    Json(input): Json<CaptionModerationInput>,
) -> Result<StatusCode, ApiError> {
    if input
        .text
        .as_deref()
        .map(str::trim)
        .is_some_and(str::is_empty)
    {
        return Err(ApiError::bad_request("Caption text cannot be empty"));
    }
    let pool = state.db_pool();
    let session = database_session_by_code(pool, &short_code).await?;
    ensure_owner(pool, session.id, lecturer.id).await?;
    let result = sqlx::query!(
        "update caption_chunks set text = coalesce($1, text), is_hidden = coalesce($2, is_hidden), moderation_note = $3, edited_at = now(), edited_by = $4 where id = $5 and session_id = $6",
        input.text,
        input.hidden,
        input.note,
        lecturer.id,
        caption_id,
        session.id
    )
    .execute(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to moderate caption chunk");
        ApiError::service_unavailable()
    })?;
    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Caption not found"));
    }
    audit::record_session_event(
        pool,
        session.id,
        Some(lecturer.id),
        Some("lecturer"),
        AuditEvent {
            event_type: "caption_moderated",
            metadata: serde_json::json!({"caption_id": caption_id, "hidden": input.hidden}),
        },
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct CaptionModerationInput {
    pub text: Option<String>,
    pub hidden: Option<bool>,
    pub note: Option<String>,
}

async fn ensure_owner(
    pool: &sqlx::PgPool,
    session_id: Uuid,
    lecturer_id: Uuid,
) -> Result<(), ApiError> {
    let owns = sqlx::query_scalar!(
        r#"select exists(select 1 from lecture_sessions where id = $1 and lecturer_id = $2) as "exists!""#,
        session_id,
        lecturer_id
    )
    .fetch_one(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to check session ownership in live controls");
        ApiError::service_unavailable()
    })?;
    if !owns {
        return Err(ApiError::not_found("Session not found"));
    }
    Ok(())
}
