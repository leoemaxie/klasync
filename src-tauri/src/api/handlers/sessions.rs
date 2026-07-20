use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    api::error::ApiError,
    models::{CreateSessionRequest, InviteResponse, LectureSession, SessionDetail, SessionStatus},
    state::AppState,
    utils::short_code,
};

pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<CreateSessionRequest>,
) -> Result<(StatusCode, Json<InviteResponse>), ApiError> {
    let mut store = state.store.lock().await;
    if !store.courses.contains_key(&input.course_id) {
        return Err(ApiError::not_found("course_not_found"));
    }
    let session = LectureSession {
        id: Uuid::new_v4(),
        course_id: input.course_id,
        title: input.title,
        short_code: short_code(),
        invite_token: Uuid::new_v4(),
        status: SessionStatus::Live,
        started_at: Utc::now(),
    };
    let join_url = format!("/?join={}", session.short_code);
    let response = InviteResponse {
        qr_payload: join_url.clone(),
        join_url,
        session: session.clone(),
    };
    store.sessions.insert(session.id, session);
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn get_by_code(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
) -> Result<Json<SessionDetail>, ApiError> {
    let store = state.store.lock().await;
    let session = find_by_code(&store.sessions, &short_code)?.clone();
    let course = store
        .courses
        .get(&session.course_id)
        .cloned()
        .ok_or_else(|| ApiError::not_found("course_not_found"))?;
    let participant_count = store
        .participants
        .values()
        .filter(|participant| participant.session_id == session.id)
        .count();
    Ok(Json(SessionDetail {
        session,
        course,
        participant_count,
    }))
}

pub async fn end(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
) -> Result<Json<LectureSession>, ApiError> {
    let mut store = state.store.lock().await;
    let session = store
        .sessions
        .values_mut()
        .find(|item| item.short_code.eq_ignore_ascii_case(&short_code))
        .ok_or_else(|| ApiError::not_found("session_not_found"))?;
    session.status = SessionStatus::Ended;
    Ok(Json(session.clone()))
}

pub fn find_by_code<'a>(
    sessions: &'a std::collections::HashMap<Uuid, LectureSession>,
    short_code: &str,
) -> Result<&'a LectureSession, ApiError> {
    sessions
        .values()
        .find(|session| session.short_code.eq_ignore_ascii_case(short_code))
        .ok_or_else(|| ApiError::not_found("session_not_found"))
}
