use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::{
    api::{
        error::{ApiError, LogApiError},
        handlers::sessions::database_session_by_code,
    },
    auth::guard::AuthenticatedLecturer,
    models::{CaptionChunk, PublishCaptionRequest, SessionStatus},
    state::AppState,
};

pub async fn list(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
) -> Result<Json<Vec<CaptionChunk>>, ApiError> {
    let pool = state.db_pool();
    let session = database_session_by_code(pool, &short_code).await?;
    let captions = sqlx::query_as!(
        CaptionChunk,
        "select id, session_id, text, created_at from caption_chunks where session_id = $1 order by sequence_number",
        session.id
    )
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
    let captions_paused = sqlx::query_scalar!(
        r#"select coalesce((select captions_paused from session_live_controls where session_id = $1), false) as "coalesce!""#,
        session.id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to query live controls");
        ApiError::service_unavailable()
    })?;
    if captions_paused {
        return Err(ApiError::conflict("Captions are paused"));
    }
    let owns_session = sqlx::query_scalar!(
        r#"select exists(select 1 from lecture_sessions where id = $1 and lecturer_id = $2) as "exists!""#,
        session.id,
        lecturer.id
    )
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
    let lock_key = session.id.to_string();
    sqlx::query!("select pg_advisory_xact_lock(hashtext($1::text))", lock_key)
        .execute(&mut *tx)
        .await
        .map_err(|error| {
            tracing::error!(%error, "Failed to acquire advisory lock for caption sequencing");
            ApiError::service_unavailable()
        })?;

    let caption_id = Uuid::now_v7();
    let caption = sqlx::query_as!(
        CaptionChunk,
        "insert into caption_chunks (id, session_id, sequence_number, text, created_at) \
         values ($1, $2, (select coalesce(max(sequence_number), 0) + 1 from caption_chunks where session_id = $2), $3, $4) \
         returning id, session_id, text, created_at",
        caption_id,
        session.id,
        text,
        chrono::Utc::now()
    )
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

    let payload = serde_json::to_string(&caption)
        .log_internal_error("Failed to serialize caption payload")?;
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
