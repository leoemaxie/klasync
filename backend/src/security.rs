use std::{collections::HashMap, sync::OnceLock, time::{Duration, Instant}};

use axum::{extract::{Request, State}, http::StatusCode, middleware::Next, response::{IntoResponse, Response}, Json};
use serde::Serialize;
use tokio::sync::Mutex;

use crate::state::AppState;

#[derive(Default)]
struct Bucket { started: Option<Instant>, count: u32 }

static BUCKETS: OnceLock<Mutex<HashMap<String, Bucket>>> = OnceLock::new();

#[derive(Serialize)]
struct RateLimitBody { error: &'static str }

pub async fn rate_limit(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let path = request.uri().path();
    let Some((scope, limit, window)) = policy(path) else { return next.run(request).await; };
    let identity = request.headers().get("x-forwarded-for")
        .and_then(|value| value.to_str().ok()).unwrap_or("anonymous");
    if let Some(redis) = &state.redis {
        match redis.consume_rate_limit(scope, identity, limit, window.as_secs()).await {
            Ok(true) => return next.run(request).await,
            Ok(false) => return (StatusCode::TOO_MANY_REQUESTS, Json(RateLimitBody { error: "Too many requests. Please try again shortly." })).into_response(),
            Err(error) if state.config.redis_required => {
                tracing::error!(%error, "Managed Redis rate limiter unavailable");
                return (StatusCode::SERVICE_UNAVAILABLE, Json(RateLimitBody { error: "This service is temporarily unavailable. Please try again shortly." })).into_response();
            }
            Err(error) => tracing::warn!(%error, "Managed Redis rate limiter unavailable; using local fallback"),
        }
    }
    let key = format!("{scope}:{identity}");
    let buckets = BUCKETS.get_or_init(|| Mutex::new(HashMap::new()));
    let now = Instant::now();
    let allowed = {
        let mut guard = buckets.lock().await;
        let bucket = guard.entry(key).or_default();
        if bucket.started.map(|started| now.duration_since(started) >= window).unwrap_or(true) {
            bucket.started = Some(now);
            bucket.count = 0;
        }
        if bucket.count >= limit { false } else { bucket.count += 1; true }
    };
    if !allowed {
        return (StatusCode::TOO_MANY_REQUESTS, Json(RateLimitBody { error: "Too many requests. Please try again shortly." })).into_response();
    }
    next.run(request).await
}

fn policy(path: &str) -> Option<(&'static str, u32, Duration)> {
    if path.contains("/auth/password-reset") { return Some(("password-reset", 5, Duration::from_secs(900))); }
    if path.contains("/auth/") { return Some(("auth", 12, Duration::from_secs(60))); }
    if path.contains("/students/claims") { return Some(("claims", 5, Duration::from_secs(300))); }
    if path.contains("/join") { return Some(("join", 30, Duration::from_secs(60))); }
    if path.contains("/ai-jobs") { return Some(("ai", 10, Duration::from_secs(60))); }
    if path.contains("/upload") { return Some(("upload", 20, Duration::from_secs(60))); }
    None
}
