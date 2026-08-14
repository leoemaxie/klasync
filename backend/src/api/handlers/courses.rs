use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::{
    api::error::ApiError,
    auth::guard::{AuthenticatedLecturer, AuthenticatedStudent},
    models::{
        Course, CourseFilterQuery, CreateCourseRequest, EnrollCourseRequest, RosterStudent,
        StudentEnrolledCourse, UploadRosterRequest,
    },
    state::AppState,
};

pub async fn create(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Json(input): Json<CreateCourseRequest>,
) -> Result<(StatusCode, Json<Course>), ApiError> {
    let pool = state.db_pool();
    let code = input.code.trim();
    let title = input.title.trim();
    let academic_session = input.academic_session.trim();
    let semester = input.semester.trim();

    if code.is_empty() {
        return Err(ApiError::bad_request("Course code cannot be empty"));
    }
    if title.is_empty() {
        return Err(ApiError::bad_request("Course title cannot be empty"));
    }
    if academic_session.is_empty() {
        return Err(ApiError::bad_request(
            "Academic session cannot be empty (e.g. 2025/2026)",
        ));
    }
    if semester.is_empty() {
        return Err(ApiError::bad_request(
            "Academic semester is required (e.g. Second Semester)",
        ));
    }

    let course_id = Uuid::now_v7();
    let course = sqlx::query_as!(
        Course,
        r#"insert into courses (id, lecturer_id, code, title, academic_session, semester, is_active)
           values ($1, $2, $3, $4, $5, $6, true)
           returning id, lecturer_id, code, title, academic_session, semester, is_active"#,
        course_id,
        lecturer.id,
        code,
        title,
        academic_session,
        semester
    )
    .fetch_one(pool)
    .await
    .map_err(|error| {
        tracing::warn!(%error, "Course offering conflict on create");
        ApiError::conflict(
            "Course offering already exists for this lecturer, session, and semester",
        )
    })?;

    Ok((StatusCode::CREATED, Json(course)))
}

pub async fn list(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Query(filters): Query<CourseFilterQuery>,
) -> Result<Json<Vec<Course>>, ApiError> {
    let pool = state.db_pool();
    let courses = sqlx::query_as!(
        Course,
        r#"select id, lecturer_id, code, title, academic_session, semester, is_active
           from courses
           where lecturer_id = $1
             and ($2::text is null or academic_session = $2)
             and ($3::text is null or semester = $3)
             and ($4::bool is null or is_active = $4)
           order by created_at desc"#,
        lecturer.id,
        filters.academic_session,
        filters.semester,
        filters.is_active
    )
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

    let owns_course = sqlx::query_scalar!(
        r#"select exists(select 1 from courses where id = $1 and lecturer_id = $2) as "exists!""#,
        course_id,
        lecturer.id
    )
    .fetch_one(&mut *transaction)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to check course ownership in upload_roster");
        ApiError::service_unavailable()
    })?;

    if !owns_course {
        return Err(ApiError::not_found("Course not found"));
    }

    sqlx::query!(
        "delete from roster_students where course_id = $1",
        course_id
    )
    .execute(&mut *transaction)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to delete old roster students");
        ApiError::service_unavailable()
    })?;

    for student in &input.students {
        let student_id = Uuid::now_v7();
        sqlx::query!(
            "insert into roster_students (id, course_id, matric_number, full_name, email) values ($1, $2, $3, $4, $5)",
            student_id,
            course_id,
            student.matric_number.trim(),
            student.full_name.trim(),
            student.email.as_deref()
        )
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
    let owns_course = sqlx::query_scalar!(
        r#"select exists(select 1 from courses where id = $1 and lecturer_id = $2) as "exists!""#,
        course_id,
        lecturer.id
    )
    .fetch_one(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to check course ownership in get_roster");
        ApiError::service_unavailable()
    })?;

    if !owns_course {
        return Err(ApiError::not_found("Course not found"));
    }

    let students = sqlx::query_as!(
        RosterStudent,
        "select matric_number, full_name, email from roster_students where course_id = $1 order by full_name asc",
        course_id
    )
    .fetch_all(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to query roster students");
        ApiError::service_unavailable()
    })?;

    Ok(Json(students))
}

pub async fn student_courses(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
) -> Result<Json<Vec<StudentEnrolledCourse>>, ApiError> {
    let pool = state.db_pool();
    let courses = sqlx::query_as!(
        StudentEnrolledCourse,
        r#"select 
            c.id,
            c.code,
            c.title,
            c.academic_session,
            c.semester,
            l.name as lecturer_name,
            count(distinct s.id) as "session_count!",
            e.enrolled_at
           from student_course_enrollments e
           join courses c on c.id = e.course_id
           join lecturers l on l.id = c.lecturer_id
           left join lecture_sessions s on s.course_id = c.id
           where e.student_account_id = $1
           group by c.id, c.code, c.title, c.academic_session, c.semester, l.name, e.enrolled_at
           order by c.academic_session desc, c.semester asc, c.code asc"#,
        student.id
    )
    .fetch_all(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to query student enrolled courses");
        ApiError::service_unavailable()
    })?;

    Ok(Json(courses))
}

pub async fn enroll_course(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Json(input): Json<EnrollCourseRequest>,
) -> Result<StatusCode, ApiError> {
    let pool = state.db_pool();
    let exists = sqlx::query_scalar!(
        r#"select exists(select 1 from courses where id = $1) as "exists!""#,
        input.course_id
    )
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?;

    if !exists {
        return Err(ApiError::not_found("Course offering not found"));
    }

    let enrollment_id = Uuid::now_v7();
    sqlx::query!(
        r#"insert into student_course_enrollments (id, student_account_id, course_id, enrollment_type)
           values ($1, $2, $3, 'direct')
           on conflict (student_account_id, course_id) do nothing"#,
        enrollment_id,
        student.id,
        input.course_id
    )
    .execute(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to enroll student in course");
        ApiError::service_unavailable()
    })?;

    Ok(StatusCode::CREATED)
}
