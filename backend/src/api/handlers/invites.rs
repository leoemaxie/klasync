use axum::{
    extract::{Path, State},
    http::header,
    response::{IntoResponse, Response},
    Json,
};
use qrcode::{render::svg, QrCode};

use crate::{
    api::{error::ApiError, handlers::sessions::database_session_by_code},
    auth::guard::AuthenticatedLecturer,
    models::{Course, InviteResolution, LectureSession},
    state::AppState,
};

pub async fn qr_svg(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(short_code): Path<String>,
) -> Result<Response, ApiError> {
    let pool = state
        .production_database()
        .ok_or_else(|| ApiError::service_unavailable("database_not_configured"))?;
    let session = database_session_by_code(pool, &short_code).await?;
    let owns_session: bool = sqlx::query_scalar(
        "select exists(select 1 from lecture_sessions where id = $1 and lecturer_id = $2)",
    )
    .bind(session.id)
    .bind(lecturer.id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::service_unavailable("session_lookup_failed"))?;
    if !owns_session {
        return Err(ApiError::not_found("session_not_found"));
    }

    let payload = format!("/?invite={}", session.invite_token);
    let code =
        QrCode::new(payload).map_err(|_| ApiError::service_unavailable("qr_generation_failed"))?;
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
    let pool = state
        .production_database()
        .ok_or_else(|| ApiError::service_unavailable("database_not_configured"))?;
    let token = uuid::Uuid::parse_str(&token).map_err(|_| ApiError::not_found("invite_not_found"))?;
    let session = sqlx::query_as::<_, LectureSession>(
        "select session.id, session.course_id, session.title, session.short_code, session.invite_token, session.status, session.started_at \
         from session_invites invite join lecture_sessions session on session.id = invite.session_id \
         where invite.token = $1 and invite.revoked_at is null and (invite.expires_at is null or invite.expires_at > now())",
    )
    .bind(token)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::service_unavailable("invite_lookup_failed"))?
    .ok_or_else(|| ApiError::not_found("invite_not_found"))?;
    let course = sqlx::query_as::<_, Course>(
        "select id, lecturer_id, code, title from courses where id = $1",
    )
    .bind(session.course_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::service_unavailable("course_lookup_failed"))?
    .ok_or_else(|| ApiError::not_found("course_not_found"))?;
    Ok(Json(InviteResolution { session, course }))
}

pub async fn revoke(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(short_code): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    let pool = state
        .production_database()
        .ok_or_else(|| ApiError::service_unavailable("database_not_configured"))?;
    let result = sqlx::query(
        "update session_invites invite set revoked_at = now() from lecture_sessions session \
         where invite.session_id = session.id and invite.short_code = upper($1) and session.lecturer_id = $2 and invite.revoked_at is null",
    )
    .bind(short_code)
    .bind(lecturer.id)
    .execute(pool)
    .await
    .map_err(|_| ApiError::service_unavailable("invite_update_failed"))?;
    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("invite_not_found"));
    }
    Ok(axum::http::StatusCode::NO_CONTENT)
}
