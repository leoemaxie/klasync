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
            "/api/v1/archive/sessions/{session_id}/ai/generate-visualization",
            post(handlers::ai_study::generate_visualization),
        )
        .route(
            "/api/v1/archive/sessions/{session_id}/visualizations",
            get(handlers::ai_study::visualizations),
        )
        .route(
            "/api/v1/archive/sessions/{session_id}/transcript",
            get(handlers::archive::transcript),
        )
        .route(
            "/api/v1/archive/sessions/{session_id}/summaries",
            get(handlers::archive::summaries),
        )
        .route(
            "/api/v1/archive/sessions/{session_id}/recordings",
            get(handlers::archive::recording_info),
        )
        .route(
            "/api/v1/archive/sessions/{session_id}/recordings/{resource_id}/stream",
            get(handlers::archive::recording_stream),
        )
        .route(
            "/api/v1/archive/sessions/{session_id}/ask",
            post(handlers::archive::ask),
        )
        .route("/api/v1/students/search", get(handlers::archive::search))
        .route(
            "/api/v1/archive/jobs/{job_id}",
            get(handlers::ai_study::job_status),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/summaries",
            get(handlers::live_extras::summaries_by_code),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/presence",
            post(handlers::live_extras::presence_by_code),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/hand-raises",
            get(handlers::live_extras::raised_hands),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/participants/{participant_id}/hand-raise",
            post(handlers::live_extras::raise_hand).delete(handlers::live_extras::lower_hand),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/polls",
            get(handlers::polls::list).post(handlers::polls::create),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/polls/generate",
            post(handlers::polls::generate),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/polls/{poll_id}/respond",
            post(handlers::polls::respond),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/polls/{poll_id}/close",
            post(handlers::polls::close),
        )
        .route(
            "/api/v1/analytics/courses/{course_id}/attendance-summary",
            get(handlers::analytics::course_summary),
        )
        .route(
            "/api/v1/analytics/courses/{course_id}/anomalies",
            get(handlers::analytics::course_anomalies),
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
        .route("/api/v1/auth/students/me", get(auth::student_me))
        .route("/api/v1/auth/lecturers/me", get(auth::lecturer_me))
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
        .route("/api/v1/students/courses", get(courses::student_courses))
        .route(
            "/api/v1/students/courses/enroll",
            post(courses::enroll_course),
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
        .route(
            "/api/v1/materials/capabilities",
            get(handlers::materials::capabilities),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/materials",
            get(handlers::materials::get_material)
                .post(handlers::materials::generate)
                .patch(handlers::materials::patch_material),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/materials/draft",
            axum::routing::put(handlers::materials::save_draft)
                .layer(DefaultBodyLimit::max(8 * 1024 * 1024)),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/materials/sections/{block_id}/regenerate",
            post(handlers::materials::regenerate_section),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/materials/publish",
            post(handlers::materials::publish),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/materials/unpublish",
            post(handlers::materials::unpublish),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/materials/export",
            get(handlers::materials::export_material),
        )
        .route(
            "/api/v1/sessions/code/{short_code}/materials/versions/{version_id}",
            get(handlers::materials::get_version),
        )
        .route(
            "/api/v1/courses/{course_id}/materials",
            get(handlers::materials::list_course_materials),
        )
        .route(
            "/api/v1/courses/{course_id}/materials/settings",
            get(handlers::materials::get_settings).put(handlers::materials::put_settings),
        )
        .route(
            "/api/v1/courses/{course_id}/book",
            get(handlers::materials::get_book).put(handlers::materials::put_book),
        )
        .route(
            "/api/v1/courses/{course_id}/book/rebuild",
            post(handlers::materials::rebuild_book),
        )
        .route(
            "/api/v1/courses/{course_id}/book/publish",
            post(handlers::materials::publish_book),
        )
        .route(
            "/api/v1/courses/{course_id}/book/export",
            get(handlers::materials::export_book),
        )
        .route(
            "/api/v1/courses/{course_id}/compare",
            post(handlers::materials::compare),
        )
        .route(
            "/api/v1/courses/{course_id}/comparisons",
            get(handlers::materials::list_comparisons),
        )
        .route(
            "/api/v1/comparisons/{comparison_id}",
            get(handlers::materials::get_comparison),
        )
        .route(
            "/api/v1/students/sessions/{session_id}/material",
            get(handlers::materials::student_material),
        )
        .route(
            "/api/v1/students/sessions/{session_id}/material/export",
            get(handlers::materials::student_material_export),
        )
        .route(
            "/api/v1/students/courses/{course_id}/materials",
            get(handlers::materials::student_course_materials),
        )
        .route(
            "/api/v1/students/courses/{course_id}/book",
            get(handlers::materials::student_book),
        )
        .route(
            "/api/v1/students/courses/{course_id}/book/export",
            get(handlers::materials::student_book_export),
        )
        .fallback(not_found::not_found)
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(cors)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::security::rate_limit,
        ))
        .layer(middleware::from_fn(crate::activity::track))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// Building the router panics on conflicting routes; this keeps that from
    /// reaching production. No database connection is made (lazy pool).
    #[tokio::test]
    async fn router_builds_without_route_conflicts() {
        let config = crate::config::AppConfig::from_env();
        let state = AppState {
            database: sqlx::postgres::PgPoolOptions::new()
                .connect_lazy("postgres://u:p@127.0.0.1:1/none")
                .expect("lazy pool"),
            storage: crate::storage::adapter_from_config(&config),
            mailer: crate::email::sender_from_config(&config),
            ai: crate::ai::gateway_from_config(&config, None),
            config: Arc::new(config),
            captions: crate::realtime::CaptionHub::default(),
            redis: None,
        };
        let _ = router(state);
    }
}
