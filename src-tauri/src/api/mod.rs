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

use crate::{config::AppConfig, state::AppState};
use handlers::{
    auth, captions, claims, courses, health, lecturers, participants, resources, sessions,
};

pub fn router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE]);

    Router::new()
        .route("/health", get(health::health))
        .route(
            "/api/v1/auth/lecturers/register",
            post(auth::register_lecturer),
        )
        .route("/api/v1/auth/lecturers/login", post(auth::login_lecturer))
        .route(
            "/api/v1/auth/students/register",
            post(auth::register_student),
        )
        .route("/api/v1/auth/students/login", post(auth::login_student))
        .route("/api/v1/auth/refresh", post(auth::refresh))
        .route("/api/v1/auth/logout", post(auth::logout))
        .route(
            "/api/v1/students/claims",
            post(claims::claim_guest_participation),
        )
        .route(
            "/api/v1/students/archive",
            get(resources::list_student_archive),
        )
        .route("/api/v1/lecturers/register", post(lecturers::register))
        .route("/api/v1/courses", get(courses::list).post(courses::create))
        .route(
            "/api/v1/courses/{course_id}/roster",
            post(courses::upload_roster),
        )
        .route("/api/v1/sessions", post(sessions::create))
        .route(
            "/api/v1/sessions/code/{short_code}",
            get(sessions::get_by_code),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/join",
            post(participants::join),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/participants",
            get(participants::list_for_session),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/attendance",
            get(participants::attendance_summary),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/end",
            post(sessions::end),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/captions",
            get(captions::list).post(captions::publish),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/resources",
            post(resources::create_for_session),
        )
        .route(
            "/api/v1/participants/{participant_id}/heartbeat",
            post(participants::heartbeat),
        )
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

pub async fn start_server() {
    let config = AppConfig::from_env();
    let state = AppState::from_config(config)
        .await
        .expect("connect configured PostgreSQL database");
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
