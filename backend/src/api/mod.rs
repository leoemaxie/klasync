pub mod error;
pub mod handlers;

use axum::{
    http::{header, Method},
    routing::{get, post},
    Router,
};
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
};

use crate::{config::AppConfig, state::AppState};
use handlers::{
    attendance, auth, caption_stream, captions, claims, courses, health, invites, participants,
    recovery, resources, rosters, sessions,
};

pub fn router(state: AppState) -> Router {
    let origins: Vec<axum::http::HeaderValue> = state
        .config
        .cors_allowed_origins
        .iter()
        .filter_map(|origin| origin.parse().ok())
        .collect();

    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

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
        .route("/api/v1/auth/password-reset/request", post(recovery::request))
        .route("/api/v1/auth/password-reset/complete", post(recovery::complete))
        .route(
            "/api/v1/students/claims",
            post(claims::claim_guest_participation),
        )
        .route(
            "/api/v1/students/archive",
            get(resources::list_student_archive),
        )
        .route("/api/v1/courses", get(courses::list).post(courses::create))
        .route(
            "/api/v1/courses/{course_id}/roster",
            post(courses::upload_roster),
        )
        .route(
            "/api/v1/courses/{course_id}/roster/import",
            post(rosters::import_file),
        )
        .route("/api/v1/sessions", post(sessions::create))
        .route("/api/v1/invites/{token}", get(invites::resolve))
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
            "/api/v1/sessions/code/{short_code}/participants/{participant_id}/review",
            post(attendance::review),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/attendance.csv",
            get(attendance::export_csv),
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
            "/api/v1/sessions/code/{short_code}/captions/ws",
            get(caption_stream::connect),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/resources",
            post(resources::create_for_session),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/resources/{resource_type}/upload",
            post(handlers::uploads::upload),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/ai-jobs",
            get(handlers::jobs::list).post(handlers::jobs::create),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/invite/qr.svg",
            get(invites::qr_svg),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/invite/revoke",
            post(invites::revoke),
        )
        .route(
            "/api/v1/participants/{participant_id}/heartbeat",
            post(participants::heartbeat),
        )
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
