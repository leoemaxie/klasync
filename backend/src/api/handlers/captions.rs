use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    api::{
        error::ApiError,
        handlers::sessions::{database_session_by_code, find_by_code},
    },
    auth::guard::AuthenticatedLecturer,
    models::{CaptionChunk, PublishCaptionRequest, SessionStatus},
    state::AppState,
};

const CAPTION_COLUMNS: &str = "id, session_id, text, created_at";

pub async fn list(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
) -> Result<Json<Vec<CaptionChunk>>, ApiError> {
    if let Some(pool) = state.production_database() {
        let session = database_session_by_code(pool, &short_code).await?;
        let captions = sqlx::query_as::<_, CaptionChunk>(&format!(
            "select {CAPTION_COLUMNS} from caption_chunks where session_id = $1 order by sequence_number"
        ))
        .bind(session.id)
        .fetch_all(pool)
        .await
        .map_err(|_| ApiError::service_unavailable())?;
        return Ok(Json(captions));
    }
    let store = state.store.lock().await;
    let session = find_by_code(&store.sessions, &short_code)?;
    Ok(Json(
        store.captions.get(&session.id).cloned().unwrap_or_default(),
    ))
}

pub async fn publish(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(short_code): Path<String>,
    Json(input): Json<PublishCaptionRequest>,
) -> Result<(StatusCode, Json<CaptionChunk>), ApiError> {
    let text = input.text.trim();
    if text.is_empty() {
        return Err(ApiError::bad_request("Caption text cannot be empty"));
    }
    let pool = state
        .production_database()
        .ok_or_else(|| ApiError::service_unavailable())?;
    let session = database_session_by_code(pool, &short_code).await?;
      if !matches!(session.status, SessionStatus::Live) {
          return Err(ApiError::conflict("Captions can only be published to live sessions"));
      }
      let captions_paused: bool = sqlx::query_scalar(
          "select coalesce((select captions_paused from session_live_controls where session_id = $1), false)",
      )
      .bind(session.id)
      .fetch_one(pool)
      .await
      .map_err(|_| ApiError::service_unavailable())?;
      if captions_paused { return Err(ApiError::conflict("Captions are paused")); }
    let owns_session: bool = sqlx::query_scalar(
        "select exists(select 1 from lecture_sessions where id = $1 and lecturer_id = $2)",
    )
    .bind(session.id)
    .bind(lecturer.id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?;
    if !owns_session {
        return Err(ApiError::not_found("Session not found"));
    }
    let caption = sqlx::query_as::<_, CaptionChunk>(&format!(
        "insert into caption_chunks (id, session_id, sequence_number, text, created_at) \
         values ($1, $2, (select coalesce(max(sequence_number), 0) + 1 from caption_chunks where session_id = $2), $3, $4) \
         returning {CAPTION_COLUMNS}"
    ))
    .bind(Uuid::now_v7())
    .bind(session.id)
    .bind(text)
    .bind(Utc::now())
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?;
    let payload = serde_json::to_string(&caption)
        .map_err(|_| ApiError::service_unavailable())?;
    if let Some(redis) = &state.redis {
        if let Err(error) = redis.publish_caption(&session.id.to_string(), &payload).await {
            tracing::warn!(%error, "Redis caption publish failed; using local broadcast");
            state.captions.publish(caption.clone()).await;
        }
    } else {
        state.captions.publish(caption.clone()).await;
    }
    Ok((StatusCode::CREATED, Json(caption)))
}
