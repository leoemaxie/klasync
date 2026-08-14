use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    api::error::{ApiError, LogApiError},
    state::AppState,
};

#[derive(Debug, Serialize)]
pub struct PresenceHeartbeat {
    pub participant_id: Uuid,
    pub heartbeat_count: i32,
    pub presence_ttl_seconds: u64,
}

pub async fn heartbeat(
    State(state): State<AppState>,
    Path(participant_id): Path<Uuid>,
) -> Result<(StatusCode, Json<PresenceHeartbeat>), ApiError> {
    let pool = state.db_pool();
    let participant = sqlx::query!(
        "update session_participants set last_seen_at = now(), heartbeat_count = heartbeat_count + 1 where id = $1 and removed_at is null returning id, session_id, heartbeat_count",
        participant_id
    )
    .fetch_optional(pool)
    .await
    .log_internal_error("Failed to record presence heartbeat")?
    .ok_or_else(|| ApiError::not_found("Participant not found or no longer active"))?;
    const TTL: u64 = 90;
    if let Some(redis) = &state.redis {
        if let Err(error) = redis
            .set_presence(&participant.session_id.to_string(), &participant.id.to_string(), TTL)
            .await
        {
            if state.config.redis_required {
                tracing::error!(%error, "Managed Redis presence update failed");
                return Err(ApiError::service_unavailable());
            }
            tracing::warn!(%error, "Managed Redis presence update failed");
        }
    }
    Ok((
        StatusCode::OK,
        Json(PresenceHeartbeat {
            participant_id: participant.id,
            heartbeat_count: participant.heartbeat_count,
            presence_ttl_seconds: TTL,
        }),
    ))
}
