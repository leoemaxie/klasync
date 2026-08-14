use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::{
    api::{error::ApiError, handlers::sessions::database_session_by_code},
    auth::guard::AuthenticatedLecturer,
    models::{
        AttendanceSummary, JoinSessionRequest, SessionParticipant, SessionStatus,
        VerificationStatus,
    },
    state::AppState,
};

pub async fn join(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
    Json(input): Json<JoinSessionRequest>,
) -> Result<(StatusCode, Json<SessionParticipant>), ApiError> {
    let pool = state.db_pool();
    let session = database_session_by_code(pool, &short_code).await?;
    if !matches!(session.status, SessionStatus::Live) {
        return Err(ApiError::conflict(
            "This lecture session is not currently active",
        ));
    }
    let roster_name = sqlx::query_scalar!(
        "select full_name from roster_students where course_id = $1 and lower(matric_number) = lower($2)",
        session.course_id,
        input.matric_number.trim()
    )
    .fetch_optional(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to query roster student for join");
        ApiError::service_unavailable()
    })?;
    let status = if roster_name.is_some() {
        VerificationStatus::Verified
    } else {
        VerificationStatus::Provisional
    };
    let name = roster_name
        .or(input.display_name)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "Guest student".to_owned());
    let participant_id = Uuid::now_v7();
    let participant = sqlx::query_as!(
        SessionParticipant,
        r#"insert into session_participants (id, session_id, matric_number, display_name, verification_status)
         values ($1, $2, $3, $4, $5)
         on conflict (session_id, matric_number) do update set last_seen_at = now()
         returning id, session_id, matric_number, display_name, verification_status as "verification_status: VerificationStatus", joined_at, last_seen_at, heartbeat_count"#,
        participant_id,
        session.id,
        input.matric_number.trim(),
        name,
        status as VerificationStatus
    )
    .fetch_one(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to insert/update session participant on join");
        ApiError::service_unavailable()
    })?;
    sqlx::query!(
        "insert into attendance_events (participant_id, event_type) values ($1, 'joined')",
        participant.id
    )
    .execute(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to insert attendance event 'joined'");
        ApiError::service_unavailable()
    })?;
    Ok((StatusCode::CREATED, Json(participant)))
}

pub async fn list_for_session(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(short_code): Path<String>,
) -> Result<Json<Vec<SessionParticipant>>, ApiError> {
    let pool = state.db_pool();
    let session = owned_session(pool, &short_code, lecturer.id).await?;
    let participants = sqlx::query_as!(
        SessionParticipant,
        r#"select id, session_id, matric_number, display_name, verification_status as "verification_status: VerificationStatus", joined_at, last_seen_at, heartbeat_count
         from session_participants where session_id = $1 order by joined_at"#,
        session.id
    )
    .fetch_all(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to list session participants");
        ApiError::service_unavailable()
    })?;
    Ok(Json(participants))
}

pub async fn heartbeat(
    State(state): State<AppState>,
    Path(participant_id): Path<Uuid>,
) -> Result<Json<SessionParticipant>, ApiError> {
    let pool = state.db_pool();
    let participant = sqlx::query_as!(
        SessionParticipant,
        r#"update session_participants p set last_seen_at = now(), heartbeat_count = heartbeat_count + 1
         from lecture_sessions s where p.id = $1 and p.session_id = s.id and s.status = 'live'
         returning p.id, p.session_id, p.matric_number, p.display_name, p.verification_status as "verification_status: VerificationStatus", p.joined_at, p.last_seen_at, p.heartbeat_count"#,
        participant_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to record participant heartbeat");
        ApiError::service_unavailable()
    })?
    .ok_or_else(|| ApiError::conflict("Participant record not found or session is no longer active"))?;
    sqlx::query!(
        "insert into attendance_events (participant_id, event_type) values ($1, 'heartbeat')",
        participant.id
    )
    .execute(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to record heartbeat attendance event");
        ApiError::service_unavailable()
    })?;
    Ok(Json(participant))
}

pub async fn attendance_summary(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(short_code): Path<String>,
) -> Result<Json<AttendanceSummary>, ApiError> {
    let pool = state.db_pool();
    let session = owned_session(pool, &short_code, lecturer.id).await?;
    let row = sqlx::query!(
        r#"select count(*) as "participant_count!",
                  count(*) filter (where verification_status = 'verified') as "verified_count!",
                  count(*) filter (where verification_status = 'provisional') as "provisional_count!",
                  coalesce(sum(heartbeat_count), 0)::bigint as "total_heartbeats!"
           from session_participants where session_id = $1"#,
        session.id
    )
    .fetch_one(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to query attendance summary");
        ApiError::service_unavailable()
    })?;
    Ok(Json(AttendanceSummary {
        session_id: session.id,
        participant_count: row.participant_count as usize,
        verified_count: row.verified_count as usize,
        provisional_count: row.provisional_count as usize,
        total_heartbeats: row.total_heartbeats.max(0) as u64,
    }))
}

async fn owned_session(
    pool: &sqlx::PgPool,
    code: &str,
    lecturer_id: Uuid,
) -> Result<crate::models::LectureSession, ApiError> {
    let session = database_session_by_code(pool, code).await?;
    let owns_session = sqlx::query_scalar!(
        r#"select exists(select 1 from lecture_sessions where id = $1 and lecturer_id = $2) as "exists!""#,
        session.id,
        lecturer_id
    )
    .fetch_one(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to verify session ownership");
        ApiError::service_unavailable()
    })?;
    if !owns_session {
        return Err(ApiError::not_found("Session not found"));
    }
    Ok(session)
}
