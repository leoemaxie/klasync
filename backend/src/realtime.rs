use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use crate::models::CaptionChunk;

#[derive(Clone, Default)]
pub struct CaptionHub {
    channels: Arc<RwLock<HashMap<Uuid, broadcast::Sender<CaptionChunk>>>>,
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

    pub async fn publish(&self, caption: CaptionChunk) {
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

