use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::{api::error::ApiError, auth::guard::AuthenticatedLecturer, state::AppState};

#[derive(Debug, Serialize, FromRow)]
pub struct CourseAttendanceSummary {
    pub course_id: Uuid,
    pub total_sessions: i64,
    pub avg_attendance_percentage: f64,
    pub roster_verification_match_rate: f64,
    pub total_provisional_students: i64,
    pub total_anomalies_flagged: i64,
}

#[derive(Debug, Serialize, FromRow)]
pub struct AttendanceAnomaly {
    pub id: Uuid,
    pub matric_number: String,
    pub anomaly_type: String,
    pub description: String,
    pub severity: String,
    pub logged_at: DateTime<Utc>,
}

pub async fn course_summary(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(course_id): Path<Uuid>,
) -> Result<Json<CourseAttendanceSummary>, ApiError> {
    let pool = state
        .production_database()
        .ok_or_else(|| ApiError::service_unavailable())?;
    ensure_course_owner(pool, course_id, lecturer.id).await?;
    let summary = sqlx::query_as::<_, CourseAttendanceSummary>(
        "select $1 as course_id,
          count(distinct s.id)::bigint as total_sessions,
          coalesce(avg(coalesce(p.attendance_score, 0)) filter (where p.id is not null), 0)::double precision as avg_attendance_percentage,
          coalesce(100.0 * avg(case when p.verification_status = 'verified' then 1.0 else 0.0 end) filter (where p.id is not null), 0)::double precision as roster_verification_match_rate,
          count(*) filter (where p.verification_status = 'provisional')::bigint as total_provisional_students,
          (select count(*) from attendance_audit_logs a where a.session_id in (select id from lecture_sessions where course_id = $1))::bigint as total_anomalies_flagged
         from lecture_sessions s left join session_participants p on p.session_id = s.id
         where s.course_id = $1 and s.deleted_at is null",
    )
    .bind(course_id).fetch_one(pool).await
    .map_err(|_| ApiError::service_unavailable())?;
    Ok(Json(summary))
}

pub async fn session_anomalies(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(session_id): Path<Uuid>,
) -> Result<Json<Vec<AttendanceAnomaly>>, ApiError> {
    let pool = state
        .production_database()
        .ok_or_else(|| ApiError::service_unavailable())?;
    let owns: bool = sqlx::query_scalar(
        "select exists(select 1 from lecture_sessions where id = $1 and lecturer_id = $2)",
    )
    .bind(session_id)
    .bind(lecturer.id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?;
    if !owns {
        return Err(ApiError::not_found("Session not found."));
    }
    let anomalies = sqlx::query_as::<_, AttendanceAnomaly>("select id, matric_number, anomaly_type, description, severity, logged_at from attendance_audit_logs where session_id = $1 order by logged_at desc")
        .bind(session_id).fetch_all(pool).await.map_err(|_| ApiError::service_unavailable())?;
    Ok(Json(anomalies))
}

async fn ensure_course_owner(
    pool: &sqlx::PgPool,
    course_id: Uuid,
    lecturer_id: Uuid,
) -> Result<(), ApiError> {
    let owns: bool = sqlx::query_scalar(
        "select exists(select 1 from courses where id = $1 and lecturer_id = $2)",
    )
    .bind(course_id)
    .bind(lecturer_id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?;
    if !owns {
        return Err(ApiError::not_found("Course not found."));
    }
    Ok(())
}
