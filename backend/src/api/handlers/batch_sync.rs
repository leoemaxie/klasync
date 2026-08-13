use axum::{extract::State, http::StatusCode, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    api::{
        error::{ApiError, LogApiError},
        handlers::sessions::database_session_by_code,
    },
    state::AppState,
};


#[derive(Debug, Deserialize)]
pub struct BatchSyncRequest {
    pub session_code: String,
    #[serde(default)]
    pub captions: Vec<OfflineCaption>,
    #[serde(default)]
    pub presence_heartbeats: Vec<OfflineHeartbeat>,
}

#[derive(Debug, Deserialize)]
pub struct OfflineCaption {
    pub text: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct OfflineHeartbeat {
    pub matric: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct BatchSyncResponse {
    pub processed_captions: i64,
    pub processed_heartbeats: i64,
    pub sync_status: &'static str,
}

pub async fn sync(
    State(state): State<AppState>,
    Json(input): Json<BatchSyncRequest>,
) -> Result<(StatusCode, Json<BatchSyncResponse>), ApiError> {
    if input.captions.len() > 500 || input.presence_heartbeats.len() > 1000 {
        return Err(ApiError::bad_request("The offline batch is too large."));
    }
    let pool = state.db_pool();
    let session = database_session_by_code(pool, &input.session_code).await?;
    let mut transaction = pool
        .begin()
        .await
        .log_internal_error("Failed to start transaction for batch sync")?;
    let mut processed_captions = 0_i64;
    for caption in input.captions {
        let text = caption.text.trim();
        if text.is_empty() || text.len() > 4000 {
            continue;
        }
        sqlx::query("insert into caption_chunks (id, session_id, sequence_number, text, created_at) values ($1, $2, (select coalesce(max(sequence_number), 0) + 1 from caption_chunks where session_id = $2), $3, $4)")
            .bind(uuid::Uuid::now_v7()).bind(session.id).bind(text).bind(caption.timestamp).execute(&mut *transaction).await
            .log_internal_error("Failed to insert offline caption chunk in batch sync")?;
        processed_captions += 1;
    }
    let mut processed_heartbeats = 0_i64;
    for heartbeat in input.presence_heartbeats {
        let updated = sqlx::query("update session_participants set last_seen_at = greatest(last_seen_at, $3), heartbeat_count = heartbeat_count + 1 where session_id = $1 and lower(matric_number) = lower($2) and removed_at is null")
            .bind(session.id).bind(heartbeat.matric.trim()).bind(heartbeat.timestamp).execute(&mut *transaction).await
            .log_internal_error("Failed to update participant heartbeat in batch sync")?;
        processed_heartbeats += updated.rows_affected() as i64;
    }
    transaction
        .commit()
        .await
        .log_internal_error("Failed to commit batch sync transaction")?;
    Ok((
        StatusCode::OK,
        Json(BatchSyncResponse {
            processed_captions,
            processed_heartbeats,
            sync_status: "complete",
        }),
    ))
}
