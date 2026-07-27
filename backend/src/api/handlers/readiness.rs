use axum::{extract::State, http::StatusCode, Json};

use crate::state::AppState;

pub async fn ready(State(state): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let redis_status = match &state.redis {
        Some(redis) => match redis.ping().await {
            Ok(()) => "healthy",
            Err(_) => "unavailable",
        },
        None => "not_configured",
    };
    if state.config.redis_required && redis_status != "healthy" {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    Ok(Json(serde_json::json!({
        "service": "klasync-backend",
        "status": "ready",
        "dependencies": {
            "postgres": state.production_database().is_some(),
            "redis": redis_status
        }
    })))
}
