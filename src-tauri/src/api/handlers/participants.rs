use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    api::{error::ApiError, handlers::sessions::find_by_code},
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
    let mut store = state.store.lock().await;
    let session = find_by_code(&store.sessions, &short_code)?.clone();
    if !matches!(session.status, SessionStatus::Live) {
        return Err(ApiError::conflict("session_not_live"));
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
    let verification_status = if roster_match.is_some() {
        VerificationStatus::Verified
    } else {
        VerificationStatus::Provisional
    };
    let display_name = roster_match
        .map(|student| student.full_name.clone())
        .or(input.display_name)
        .unwrap_or_else(|| "Guest student".to_owned());
    let now = Utc::now();
    let participant = SessionParticipant {
        id: Uuid::new_v4(),
        session_id: session.id,
        matric_number: input.matric_number,
        display_name,
        verification_status,
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
    Path(short_code): Path<String>,
) -> Result<Json<Vec<SessionParticipant>>, ApiError> {
    let store = state.store.lock().await;
    let session = find_by_code(&store.sessions, &short_code)?;
    let mut participants: Vec<_> = store
        .participants
        .values()
        .filter(|participant| participant.session_id == session.id)
        .cloned()
        .collect();
    participants.sort_by_key(|participant| participant.joined_at);
    Ok(Json(participants))
}

pub async fn heartbeat(
    State(state): State<AppState>,
    Path(participant_id): Path<Uuid>,
) -> Result<Json<SessionParticipant>, ApiError> {
    let mut store = state.store.lock().await;
    let existing = store
        .participants
        .get(&participant_id)
        .cloned()
        .ok_or_else(|| ApiError::not_found("participant_not_found"))?;
    let session = store
        .sessions
        .get(&existing.session_id)
        .ok_or_else(|| ApiError::not_found("session_not_found"))?;
    if !matches!(session.status, SessionStatus::Live) {
        return Err(ApiError::conflict("session_not_live"));
    }
    let participant = store
        .participants
        .get_mut(&participant_id)
        .ok_or_else(|| ApiError::not_found("participant_not_found"))?;
    participant.last_seen_at = Utc::now();
    participant.heartbeat_count += 1;
    Ok(Json(participant.clone()))
}

pub async fn attendance_summary(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
) -> Result<Json<AttendanceSummary>, ApiError> {
    let store = state.store.lock().await;
    let session = find_by_code(&store.sessions, &short_code)?;
    let participants = store
        .participants
        .values()
        .filter(|participant| participant.session_id == session.id);
    let mut summary = AttendanceSummary {
        session_id: session.id,
        participant_count: 0,
        verified_count: 0,
        provisional_count: 0,
        total_heartbeats: 0,
    };
    for participant in participants {
        summary.participant_count += 1;
        summary.total_heartbeats += participant.heartbeat_count;
        match &participant.verification_status {
            VerificationStatus::Verified => summary.verified_count += 1,
            VerificationStatus::Provisional => summary.provisional_count += 1,
        }
    }
    Ok(Json(summary))
}
