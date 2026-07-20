use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::{
    api::error::ApiError,
    models::{Course, CreateCourseRequest, RosterStudent, UploadRosterRequest},
    state::AppState,
};

pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<CreateCourseRequest>,
) -> Result<(StatusCode, Json<Course>), ApiError> {
    let mut store = state.store.lock().await;
    if !store.lecturers.contains_key(&input.lecturer_id) {
        return Err(ApiError::bad_request("lecturer_not_found"));
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

pub async fn list(State(state): State<AppState>) -> Json<Vec<Course>> {
    Json(state.store.lock().await.courses.values().cloned().collect())
}

pub async fn upload_roster(
    State(state): State<AppState>,
    Path(course_id): Path<Uuid>,
    Json(input): Json<UploadRosterRequest>,
) -> Result<Json<Vec<RosterStudent>>, ApiError> {
    let mut store = state.store.lock().await;
    if !store.courses.contains_key(&course_id) {
        return Err(ApiError::not_found("course_not_found"));
    }
    store.rosters.insert(course_id, input.students.clone());
    Ok(Json(input.students))
}
