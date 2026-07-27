use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    api::{
        error::ApiError,
        handlers::sessions::{database_session_by_code, find_by_code},
    },
    auth::guard::AuthenticatedLecturer,
    models::{
        AttendanceSummary, JoinSessionRequest, SessionParticipant, SessionStatus,
        VerificationStatus,
    },
    state::AppState,
};

const PARTICIPANT_COLUMNS: &str = "id, session_id, matric_number, display_name, verification_status, joined_at, last_seen_at, heartbeat_count";

pub async fn join(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
    Json(input): Json<JoinSessionRequest>,
) -> Result<(StatusCode, Json<SessionParticipant>), ApiError> {
    if let Some(pool) = state.production_database() {
        let session = database_session_by_code(pool, &short_code).await?;
        if !matches!(session.status, SessionStatus::Live) {
            return Err(ApiError::conflict("This lecture session is not currently active"));
        }
        let roster_name: Option<String> = sqlx::query_scalar(
            "select full_name from roster_students where course_id = $1 and lower(matric_number) = lower($2)",
        )
        .bind(session.course_id)
        .bind(input.matric_number.trim())
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::service_unavailable())?;
        let status = if roster_name.is_some() {
            VerificationStatus::Verified
        } else {
            VerificationStatus::Provisional
        };
        let name = roster_name
            .or(input.display_name)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "Guest student".to_owned());
        let participant = sqlx::query_as::<_, SessionParticipant>(&format!(
            "insert into session_participants (id, session_id, matric_number, display_name, verification_status) \
             values ($1, $2, $3, $4, $5) \
             on conflict (session_id, matric_number) do update set last_seen_at = now() \
             returning {PARTICIPANT_COLUMNS}"
        ))
        .bind(Uuid::now_v7())
        .bind(session.id)
        .bind(input.matric_number.trim())
        .bind(name)
        .bind(status)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::service_unavailable())?;
        sqlx::query(
            "insert into attendance_events (participant_id, event_type) values ($1, 'joined')",
        )
        .bind(participant.id)
        .execute(pool)
        .await
        .map_err(|_| ApiError::service_unavailable())?;
        return Ok((StatusCode::CREATED, Json(participant)));
    }

    let mut store = state.store.lock().await;
    let session = find_by_code(&store.sessions, &short_code)?.clone();
    if !matches!(session.status, SessionStatus::Live) {
        return Err(ApiError::conflict("This lecture session is not currently active"));
    }
    if let Some(existing) = store
        .participants
        .values()
        .find(|participant| {
            participant.session_id == session.id
                && participant
                    .matric_number
                    .eq_ignore_ascii_case(&input.matric_number)
        })
        .cloned()
    {
        return Ok((StatusCode::OK, Json(existing)));
    }
    let roster_match = store.rosters.get(&session.course_id).and_then(|roster| {
        roster.iter().find(|student| {
            student
                .matric_number
                .eq_ignore_ascii_case(&input.matric_number)
        })
    });
    let now = Utc::now();
    let participant = SessionParticipant {
        id: Uuid::now_v7(),
        session_id: session.id,
        matric_number: input.matric_number,
        display_name: roster_match
            .map(|student| student.full_name.clone())
            .or(input.display_name)
            .unwrap_or_else(|| "Guest student".to_owned()),
        verification_status: if roster_match.is_some() {
            VerificationStatus::Verified
        } else {
            VerificationStatus::Provisional
        },
        joined_at: now,
        last_seen_at: now,
        heartbeat_count: 0,
    };
    store
        .participants
        .insert(participant.id, participant.clone());
    Ok((StatusCode::CREATED, Json(participant)))
}

pub async fn list_for_session(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(short_code): Path<String>,
) -> Result<Json<Vec<SessionParticipant>>, ApiError> {
    let pool = state
        .production_database()
        .ok_or_else(|| ApiError::service_unavailable())?;
    let session = owned_session(pool, &short_code, lecturer.id).await?;
    let participants = sqlx::query_as::<_, SessionParticipant>(&format!(
        "select {PARTICIPANT_COLUMNS} from session_participants where session_id = $1 order by joined_at"
    ))
    .bind(session.id)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?;
    Ok(Json(participants))
}

pub async fn heartbeat(
    State(state): State<AppState>,
    Path(participant_id): Path<Uuid>,
) -> Result<Json<SessionParticipant>, ApiError> {
    if let Some(pool) = state.production_database() {
        let participant = sqlx::query_as::<_, SessionParticipant>(&format!(
            "update session_participants p set last_seen_at = now(), heartbeat_count = heartbeat_count + 1 \
             from lecture_sessions s where p.id = $1 and p.session_id = s.id and s.status = 'live' \
             returning p.{PARTICIPANT_COLUMNS}"
        ))
        .bind(participant_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::service_unavailable())?
        .ok_or_else(|| ApiError::conflict("Participant record not found or session is no longer active"))?;
        sqlx::query(
            "insert into attendance_events (participant_id, event_type) values ($1, 'heartbeat')",
        )
        .bind(participant.id)
        .execute(pool)
        .await
        .map_err(|_| ApiError::service_unavailable())?;
        return Ok(Json(participant));
    }
    let mut store = state.store.lock().await;
    let participant = store
        .participants
        .get_mut(&participant_id)
        .ok_or_else(|| ApiError::not_found("Participant not found"))?;
    participant.last_seen_at = Utc::now();
    participant.heartbeat_count += 1;
    Ok(Json(participant.clone()))
}

pub async fn attendance_summary(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(short_code): Path<String>,
) -> Result<Json<AttendanceSummary>, ApiError> {
    let pool = state
        .production_database()
        .ok_or_else(|| ApiError::service_unavailable())?;
    let session = owned_session(pool, &short_code, lecturer.id).await?;
    let (participant_count, verified_count, provisional_count, total_heartbeats): (i64, i64, i64, i64) = sqlx::query_as(
        "select count(*), count(*) filter (where verification_status = 'verified'), count(*) filter (where verification_status = 'provisional'), coalesce(sum(heartbeat_count), 0) from session_participants where session_id = $1",
    )
    .bind(session.id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?;
    Ok(Json(AttendanceSummary {
        session_id: session.id,
        participant_count: participant_count as usize,
        verified_count: verified_count as usize,
        provisional_count: provisional_count as usize,
        total_heartbeats: total_heartbeats as u32,
    }))
}

async fn owned_session(
    pool: &sqlx::PgPool,
    code: &str,
    lecturer_id: Uuid,
) -> Result<crate::models::LectureSession, ApiError> {
    let session = database_session_by_code(pool, code).await?;
    let owns_session: bool = sqlx::query_scalar(
        "select exists(select 1 from lecture_sessions where id = $1 and lecturer_id = $2)",
    )
    .bind(session.id)
    .bind(lecturer_id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?;
    if !owns_session {
        return Err(ApiError::not_found("Session not found"));
    }
    Ok(session)
}
