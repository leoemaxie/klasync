//! Smaller live-session endpoints the UI already called but that did not exist:
//! hand raising, presence-by-code and the rolling summary feed.

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
    api::{
        error::{ApiError, LogApiError},
        handlers::{archive::RollingSummary, sessions::database_session_by_code},
    },
    auth::guard::AuthenticatedLecturer,
    state::AppState,
};

pub async fn raise_hand(
    State(state): State<AppState>,
    Path((code, participant_id)): Path<(String, Uuid)>,
) -> Result<StatusCode, ApiError> {
    set_hand(&state, &code, participant_id, true).await
}

pub async fn lower_hand(
    State(state): State<AppState>,
    Path((code, participant_id)): Path<(String, Uuid)>,
) -> Result<StatusCode, ApiError> {
    set_hand(&state, &code, participant_id, false).await
}

async fn set_hand(
    state: &AppState,
    code: &str,
    participant_id: Uuid,
    raised: bool,
) -> Result<StatusCode, ApiError> {
    let pool = state.db_pool();
    let session = database_session_by_code(pool, code).await?;
    let result = sqlx::query(
        "update session_participants
         set hand_raised_at = case when $3 then now() else null end
         where id = $1 and session_id = $2 and removed_at is null",
    )
    .bind(participant_id)
    .bind(session.id)
    .bind(raised)
    .execute(pool)
    .await
    .log_internal_error("Failed to update hand raise")?;
    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Participant not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize, FromRow)]
pub struct RaisedHand {
    pub participant_id: Uuid,
    pub display_name: String,
    pub matric_number: String,
    pub raised_at: DateTime<Utc>,
}

pub async fn raised_hands(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(code): Path<String>,
) -> Result<Json<Vec<RaisedHand>>, ApiError> {
    let pool = state.db_pool();
    let rows = sqlx::query_as::<_, RaisedHand>(
        "select p.id as participant_id, p.display_name, p.matric_number, p.hand_raised_at as raised_at
         from session_participants p join lecture_sessions s on s.id = p.session_id
         where s.short_code = upper($1) and s.lecturer_id = $2
           and p.hand_raised_at is not null and p.removed_at is null
         order by p.hand_raised_at",
    )
    .bind(code)
    .bind(lecturer.id)
    .fetch_all(pool)
    .await
    .log_internal_error("Failed to list raised hands")?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
pub struct PresenceByCode {
    pub matric_number: String,
}

/// Presence heartbeat keyed by session code + matric number (what the student
/// screen has on hand). Mirrors the participant-id based endpoint.
pub async fn presence_by_code(
    State(state): State<AppState>,
    Path(code): Path<String>,
    Json(input): Json<PresenceByCode>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = state.db_pool();
    let session = database_session_by_code(pool, &code).await?;
    let row = sqlx::query_as::<_, (Uuid, i32)>(
        "update session_participants set last_seen_at = now(), heartbeat_count = heartbeat_count + 1
         where session_id = $1 and lower(matric_number) = lower($2) and removed_at is null
         returning id, heartbeat_count",
    )
    .bind(session.id)
    .bind(input.matric_number.trim())
    .fetch_optional(pool)
    .await
    .log_internal_error("Failed to record presence by code")?
    .ok_or_else(|| ApiError::not_found("Participant not found or no longer active"))?;
    Ok(Json(serde_json::json!({
        "participant_id": row.0,
        "heartbeat_count": row.1,
        "presence_ttl_seconds": 90,
    })))
}

/// Public feed of the rolling summaries for a lecture (live or finished).
pub async fn summaries_by_code(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<Vec<RollingSummary>>, ApiError> {
    let pool = state.db_pool();
    let session = database_session_by_code(pool, &code).await?;
    let rows = sqlx::query_as::<_, RollingSummary>(
        "select id, sequence, summary, key_points, source, created_at
         from session_rolling_summaries where session_id = $1 order by sequence",
    )
    .bind(session.id)
    .fetch_all(pool)
    .await
    .log_internal_error("Failed to load rolling summaries")?;
    Ok(Json(rows))
}
