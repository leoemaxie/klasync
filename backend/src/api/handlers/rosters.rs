use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::{
    api::error::{ApiError, LogApiError},
    auth::guard::AuthenticatedLecturer,
    models::RosterImportReport,
    roster_file,
    state::AppState,
};

pub async fn import_file(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(course_id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<RosterImportReport>), ApiError> {
    let file = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::bad_request("Invalid upload format"))?
        .ok_or_else(|| ApiError::bad_request("Roster file is required"))?;
    let file_name = file
        .file_name()
        .map(ToOwned::to_owned)
        .ok_or_else(|| ApiError::bad_request("Roster file name is required"))?;
    let bytes = file
        .bytes()
        .await
        .map_err(|_| ApiError::bad_request("Failed to read roster file"))?;
    let parsed = roster_file::parse_async(file_name, bytes.to_vec())
        .await
        .map_err(|_| ApiError::bad_request("Invalid or unsupported roster file format"))?;

    let report = RosterImportReport {
        imported_count: parsed.students.len(),
        issues: parsed.issues,
    };
    if !report.issues.is_empty() {
        return Ok((StatusCode::UNPROCESSABLE_ENTITY, Json(report)));
    }

    let pool = state.db_pool();
    let mut transaction = pool
        .begin()
        .await
        .log_internal_error("Failed to start transaction for roster import")?;
    let owns_course: bool = sqlx::query_scalar(
        "select exists(select 1 from courses where id = $1 and lecturer_id = $2)",
    )
    .bind(course_id)
    .bind(lecturer.id)
    .fetch_one(&mut *transaction)
    .await
    .log_internal_error("Failed to verify course ownership in import_file")?;
    if !owns_course {
        return Err(ApiError::not_found("Course not found"));
    }
    sqlx::query("delete from roster_students where course_id = $1")
        .bind(course_id)
        .execute(&mut *transaction)
        .await
        .log_internal_error("Failed to delete previous roster students")?;
    for student in parsed.students {
        sqlx::query(
            "insert into roster_students (course_id, matric_number, full_name, email) values ($1, $2, $3, $4)",
        )
        .bind(course_id)
        .bind(student.matric_number)
        .bind(student.full_name)
        .bind(student.email)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::conflict("Roster contains duplicate or invalid student records"))?;
    }
    transaction
        .commit()
        .await
        .log_internal_error("Failed to commit roster import transaction")?;
    Ok((StatusCode::OK, Json(report)))
}
