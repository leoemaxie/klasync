//! Live microphone ingestion from the lecturer's device.
//!
//! The socket does two jobs:
//!   1. rolling speech-to-text so captions keep flowing when the browser's own
//!      recogniser is unavailable, and
//!   2. recording the whole lecture so students can replay it afterwards. The
//!      recording used to be thrown away, which is why the archive player had
//!      nothing to play.
//!
//! The recording is checkpointed to object storage every few minutes (and on
//! close), so a restart loses at most one interval of audio.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    response::Response,
};
use futures_util::StreamExt;
use serde::Deserialize;
use std::time::{Duration, Instant};
use tokio::time::interval;
use uuid::Uuid;

use crate::{
    ai::TranscribeRequest,
    api::{error::ApiError, handlers::sessions::database_session_by_code},
    auth::{contracts::AccountRole, tokens},
    models::CaptionChunk,
    state::AppState,
};

const CHECKPOINT_EVERY: Duration = Duration::from_secs(300);
const MAX_RECORDING_BYTES: usize = 150 * 1024 * 1024;
const MIN_RECORDING_BYTES: usize = 2 * 1024;

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    token: Option<String>,
}

pub async fn connect(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
    Query(query): Query<WsQuery>,
    websocket: WebSocketUpgrade,
) -> Result<Response, ApiError> {
    let pool = state.db_pool();
    let session = database_session_by_code(pool, &short_code).await?;

    // Browsers cannot attach an Authorization header to a WebSocket, so the
    // lecturer's access token travels in the query string. Without this check
    // anyone who knew a session code could inject captions and audio.
    let token = query
        .token
        .as_deref()
        .ok_or_else(|| ApiError::unauthorized("Lecturer access token is required"))?;
    let claims = tokens::validate_access_token(&state.config, token)
        .map_err(|_| ApiError::unauthorized("Invalid or expired access token"))?;
    if !matches!(claims.role, AccountRole::Lecturer) {
        return Err(ApiError::forbidden("Lecturer access required"));
    }
    let owns = sqlx::query_scalar::<_, bool>(
        "select exists(select 1 from lecture_sessions where id = $1 and lecturer_id = $2)",
    )
    .bind(session.id)
    .bind(claims.sub)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?;
    if !owns {
        return Err(ApiError::not_found("Session not found"));
    }

    Ok(websocket.on_upgrade(move |socket| handle_socket(socket, state, session.id)))
}

struct Recorder {
    resource_id: Option<Uuid>,
    last_key: Option<String>,
    started: Instant,
    stored_len: usize,
}

async fn checkpoint(state: &AppState, session_id: Uuid, recorder: &mut Recorder, audio: &[u8]) {
    if audio.len() < MIN_RECORDING_BYTES || audio.len() == recorder.stored_len {
        return;
    }
    let pool = state.db_pool();
    let stored = match state
        .storage
        .put("lecture-recording.webm", audio.to_vec())
        .await
    {
        Ok(stored) => stored,
        Err(error) => {
            tracing::error!(%error, %session_id, "Failed to store lecture recording checkpoint");
            return;
        }
    };
    let duration = recorder.started.elapsed().as_secs() as i32;
    let result = match recorder.resource_id {
        Some(resource_id) => {
            sqlx::query(
                "update lecture_resources set storage_key = $2, byte_size = $3, duration_sec = $4 where id = $1",
            )
            .bind(resource_id)
            .bind(&stored.key)
            .bind(stored.bytes as i64)
            .bind(duration)
            .execute(pool)
            .await
        }
        None => {
            let resource_id = Uuid::now_v7();
            let inserted = sqlx::query(
                "insert into lecture_resources (id, session_id, resource_type, storage_key, original_filename, content_type, byte_size, duration_sec)
                 values ($1, $2, 'audio', $3, 'lecture-recording.webm', 'audio/webm', $4, $5)",
            )
            .bind(resource_id)
            .bind(session_id)
            .bind(&stored.key)
            .bind(stored.bytes as i64)
            .bind(duration)
            .execute(pool)
            .await;
            if inserted.is_ok() {
                recorder.resource_id = Some(resource_id);
            }
            inserted
        }
    };
    match result {
        Ok(_) => {
            if let Some(previous) = recorder.last_key.replace(stored.key) {
                let _ = state.storage.delete(&previous).await;
            }
            recorder.stored_len = audio.len();
        }
        Err(error) => {
            tracing::error!(%error, %session_id, "Failed to record lecture audio resource");
            let _ = state.storage.delete(&stored.key).await;
        }
    }
}

