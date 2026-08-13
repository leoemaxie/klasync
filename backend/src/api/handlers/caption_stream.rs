use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};

use crate::{
    api::{error::ApiError, handlers::sessions::database_session_by_code},
    models::CaptionChunk,
    state::AppState,
};

pub async fn connect(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
    websocket: WebSocketUpgrade,
) -> Result<Response, ApiError> {
    let pool = state
        .production_database()
        .ok_or_else(|| ApiError::service_unavailable())?;
    let session = database_session_by_code(pool, &short_code).await?;
    let receiver = state.captions.subscribe(session.id).await;
    let redis_stream = match &state.redis {
        Some(redis) => redis.subscribe_captions(&session.id.to_string()).await.ok(),
        None => None,
    };
    Ok(websocket.on_upgrade(move |socket| async move {
        if let Some(pubsub) = redis_stream {
            stream_redis(socket, receiver, pubsub).await;
        } else {
            stream_local(socket, receiver).await;
        }
    }))
}

async fn stream_local(
    socket: WebSocket,
    mut captions: tokio::sync::broadcast::Receiver<CaptionChunk>,
) {
    let (mut sender, mut receiver) = socket.split();
    loop {
        tokio::select! {
            caption = captions.recv() => match caption {
                Ok(caption) => {
                    let Ok(payload) = serde_json::to_string(&caption) else { continue; };
                    if sender.send(Message::Text(payload.into())).await.is_err() { break; }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            },
            message = receiver.next() => match message {
                Some(Ok(Message::Close(_))) | None => break,
                Some(Err(_)) => break,
                _ => {}
            }
        }
    }
}

async fn stream_redis(
    socket: WebSocket,
    mut local_captions: tokio::sync::broadcast::Receiver<CaptionChunk>,
    mut pubsub: redis::aio::PubSub,
) {
    let (mut sender, mut receiver) = socket.split();
    let mut messages = pubsub.on_message();
    loop {
        tokio::select! {
            message = messages.next() => {
                let Some(message) = message else { break; };
                let Ok(payload) = message.get_payload::<String>() else { continue; };
                if sender.send(Message::Text(payload.into())).await.is_err() { break; }
            },
            caption = local_captions.recv() => match caption {
                Ok(_) => {},
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            },
            message = receiver.next() => match message {
                Some(Ok(Message::Close(_))) | None => break,
                Some(Err(_)) => break,
                _ => {}
            }
        }
    }
}
