use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::{
    api::error::ApiError, auth::guard::AuthenticatedLecturer, models::RosterImportReport,
    roster_file, state::AppState,
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
        .map_err(|_| ApiError::bad_request("invalid_multipart_upload"))?
        .ok_or_else(|| ApiError::bad_request("roster_file_required"))?;
    let file_name = file
        .file_name()
        .map(ToOwned::to_owned)
        .ok_or_else(|| ApiError::bad_request("roster_file_name_required"))?;
    let bytes = file
        .bytes()
        .await
        .map_err(|_| ApiError::bad_request("roster_file_read_failed"))?;
    let parsed = roster_file::parse(&file_name, &bytes)
        .map_err(|_| ApiError::bad_request("invalid_roster_file"))?;
    let report = RosterImportReport {
        imported_count: parsed.students.len(),
        issues: parsed.issues,
    };
    if !report.issues.is_empty() {
        return Ok((StatusCode::UNPROCESSABLE_ENTITY, Json(report)));
    }

    let pool = state
        .production_database()
        .ok_or_else(|| ApiError::service_unavailable("database_not_configured"))?;
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
        .map_err(|_| ApiError::conflict("invalid_or_duplicate_roster_record"))?;
    }
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::service_unavailable("roster_commit_failed"))?;
    Ok((StatusCode::OK, Json(report)))
}
