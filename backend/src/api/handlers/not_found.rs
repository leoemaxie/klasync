use axum::{http::StatusCode, Json};
use serde_json::{json, Value};

pub async fn not_found(uri: axum::http::Uri) -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": "Route not Found",
            "message": format!("The requested endpoint '{}' was not found on this server", uri.path()),
            "status": 404
        })),
    )
}