async fn handle_socket(socket: WebSocket, state: AppState, session_id: Uuid) {
    let (_, mut receiver) = socket.split();
    let mut header_bytes: Vec<u8> = Vec::new();
    let mut pending: Vec<u8> = Vec::new();
    let mut recording: Vec<u8> = Vec::new();
    let mut recorder = Recorder {
        resource_id: None,
        last_key: None,
        started: Instant::now(),
        stored_len: 0,
    };
    let mut flush_interval = interval(Duration::from_secs(10));
    let mut checkpoint_interval = interval(CHECKPOINT_EVERY);
    checkpoint_interval.tick().await;

    // The first MediaRecorder frame carries the WebM/Ogg header.
    let mut is_first = true;

    loop {
        tokio::select! {
            message = receiver.next() => match message {
                Some(Ok(Message::Binary(data))) => {
                    if recording.len() + data.len() <= MAX_RECORDING_BYTES {
                        recording.extend_from_slice(&data);
                    }
                    if is_first {
                        header_bytes.extend_from_slice(&data);
                        is_first = false;
                    } else {
                        pending.extend_from_slice(&data);
                    }
                }
                Some(Ok(Message::Close(_))) | None => break,
                Some(Err(_)) => break,
                _ => {}
            },
            _ = checkpoint_interval.tick() => {
                checkpoint(&state, session_id, &mut recorder, &recording).await;
            }
            _ = flush_interval.tick() => {
                if pending.is_empty() {
                    continue;
                }
                let mut payload = header_bytes.clone();
                payload.append(&mut pending);
                let state_clone = state.clone();
                tokio::spawn(async move {
                    transcribe_and_publish(state_clone, session_id, payload).await;
                });
            }
        }
    }

    checkpoint(&state, session_id, &mut recorder, &recording).await;
}

async fn transcribe_and_publish(state: AppState, session_id: Uuid, payload: Vec<u8>) {
    let payload_len = payload.len();
    let result = match state
        .ai
        .transcribe(&TranscribeRequest {
            audio: payload,
            mime: "audio/webm".to_owned(),
            language: Some("English".to_owned()),
            duration_sec: None,
            timestamps: false,
            session_id: Some(session_id),
            job_id: None,
        })
        .await
    {
        Ok(result) => result,
        Err(error) => {
            tracing::warn!(%error, bytes = payload_len, "Live transcription chunk failed");
            return;
        }
    };
    let text = result.text.trim();
    tracing::debug!(bytes = payload_len, chars = text.len(), "Live transcription chunk");
    if text.is_empty() {
        return;
    }
    // The lecturer's browser recogniser also posts captions; skip ours while it
    // is actively producing them so the transcript has no duplicates.
    let pool = state.db_pool();
    let browser_active = sqlx::query_scalar::<_, bool>(
        "select exists(select 1 from caption_chunks where session_id = $1 and created_at > now() - interval '20 seconds')",
    )
    .bind(session_id)
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if browser_active {
        return;
    }
    if let Ok(caption) =
        crate::api::handlers::captions::store_caption(&state, session_id, text).await
    {
        crate::api::handlers::captions::broadcast_caption(&state, &caption).await;
    }
}

#[allow(dead_code)]
fn _assert_caption_type(_: &CaptionChunk) {}
