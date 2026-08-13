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
    let pool = state.db_pool();
    let course = sqlx::query_as::<_, Course>(
        "insert into courses (lecturer_id, code, title) values ($1, $2, $3) returning id, lecturer_id, code, title",
    )
    .bind(lecturer.id)
    .bind(input.code.trim())
    .bind(input.title.trim())
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::conflict("Course code already exists for this lecturer"))?;
    Ok((StatusCode::CREATED, Json(course)))
}

pub async fn list(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
) -> Result<Json<Vec<Course>>, ApiError> {
    let pool = state.db_pool();
    let courses = sqlx::query_as::<_, Course>(
        "select id, lecturer_id, code, title from courses where lecturer_id = $1 order by created_at desc",
    )
    .bind(lecturer.id)
    .fetch_all(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to list lecturer courses");
        ApiError::service_unavailable()
    })?;
    Ok(Json(courses))
}

pub async fn upload_roster(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(course_id): Path<Uuid>,
    Json(input): Json<UploadRosterRequest>,
) -> Result<Json<Vec<RosterStudent>>, ApiError> {
    let pool = state.db_pool();
    let mut transaction = pool.begin().await.map_err(|error| {
        tracing::error!(%error, "Failed to start transaction for roster upload");
        ApiError::service_unavailable()
    })?;
    let owns_course: bool = sqlx::query_scalar(
        "select exists(select 1 from courses where id = $1 and lecturer_id = $2)",
    )
    .bind(course_id)
    .bind(lecturer.id)
    .fetch_one(&mut *transaction)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to check course ownership in upload_roster");
        ApiError::service_unavailable()
    })?;
    if !owns_course {
        return Err(ApiError::not_found("Course not found"));
    }
    sqlx::query("delete from roster_students where course_id = $1")
        .bind(course_id)
        .execute(&mut *transaction)
        .await
        .map_err(|error| {
            tracing::error!(%error, "Failed to delete old roster students");
            ApiError::service_unavailable()
        })?;
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
        .map_err(|_| ApiError::conflict("Roster contains duplicate or invalid student records"))?;
    }
    transaction.commit().await.map_err(|error| {
        tracing::error!(%error, "Failed to commit roster upload transaction");
        ApiError::service_unavailable()
    })?;
    Ok(Json(input.students))
}

pub async fn get_roster(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(course_id): Path<Uuid>,
) -> Result<Json<Vec<RosterStudent>>, ApiError> {
    let pool = state.db_pool();
    let owns_course: bool = sqlx::query_scalar(
        "select exists(select 1 from courses where id = $1 and lecturer_id = $2)",
    )
    .bind(course_id)
    .bind(lecturer.id)
    .fetch_one(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to check course ownership in get_roster");
        ApiError::service_unavailable()
    })?;

    if !owns_course {
        return Err(ApiError::not_found("Course not found"));
    }

    let students = sqlx::query_as::<_, RosterStudent>(
        "select matric_number, full_name, email from roster_students where course_id = $1 order by full_name asc",
    )
    .bind(course_id)
    .fetch_all(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to query roster students");
        ApiError::service_unavailable()
    })?;

    Ok(Json(students))
}
