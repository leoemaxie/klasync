use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
};
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use crate::models::CaptionChunk;

#[derive(Clone, Default)]
pub struct CaptionHub {
    channels: Arc<RwLock<HashMap<Uuid, broadcast::Sender<CaptionChunk>>>>,
    /// Recently delivered caption ids. A caption reaches the hub twice on the
    /// instance that created it (direct publish + Redis echo); this keeps
    /// subscribers from seeing it twice.
    recent: Arc<Mutex<VecDeque<Uuid>>>,
}

impl CaptionHub {
    pub async fn subscribe(&self, session_id: Uuid) -> broadcast::Receiver<CaptionChunk> {
        {
            let read_guard = self.channels.read().await;
            if let Some(sender) = read_guard.get(&session_id) {
                return sender.subscribe();
            }
        }
        let mut write_guard = self.channels.write().await;
        write_guard
            .entry(session_id)
            .or_insert_with(|| broadcast::channel(128).0)
            .subscribe()
    }

    fn first_delivery(&self, id: Uuid) -> bool {
        let mut recent = self.recent.lock().unwrap_or_else(|e| e.into_inner());
        if recent.contains(&id) {
            return false;
        }
        recent.push_back(id);
        while recent.len() > 512 {
            recent.pop_front();
        }
        true
    }

    pub async fn publish(&self, caption: CaptionChunk) {
        if !self.first_delivery(caption.id) {
            return;
        }
        {
            let read_guard = self.channels.read().await;
            if let Some(sender) = read_guard.get(&caption.session_id) {
                let _ = sender.send(caption);
                return;
            }
        }
        let mut write_guard = self.channels.write().await;
        let sender = write_guard
            .entry(caption.session_id)
            .or_insert_with(|| broadcast::channel(128).0);
        let _ = sender.send(caption);
    }
}

/// Forwards captions published by *other* instances (via Redis) into the local
/// hub. Reconnects forever; local publishing keeps working while it is down.
pub fn spawn_redis_bridge(state: crate::state::AppState) {
    let Some(redis) = state.redis.clone() else {
        return;
    };
    tokio::spawn(async move {
        use futures_util::StreamExt;
        loop {
            match redis.listen_pattern_captions().await {
                Ok(mut pubsub) => {
                    let mut messages = pubsub.on_message();
                    while let Some(message) = messages.next().await {
                        let Ok(payload) = message.get_payload::<String>() else {
                            continue;
                        };
                        if let Ok(caption) = serde_json::from_str::<CaptionChunk>(&payload) {
                            state.captions.publish(caption).await;
                        }
                    }
                    tracing::warn!("Redis caption subscription ended; reconnecting");
                }
                Err(error) => {
                    tracing::warn!(%error, "Unable to subscribe to Redis captions; retrying");
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
    });
}
