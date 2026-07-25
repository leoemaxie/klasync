use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::{
    api::error::ApiError,
    auth::guard::AuthenticatedLecturer,
    models::{Course, CreateCourseRequest, RosterStudent, UploadRosterRequest},
    state::AppState,
};

pub async fn create(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Json(input): Json<CreateCourseRequest>,
) -> Result<(StatusCode, Json<Course>), ApiError> {
    if let Some(pool) = state.production_database() {
        let course = sqlx::query_as::<_, Course>(
            "insert into courses (lecturer_id, code, title) values ($1, $2, $3) returning id, lecturer_id, code, title",
        )
        .bind(lecturer.id)
        .bind(input.code.trim())
        .bind(input.title.trim())
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::conflict("course_creation_failed"))?;
        return Ok((StatusCode::CREATED, Json(course)));
    }
    let mut store = state.store.lock().await;
    let course = Course {
        id: Uuid::now_v7(),
        lecturer_id: lecturer.id,
        code: input.code,
        title: input.title,
    };
    store.courses.insert(course.id, course.clone());
    Ok((StatusCode::CREATED, Json(course)))
}

pub async fn list(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
) -> Result<Json<Vec<Course>>, ApiError> {
    if let Some(pool) = state.production_database() {
        let courses = sqlx::query_as::<_, Course>(
            "select id, lecturer_id, code, title from courses where lecturer_id = $1 order by created_at desc",
        )
        .bind(lecturer.id)
        .fetch_all(pool)
        .await
        .map_err(|_| ApiError::service_unavailable("course_lookup_failed"))?;
        return Ok(Json(courses));
    }
    let courses = state
        .store
        .lock()
        .await
        .courses
        .values()
        .filter(|course| course.lecturer_id == lecturer.id)
        .cloned()
        .collect();
    Ok(Json(courses))
}

pub async fn upload_roster(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(course_id): Path<Uuid>,
    Json(input): Json<UploadRosterRequest>,
) -> Result<Json<Vec<RosterStudent>>, ApiError> {
    if let Some(pool) = state.production_database() {
        let mut transaction = pool
            .begin()
            .await
            .map_err(|_| ApiError::service_unavailable("roster_transaction_failed"))?;
        let owns_course: bool = sqlx::query_scalar(
            "select exists(select 1 from courses where id = $1 and lecturer_id = $2)",
        )
        .bind(course_id)
        .bind(lecturer.id)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| ApiError::service_unavailable("course_lookup_failed"))?;
        if !owns_course {
            return Err(ApiError::not_found("course_not_found"));
        }
        sqlx::query("delete from roster_students where course_id = $1")
            .bind(course_id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| ApiError::service_unavailable("roster_replace_failed"))?;
        for student in &input.students {
            sqlx::query(
                "insert into roster_students (course_id, matric_number, full_name, email) values ($1, $2, $3, $4)",
            )
            .bind(course_id)
            .bind(student.matric_number.trim())
            .bind(student.full_name.trim())
            .bind(student.email.as_deref())
            .execute(&mut *transaction)
            .await
            .map_err(|_| ApiError::conflict("invalid_or_duplicate_roster_record"))?;
        }
        transaction
            .commit()
            .await
            .map_err(|_| ApiError::service_unavailable("roster_commit_failed"))?;
        return Ok(Json(input.students));
    }
    let mut store = state.store.lock().await;
    if !store
        .courses
        .get(&course_id)
        .is_some_and(|course| course.lecturer_id == lecturer.id)
    {
        return Err(ApiError::not_found("course_not_found"));
    }
    store.rosters.insert(course_id, input.students.clone());
    Ok(Json(input.students))
}
