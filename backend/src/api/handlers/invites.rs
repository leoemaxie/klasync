use axum::{
    extract::{Path, State},
    http::header,
    response::{IntoResponse, Response},
    Json,
};
use qrcode::{render::svg, QrCode};

use crate::{
    api::{
        error::{ApiError, LogApiError},
        handlers::sessions::database_session_by_code,
    },
    auth::guard::AuthenticatedLecturer,
    models::{Course, InviteResolution, LectureSession, SessionStatus},
    state::AppState,
};

pub async fn qr_svg(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(short_code): Path<String>,
) -> Result<Response, ApiError> {
    let pool = state.db_pool();
    let session = database_session_by_code(pool, &short_code).await?;
    let owns_session = sqlx::query_scalar!(
        r#"select exists(select 1 from lecture_sessions where id = $1 and lecturer_id = $2) as "exists!""#,
        session.id,
        lecturer.id
    )
    .fetch_one(pool)
    .await
    .log_internal_error("Failed to verify session ownership for QR SVG")?;
    if !owns_session {
        return Err(ApiError::not_found("Session not found"));
    }

    let payload = format!("/?invite={}", session.invite_token);
    let code = QrCode::new(payload).log_internal_error("Failed to generate QR code SVG")?;
    let svg = code
        .render::<svg::Color>()
        .min_dimensions(512, 512)
        .dark_color(svg::Color("#100904"))
        .light_color(svg::Color("#ffedd7"))
        .build();
    Ok((
        [(header::CONTENT_TYPE, "image/svg+xml; charset=utf-8")],
        svg,
    )
        .into_response())
}

pub async fn resolve(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<InviteResolution>, ApiError> {
    let pool = state.db_pool();
    let token = uuid::Uuid::parse_str(&token)
        .map_err(|_| ApiError::not_found("Invite link not found or expired"))?;
    let session = sqlx::query_as!(
        LectureSession,
        r#"select session.id as "id!", session.course_id as "course_id!", session.title as "title!", session.short_code as "short_code!", session.invite_token as "invite_token!", session.status as "status!: SessionStatus", session.started_at as "started_at!"
         from session_invites invite join lecture_sessions session on session.id = invite.session_id
         where invite.token = $1 and invite.revoked_at is null and (invite.expires_at is null or invite.expires_at > now())"#,
        token
    )
    .fetch_optional(pool)
    .await
    .log_internal_error("Failed to resolve invite token")?
    .ok_or_else(|| ApiError::not_found("Invite link not found or expired"))?;
    let course = sqlx::query_as!(
        Course,
        "select id, lecturer_id, code, title, academic_session, semester, is_active from courses where id = $1",
        session.course_id
    )
    .fetch_optional(pool)
    .await
    .log_internal_error("Failed to query course for resolved invite")?
    .ok_or_else(|| ApiError::not_found("Course not found"))?;
    Ok(Json(InviteResolution { session, course }))
}

pub async fn revoke(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(short_code): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    let pool = state.db_pool();
    let result = sqlx::query!(
        "update session_invites invite set revoked_at = now() from lecture_sessions session
         where invite.session_id = session.id and invite.short_code = upper($1) and session.lecturer_id = $2 and invite.revoked_at is null",
        short_code,
        lecturer.id
    )
    .execute(pool)
    .await
    .log_internal_error("Failed to revoke session invite")?;
    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Invite link not found or expired"));
    }
    Ok(axum::http::StatusCode::NO_CONTENT)
}
