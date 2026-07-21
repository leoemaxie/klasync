use std::{collections::HashMap, sync::Arc};

use tokio::sync::{broadcast, Mutex};
use uuid::Uuid;

use crate::models::CaptionChunk;

#[derive(Clone, Default)]
pub struct CaptionHub {
    channels: Arc<Mutex<HashMap<Uuid, broadcast::Sender<CaptionChunk>>>>,
}

impl CaptionHub {
    pub async fn subscribe(&self, session_id: Uuid) -> broadcast::Receiver<CaptionChunk> {
        let mut channels = self.channels.lock().await;
        channels
            .entry(session_id)
            .or_insert_with(|| broadcast::channel(128).0)
            .subscribe()
    }

    pub async fn publish(&self, caption: CaptionChunk) {
        let mut channels = self.channels.lock().await;
        let sender = channels
            .entry(caption.session_id)
            .or_insert_with(|| broadcast::channel(128).0);
        let _ = sender.send(caption);
    }
}
