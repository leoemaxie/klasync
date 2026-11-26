//! The event relay: drains the `material_events` outbox, mirrors each event to
//! the Redis stream, and delivers it to every plug-in that reacts to events.
//!
//! Events are written in the same transaction as the change that caused them,
//! so a crash can lose a relay but never an event. Plug-ins therefore see each
//! event at least once and must be idempotent (the built-ins are).
//!
//! Event types: `material.generated`, `material.published`,
//! `material.unpublished`, `material.delivery_requested`, `book.stale`,
//! `book.published`.

use std::time::Duration;

use serde_json::Value;
use sqlx::FromRow;

use super::{
    delivery,
    registry::{registry, MaterialEvent},
};
use crate::{activity, state::AppState};

#[derive(Debug, FromRow)]
struct EventRow {
    id: i64,
    event_type: String,
    payload: Value,
}

/// Relays up to `limit` pending events. Returns how many were handled.
pub async fn relay_once(state: &AppState, limit: i64) -> Result<usize, String> {
    let pool = state.db_pool();
    // Claim a batch; `skip locked` lets several instances share the outbox.
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let rows = sqlx::query_as::<_, EventRow>(
        "select id, event_type, payload from material_events
         where relayed_at is null order by id limit $1 for update skip locked",
    )
    .bind(limit)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    if rows.is_empty() {
        drop(tx);
        sweep_deliveries(state).await;
        return Ok(0);
    }
    // Deliveries (email) are slow, so they run after the outbox rows are
    // released; the sweep below picks up any a crash left queued.
    let mut deliveries: Vec<uuid::Uuid> = Vec::new();
    for row in &rows {
        let event = MaterialEvent {
            id: row.id,
            event_type: row.event_type.clone(),
            payload: row.payload.clone(),
        };
        if let Some(redis) = &state.redis {
            if let Err(error) = redis
                .publish_material_event(&event.event_type, &event.payload.to_string())
                .await
            {
                tracing::warn!(%error, event = %event.event_type, "Could not mirror a material event to Redis");
            }
        }
        if event.event_type == "material.delivery_requested" {
            if let Some(id) = event
                .payload
                .get("delivery_id")
                .and_then(Value::as_str)
                .and_then(|s| uuid::Uuid::parse_str(s).ok())
            {
                deliveries.push(id);
            }
        }
        registry().dispatch_event(state, &event).await;
        sqlx::query("update material_events set relayed_at = now() where id = $1")
            .bind(row.id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    }
    tx.commit().await.map_err(|e| e.to_string())?;
    for id in deliveries {
        if let Err(error) = delivery::process(state, id).await {
            tracing::warn!(%error, delivery = %id, "Material delivery failed");
        }
    }
    sweep_deliveries(state).await;
    Ok(rows.len())
}

/// Deliveries still queued a minute after being requested (a relay crashed
/// between commit and send) are processed now. `process` claims atomically.
async fn sweep_deliveries(state: &AppState) {
    let stale: Vec<uuid::Uuid> = sqlx::query_scalar(
        "select id from material_deliveries
         where status = 'queued' and created_at < now() - interval '1 minute' limit 5",
    )
    .fetch_all(state.db_pool())
    .await
    .unwrap_or_default();
    for id in stale {
        if let Err(error) = delivery::process(state, id).await {
            tracing::warn!(%error, delivery = %id, "Swept material delivery failed");
        }
    }
}

/// Background loop, gated on activity like the other workers.
pub async fn run_loop(state: AppState) {
    loop {
        activity::sleep(Duration::from_secs(5)).await;
        if !activity::db_active() {
            continue;
        }
        match relay_once(&state, 25).await {
            Ok(_) => {}
            Err(error) => tracing::warn!(%error, "Material event relay failed"),
        }
    }
}
