use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    api::error::ApiError,
    auth::guard::AuthenticatedLecturer,
    models::{CreateSessionRequest, InviteResponse, LectureSession, SessionDetail, SessionStatus},
    state::AppState,
    utils::short_code,
};

const SESSION_COLUMNS: &str = "id, course_id, title, short_code, invite_token, status, started_at";

pub async fn create(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Json(input): Json<CreateSessionRequest>,
) -> Result<(StatusCode, Json<InviteResponse>), ApiError> {
    let pool = state.db_pool();
    let owns_course: bool = sqlx::query_scalar(
        "select exists(select 1 from courses where id = $1 and lecturer_id = $2)",
    )
    .bind(input.course_id)
    .bind(lecturer.id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?;
    if !owns_course {
        return Err(ApiError::not_found("Course not found"));
    }

    let code = short_code();
    let invite_token = Uuid::now_v7();
    let session = sqlx::query_as::<_, LectureSession>(&format!(
        "insert into lecture_sessions (course_id, title, short_code, invite_token, status, started_at, lecturer_id) \
         values ($1, $2, $3, $4, $5, $6, $7) returning {SESSION_COLUMNS}"
    ))
    .bind(input.course_id)
    .bind(input.title.trim())
    .bind(&code)
    .bind(invite_token)
    .bind(SessionStatus::Live)
    .bind(Utc::now())
    .bind(lecturer.id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::conflict("A session code collision occurred, please try again"))?;

    sqlx::query(
        "insert into session_invites (session_id, token, short_code, created_by) values ($1, $2, $3, $4)",
    )
    .bind(session.id)
    .bind(invite_token)
    .bind(&code)
    .bind(lecturer.id)
    .execute(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?;

    let join_url = format!("/?join={code}");
    Ok((
        StatusCode::CREATED,
        Json(InviteResponse {
            qr_payload: join_url.clone(),
            join_url,
            session,
        }),
    ))
}

pub async fn get_by_code(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
) -> Result<Json<SessionDetail>, ApiError> {
    let pool = state.db_pool();
    let session = database_session_by_code(pool, &short_code).await?;
    let course = sqlx::query_as("select id, lecturer_id, code, title from courses where id = $1")
        .bind(session.course_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::service_unavailable())?
        .ok_or_else(|| ApiError::not_found("Course not found"))?;
    let participant_count: i64 =
        sqlx::query_scalar("select count(*) from session_participants where session_id = $1")
            .bind(session.id)
            .fetch_one(pool)
            .await
            .map_err(|_| ApiError::service_unavailable())?;
    Ok(Json(SessionDetail {
        session,
        course,
        participant_count: participant_count as usize,
    }))
}

pub async fn end(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(short_code): Path<String>,
) -> Result<Json<LectureSession>, ApiError> {
    let pool = state.db_pool();
    let session = sqlx::query_as::<_, LectureSession>(&format!(
        "update lecture_sessions set status = 'ended', ended_at = now() \
         where short_code = upper($1) and lecturer_id = $2 returning {SESSION_COLUMNS}"
    ))
    .bind(short_code)
    .bind(lecturer.id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?
    .ok_or_else(|| ApiError::not_found("Session not found"))?;
    Ok(Json(session))
}

pub async fn database_session_by_code<'e, E>(
    executor: E,
    short_code: &str,
) -> Result<LectureSession, ApiError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query_as::<_, LectureSession>(&format!(
        "select {SESSION_COLUMNS} from lecture_sessions where short_code = upper($1)"
    ))
    .bind(short_code)
    .fetch_optional(executor)
    .await
    .map_err(|error| {
        tracing::error!(%error, "database_session_by_code failed");
        ApiError::service_unavailable()
    })?
    .ok_or_else(|| ApiError::not_found("Session not found"))
}

