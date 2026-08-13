use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    api::{error::ApiError, handlers::sessions::database_session_by_code},
    auth::guard::AuthenticatedLecturer,
    models::{CaptionChunk, PublishCaptionRequest, SessionStatus},
    state::AppState,
};

const CAPTION_COLUMNS: &str = "id, session_id, text, created_at";

pub async fn list(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
) -> Result<Json<Vec<CaptionChunk>>, ApiError> {
    let pool = state.db_pool();
    let session = database_session_by_code(pool, &short_code).await?;
    let captions = sqlx::query_as::<_, CaptionChunk>(&format!(
        "select {CAPTION_COLUMNS} from caption_chunks where session_id = $1 order by sequence_number"
    ))
    .bind(session.id)
    .fetch_all(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to list captions");
        ApiError::service_unavailable()
    })?;
    Ok(Json(captions))
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
    let pool = state.db_pool();
    let mut tx = pool.begin().await.map_err(|error| {
        tracing::error!(%error, "Failed to start transaction for caption publish");
        ApiError::service_unavailable()
    })?;

    let session = database_session_by_code(&mut *tx, &short_code).await?;
    if !matches!(session.status, SessionStatus::Live) {
        return Err(ApiError::conflict(
            "Captions can only be published to live sessions",
        ));
    }
    let captions_paused: bool = sqlx::query_scalar(
        "select coalesce((select captions_paused from session_live_controls where session_id = $1), false)",
    )
    .bind(session.id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to query live controls");
        ApiError::service_unavailable()
    })?;
    if captions_paused {
        return Err(ApiError::conflict("Captions are paused"));
    }
    let owns_session: bool = sqlx::query_scalar(
        "select exists(select 1 from lecture_sessions where id = $1 and lecturer_id = $2)",
    )
    .bind(session.id)
    .bind(lecturer.id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to check session ownership");
        ApiError::service_unavailable()
    })?;
    if !owns_session {
        return Err(ApiError::not_found("Session not found"));
    }

    // Acquire transaction-scoped advisory lock on session ID to serialize sequence number updates
    sqlx::query("select pg_advisory_xact_lock(hashtext($1::text))")
        .bind(session.id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|error| {
            tracing::error!(%error, "Failed to acquire advisory lock for caption sequencing");
            ApiError::service_unavailable()
        })?;

    let caption = sqlx::query_as::<_, CaptionChunk>(&format!(
        "insert into caption_chunks (id, session_id, sequence_number, text, created_at) \
         values ($1, $2, (select coalesce(max(sequence_number), 0) + 1 from caption_chunks where session_id = $2), $3, $4) \
         returning {CAPTION_COLUMNS}"
    ))
    .bind(Uuid::now_v7())
    .bind(session.id)
    .bind(text)
    .bind(Utc::now())
    .fetch_one(&mut *tx)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to insert caption chunk");
        ApiError::service_unavailable()
    })?;

    tx.commit().await.map_err(|error| {
        tracing::error!(%error, "Failed to commit caption chunk transaction");
        ApiError::service_unavailable()
    })?;

    let payload = serde_json::to_string(&caption).map_err(|_| ApiError::service_unavailable())?;
    if let Some(redis) = &state.redis {
        if let Err(error) = redis
            .publish_caption(&session.id.to_string(), &payload)
            .await
        {
            tracing::warn!(%error, "Redis caption publish failed; using local broadcast");
            state.captions.publish(caption.clone()).await;
        }
    } else {
        state.captions.publish(caption.clone()).await;
    }
    Ok((StatusCode::CREATED, Json(caption)))
}

