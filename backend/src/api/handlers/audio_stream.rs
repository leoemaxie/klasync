use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::Response,
};
use futures_util::StreamExt;
use std::time::Duration;
use tokio::time::interval;
use uuid::Uuid;

use crate::{
    ai::AiWorkItem,
    api::{error::ApiError, handlers::sessions::database_session_by_code},
    models::CaptionChunk,
    state::AppState,
};

pub async fn connect(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
    websocket: WebSocketUpgrade,
) -> Result<Response, ApiError> {
    let pool = state.db_pool();
    let session = database_session_by_code(pool, &short_code).await?;

    Ok(websocket.on_upgrade(move |socket| handle_socket(socket, state, session.id)))
}

async fn handle_socket(socket: WebSocket, state: AppState, session_id: Uuid) {
    let (_, mut receiver) = socket.split();
    let mut header_bytes = Vec::new();
    let mut chunk_buffer = Vec::new();
    let mut flush_interval = interval(Duration::from_secs(5));

    // The first frame from MediaRecorder usually contains the WebM/Ogg header.
    // We'll capture the first few bytes.
    let mut is_first = true;

    loop {
        tokio::select! {
            message = receiver.next() => match message {
                Some(Ok(Message::Binary(data))) => {
                    if is_first {
                        header_bytes.extend_from_slice(&data);
                        is_first = false;
                    } else {
                        chunk_buffer.extend_from_slice(&data);
                    }
                }
                Some(Ok(Message::Close(_))) | None => break,
                Some(Err(_)) => break,
                _ => {}
            },
            _ = flush_interval.tick() => {
                if chunk_buffer.is_empty() {
                    continue;
                }

                // Prepare the payload: header + accumulated chunks
                let mut payload = header_bytes.clone();
                payload.append(&mut chunk_buffer); // This clears chunk_buffer

                let state_clone = state.clone();

                tokio::spawn(async move {
                    let input = crate::ai::transcription_input(&payload, "webm", Some("en"));
                    let work = AiWorkItem {
                        job_id: Uuid::now_v7(),
                        session_id,
                        job_type: "transcribe".to_owned(),
                        input,
                    };

                    if let Ok(result) = state_clone.ai.execute(work).await {
                        if let Some(text) = result.content.get("text").and_then(|v| v.as_str()) {
                            if !text.trim().is_empty() {
                                let caption = CaptionChunk {
                                    id: Uuid::now_v7(),
                                    session_id,
                                    text: text.trim().to_owned(),
                                    created_at: chrono::Utc::now(),
                                };
                                let _ = state_clone.captions.publish(caption.clone()).await;

                                // Also broadcast via redis if configured
                                if let Some(redis) = &state_clone.redis {
                                    if let Ok(payload) = serde_json::to_string(&caption) {
                                        let _ = redis.publish_caption(&session_id.to_string(), &payload).await;
                                    }
                                }
                            }
                        }
                    }
                });
            }
        }
    }
}
