pub mod error;
pub mod handlers;

use std::net::SocketAddr;

use axum::{
    http::{header, Method},
    routing::{get, post},
    Router,
};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use crate::state::AppState;
use handlers::*;

pub fn router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE]);

    Router::new()
        .route("/health", get(health))
        .route("/api/v1/lecturers/register", post(register_lecturer))
        .route("/api/v1/courses", get(list_courses).post(create_course))
        .route("/api/v1/courses/{course_id}/roster", post(upload_roster))
        .route("/api/v1/sessions", post(create_session))
        .route(
            "/api/v1/sessions/code/{short_code}",
            get(get_session_by_code),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/join",
            post(join_session),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/participants",
            get(list_session_participants),
        )
        .route("/api/v1/sessions/code/{short_code}/end", post(end_session))
        .route(
            "/api/v1/sessions/code/{short_code}/captions",
            get(list_captions).post(publish_caption),
        )
        .route(
            "/api/v1/participants/{participant_id}/heartbeat",
            post(heartbeat),
        )
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

pub async fn start_server() {
    let state = AppState::default();
    let app = router(state);

    let address = SocketAddr::from(([127, 0, 0, 1], 8787));
    println!("KLASYNC API listening on http://{address}");
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("bind API port");

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("run API server");
    });
}
