pub mod error;
pub mod handlers;

use axum::{
    extract::DefaultBodyLimit,
    http::{header, Method},
    middleware,
    routing::{get, post},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::state::AppState;
use handlers::{
    attendance, auth, caption_stream, captions, claims, courses, health, invites, not_found,
    participants, recovery, resources, rosters, sessions,
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
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .allow_credentials(true);

    Router::new()
        .route("/", get(health::health))
        .route("/health", get(health::health))
        .route("/health/ready", get(handlers::readiness::ready))
        .route(
            "/api/v1/sessions/{code}/questions",
            get(handlers::questions::list).post(handlers::questions::submit),
        )
        .route(
            "/api/v1/sessions/{code}/questions/{question_id}/upvote",
            post(handlers::questions::upvote),
        )
        .route(
            "/api/v1/sessions/{code}/questions/{question_id}/resolve",
            post(handlers::questions::resolve),
        )
        .route(
            "/api/v1/sessions/code/{code}/questions",
            get(handlers::questions::list).post(handlers::questions::submit),
        )
        .route(
            "/api/v1/sessions/code/{code}/questions/{question_id}/upvote",
            post(handlers::questions::upvote),
        )
        .route(
            "/api/v1/sessions/code/{code}/questions/{question_id}/resolve",
            post(handlers::questions::resolve),
        )
        .route(
            "/api/v1/archive/sessions/{session_id}/ai/generate-chapters",
            post(handlers::ai_study::generate_chapters),
        )
        .route(
            "/api/v1/archive/sessions/{session_id}/chapters",
            get(handlers::ai_study::chapters),
        )
        .route(
            "/api/v1/archive/sessions/{session_id}/ai/generate-flashcards",
            post(handlers::ai_study::generate_flashcards),
        )
        .route(
            "/api/v1/archive/sessions/{session_id}/flashcards",
            get(handlers::ai_study::flashcards),
        )
        .route(
            "/api/v1/analytics/courses/{course_id}/attendance-summary",
            get(handlers::analytics::course_summary),
        )
        .route(
            "/api/v1/analytics/sessions/{session_id}/anomalies",
            get(handlers::analytics::session_anomalies),
        )
        .route(
            "/api/v1/courses/{course_id}/lms-sync/canvas",
            post(handlers::lms_sync::canvas),
        )
        .route(
            "/api/v1/sessions/batch-sync",
            post(handlers::batch_sync::sync),
        )
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
            "/api/v1/auth/password-reset/request",
            post(recovery::request),
        )
        .route(
            "/api/v1/auth/password-reset/complete",
            post(recovery::complete),
        )
        .route("/api/v1/resources", get(resources::list_public_resources))
        .route(
            "/api/v1/students/claims",
            post(claims::claim_guest_participation),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/claims",
            post(claims::claim_guest_participation),
        )
        .route(
            "/api/v1/students/claims/request-verification",
            post(handlers::student_claims::request),
        )
        .route(
            "/api/v1/students/claims/verify",
            post(handlers::student_claims::verify),
        )
        .route(
            "/api/v1/students/archive",
            get(resources::list_student_archive),
        )
        .route(
            "/api/v1/students/resources/{resource_id}/download",
            get(resources::download_for_student),
        )
        .route("/api/v1/courses", get(courses::list).post(courses::create))
        .route(
            "/api/v1/courses/{course_id}/roster",
            get(courses::get_roster).post(courses::upload_roster),
        )
        .route(
            "/api/v1/courses/{course_id}/roster/import",
            post(rosters::import_file).layer(DefaultBodyLimit::max(50 * 1024 * 1024)),
        )
        .route("/api/v1/sessions", post(sessions::create))
        .route("/api/v1/invites/{token}", get(invites::resolve))
        .route(
            "/api/v1/sessions/code/{short_code}",
            get(sessions::get_by_code).patch(handlers::lifecycle::update),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/archive",
            post(handlers::lifecycle::archive),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/reopen",
            post(handlers::lifecycle::reopen),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/delete",
            post(handlers::lifecycle::remove),
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
            "/api/v1/sessions/code/{short_code}/attendance/reconcile",
            post(handlers::attendance_scoring::reconcile),
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
            "/api/v1/sessions/code/{short_code}/live-controls",
            post(handlers::live_controls::update),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/captions/pause",
            post(handlers::live_controls::pause_captions),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/pause",
            post(handlers::live_controls::pause_captions),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/captions/resume",
            post(handlers::live_controls::resume_captions),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/resume",
            post(handlers::live_controls::resume_captions),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/audio/start",
            post(handlers::live_controls::start_audio),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/audio/stop",
            post(handlers::live_controls::stop_audio),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/participants/{participant_id}/{action}",
            post(handlers::live_controls::participant_action),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/captions/{caption_id}/moderate",
            post(handlers::live_controls::moderate_caption),
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
            post(handlers::uploads::upload).layer(DefaultBodyLimit::max(50 * 1024 * 1024)),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/resources/{resource_id}/download",
            get(resources::download_for_lecturer),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/audio/upload",
            post(handlers::audio::upload).layer(DefaultBodyLimit::max(50 * 1024 * 1024)),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/audio/ws",
            get(handlers::audio_stream::connect),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/ai-jobs",
            get(handlers::jobs::list).post(handlers::jobs::create),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/ai-jobs/{job_id}/dispatch",
            post(crate::ai_worker::dispatch),
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
        .route(
            "/api/v1/participants/{participant_id}/presence",
            post(handlers::presence::heartbeat),
        )
        .fallback(not_found::not_found)
        .layer(DefaultBodyLimit::max(1 * 1024 * 1024))
        .layer(cors)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::security::rate_limit,
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
