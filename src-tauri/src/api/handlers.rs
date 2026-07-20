use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    api::error::ApiError,
    models::*,
    state::AppState,
    utils::short_code,
};

pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "service": "klasync-api", "status": "healthy" }))
}

pub async fn register_lecturer(
    State(state): State<AppState>,
    Json(input): Json<RegisterLecturerRequest>,
) -> Json<Lecturer> {
    let lecturer = Lecturer {
        id: Uuid::new_v4(),
        name: input.name,
        email: input.email,
        created_at: Utc::now(),
    };
    state
        .store
        .lock()
        .await
        .lecturers
        .insert(lecturer.id, lecturer.clone());
    Json(lecturer)
}

pub async fn create_course(
    State(state): State<AppState>,
    Json(input): Json<CreateCourseRequest>,
) -> Result<(StatusCode, Json<Course>), ApiError> {
    let mut store = state.store.lock().await;
    if !store.lecturers.contains_key(&input.lecturer_id) {
        return Err(ApiError {
            error: "lecturer_not_found",
        });
    }
    let course = Course {
        id: Uuid::new_v4(),
        lecturer_id: input.lecturer_id,
        code: input.code,
        title: input.title,
    };
    store.courses.insert(course.id, course.clone());
    Ok((StatusCode::CREATED, Json(course)))
}

pub async fn list_courses(State(state): State<AppState>) -> Json<Vec<Course>> {
    Json(state.store.lock().await.courses.values().cloned().collect())
}

pub async fn upload_roster(
    State(state): State<AppState>,
    Path(course_id): Path<Uuid>,
    Json(input): Json<UploadRosterRequest>,
) -> Result<Json<Vec<RosterStudent>>, ApiError> {
    let mut store = state.store.lock().await;
    if !store.courses.contains_key(&course_id) {
        return Err(ApiError {
            error: "course_not_found",
        });
    }
    store.rosters.insert(course_id, input.students.clone());
    Ok(Json(input.students))
}

pub async fn create_session(
    State(state): State<AppState>,
    Json(input): Json<CreateSessionRequest>,
) -> Result<(StatusCode, Json<InviteResponse>), ApiError> {
    let mut store = state.store.lock().await;
    if !store.courses.contains_key(&input.course_id) {
        return Err(ApiError {
            error: "course_not_found",
        });
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
    let join_url = format!("http://localhost:5173/join/{}", session.invite_token);
    let response = InviteResponse {
        qr_payload: join_url.clone(),
        join_url,
        session: session.clone(),
    };
    store.sessions.insert(session.id, session);
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn get_session_by_code(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
) -> Result<Json<SessionDetail>, ApiError> {
    let store = state.store.lock().await;
    let session = store
        .sessions
        .values()
        .find(|item| item.short_code.eq_ignore_ascii_case(&short_code))
        .cloned()
        .ok_or(ApiError {
            error: "session_not_found",
        })?;
    let course = store
        .courses
        .get(&session.course_id)
        .cloned()
        .ok_or(ApiError {
            error: "course_not_found",
        })?;
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

pub async fn join_session(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
    Json(input): Json<JoinSessionRequest>,
) -> Result<(StatusCode, Json<SessionParticipant>), ApiError> {
    let mut store = state.store.lock().await;
    let session = store
        .sessions
        .values()
        .find(|item| item.short_code.eq_ignore_ascii_case(&short_code))
        .cloned()
        .ok_or(ApiError {
            error: "session_not_found",
        })?;
    if !matches!(session.status, SessionStatus::Live) {
        return Err(ApiError {
            error: "session_not_live",
        });
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

pub async fn list_session_participants(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
) -> Result<Json<Vec<SessionParticipant>>, ApiError> {
    let store = state.store.lock().await;
    let session = store
        .sessions
        .values()
        .find(|item| item.short_code.eq_ignore_ascii_case(&short_code))
        .ok_or(ApiError {
            error: "session_not_found",
        })?;
    let mut participants: Vec<_> = store
        .participants
        .values()
        .filter(|participant| participant.session_id == session.id)
        .cloned()
        .collect();
    participants.sort_by_key(|participant| participant.joined_at);
    Ok(Json(participants))
}

pub async fn end_session(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
) -> Result<Json<LectureSession>, ApiError> {
    let mut store = state.store.lock().await;
    let session = store
        .sessions
        .values_mut()
        .find(|item| item.short_code.eq_ignore_ascii_case(&short_code))
        .ok_or(ApiError {
            error: "session_not_found",
        })?;
    session.status = SessionStatus::Ended;
    Ok(Json(session.clone()))
}

pub async fn list_captions(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
) -> Result<Json<Vec<CaptionChunk>>, ApiError> {
    let store = state.store.lock().await;
    let session = store
        .sessions
        .values()
        .find(|item| item.short_code.eq_ignore_ascii_case(&short_code))
        .ok_or(ApiError {
            error: "session_not_found",
        })?;
    Ok(Json(
        store.captions.get(&session.id).cloned().unwrap_or_default(),
    ))
}

pub async fn publish_caption(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
    Json(input): Json<PublishCaptionRequest>,
) -> Result<(StatusCode, Json<CaptionChunk>), ApiError> {
    let mut store = state.store.lock().await;
    let session = store
        .sessions
        .values()
        .find(|item| item.short_code.eq_ignore_ascii_case(&short_code))
        .cloned()
        .ok_or(ApiError {
            error: "session_not_found",
        })?;
    if !matches!(session.status, SessionStatus::Live) {
        return Err(ApiError {
            error: "session_not_live",
        });
    }
    let text = input.text.trim();
    if text.is_empty() {
        return Err(ApiError {
            error: "caption_text_required",
        });
    }
    let caption = CaptionChunk {
        id: Uuid::new_v4(),
        session_id: session.id,
        text: text.to_owned(),
        created_at: Utc::now(),
    };
    store
        .captions
        .entry(session.id)
        .or_default()
        .push(caption.clone());
    Ok((StatusCode::CREATED, Json(caption)))
}

pub async fn heartbeat(
    State(state): State<AppState>,
    Path(participant_id): Path<Uuid>,
) -> Result<Json<SessionParticipant>, ApiError> {
    let mut store = state.store.lock().await;
    let participant = store
        .participants
        .get_mut(&participant_id)
        .ok_or(ApiError {
            error: "participant_not_found",
        })?;
    participant.last_seen_at = Utc::now();
    participant.heartbeat_count += 1;
    Ok(Json(participant.clone()))
}
