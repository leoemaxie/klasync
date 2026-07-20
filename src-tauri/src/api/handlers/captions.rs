use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    api::{error::ApiError, handlers::sessions::find_by_code},
    models::{CaptionChunk, PublishCaptionRequest, SessionStatus},
    state::AppState,
};

pub async fn list(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
) -> Result<Json<Vec<CaptionChunk>>, ApiError> {
    let store = state.store.lock().await;
    let session = find_by_code(&store.sessions, &short_code)?;
    Ok(Json(
        store.captions.get(&session.id).cloned().unwrap_or_default(),
    ))
}

pub async fn publish(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
    Json(input): Json<PublishCaptionRequest>,
) -> Result<(StatusCode, Json<CaptionChunk>), ApiError> {
    let mut store = state.store.lock().await;
    let session = find_by_code(&store.sessions, &short_code)?.clone();
    if !matches!(session.status, SessionStatus::Live) {
        return Err(ApiError::conflict("session_not_live"));
    }
    let text = input.text.trim();
    if text.is_empty() {
        return Err(ApiError::bad_request("caption_text_required"));
    }
    let caption = CaptionChunk {
        id: Uuid::new_v4(),
        session_id: session.id,
        text: text.to_owned(),
        created_at: Utc::now(),
    };
    store
        .captions
        .entry(session.id)
        .or_default()
        .push(caption.clone());
    Ok((StatusCode::CREATED, Json(caption)))
}
