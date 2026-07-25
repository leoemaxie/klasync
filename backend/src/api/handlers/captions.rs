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
        .map_err(|_| ApiError::service_unavailable("caption_lookup_failed"))?;
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
        return Err(ApiError::bad_request("caption_text_required"));
    }
    let pool = state
        .production_database()
        .ok_or_else(|| ApiError::service_unavailable("database_not_configured"))?;
    let session = database_session_by_code(pool, &short_code).await?;
    if !matches!(session.status, SessionStatus::Live) {
        return Err(ApiError::conflict("session_not_live"));
    }
    let owns_session: bool = sqlx::query_scalar(
        "select exists(select 1 from lecture_sessions where id = $1 and lecturer_id = $2)",
    )
    .bind(session.id)
    .bind(lecturer.id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::service_unavailable("session_lookup_failed"))?;
    if !owns_session {
        return Err(ApiError::not_found("session_not_found"));
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
    .map_err(|_| ApiError::service_unavailable("caption_persistence_failed"))?;
    state.captions.publish(caption.clone()).await;
    Ok((StatusCode::CREATED, Json(caption)))
}
