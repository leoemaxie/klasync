use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;

use crate::{
    api::{error::ApiError, handlers::sessions::database_session_by_code},
    auth::guard::AuthenticatedLecturer,
    state::AppState,
};

#[derive(Debug, Serialize)]
pub struct ReconciliationResult {
    pub participants_scored: i64,
    pub duplicate_participants_flagged: i64,
}

pub async fn reconcile(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(short_code): Path<String>,
) -> Result<Json<ReconciliationResult>, ApiError> {
    let pool = state
        .production_database()
        .ok_or_else(|| ApiError::service_unavailable())?;
    let session = database_session_by_code(pool, &short_code).await?;
    let owns_session: bool = sqlx::query_scalar(
        "select exists(select 1 from lecture_sessions where id = $1 and lecturer_id = $2)",
    )
    .bind(session.id)
    .bind(lecturer.id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?;
    if !owns_session {
        return Err(ApiError::not_found("Session not found"));
    }

    let scored: i64 = sqlx::query_scalar(
        "with duration as (
           select greatest(60, extract(epoch from (coalesce(ended_at, now()) - started_at))) as seconds
           from lecture_sessions where id = $1
         ), updated as (
           update session_participants p
           set attendance_score = least(100.0, greatest(0.0,
             (p.heartbeat_count::numeric / greatest(1.0, (duration.seconds / 30.0))) * 100.0
           ))
           from duration where p.session_id = $1
           returning 1
         ) select count(*) from updated",
    )
    .bind(session.id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?;

    let flagged: i64 = sqlx::query_scalar(
        "with duplicates as (
           select matric_number from session_participants where session_id = $1 group by matric_number having count(*) > 1
         ), flagged as (
           update session_participants p set duplicate_flag = true
           from duplicates d where p.session_id = $1 and p.matric_number = d.matric_number
           returning 1
         ) select count(*) from flagged",
    )
    .bind(session.id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?;
    Ok(Json(ReconciliationResult {
        participants_scored: scored,
        duplicate_participants_flagged: flagged,
    }))
}
