use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::{
    api::{error::ApiError, handlers::sessions::database_session_by_code},
    auth::guard::{AuthenticatedLecturer, AuthenticatedStudent},
    models::{CreateLectureResourceRequest, LectureResource},
    state::AppState,
};

const RESOURCE_COLUMNS: &str =
    "id, session_id, resource_type, storage_key, content, checksum, created_at, expires_at";
const RESOURCE_SELECT_COLUMNS: &str = "resource.id, resource.session_id, resource.resource_type, resource.storage_key, resource.content, resource.checksum, resource.created_at, resource.expires_at";

pub async fn create_for_session(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(short_code): Path<String>,
    Json(input): Json<CreateLectureResourceRequest>,
) -> Result<(StatusCode, Json<LectureResource>), ApiError> {
    validate_resource(&input)?;
    let pool = state
        .production_database()
        .ok_or_else(|| ApiError::service_unavailable("database_not_configured"))?;
    let session = database_session_by_code(pool, &short_code).await?;
    let owns_session: bool = sqlx::query_scalar(
        "select exists(select 1 from lecture_sessions where id = $1 and lecturer_id = $2)",
    )
    .bind(session.id)
    .bind(lecturer.id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::service_unavailable("session_lookup_failed"))?;
    if !owns_session {
        return Err(ApiError::not_found("session_not_found"));
    }
    let resource = sqlx::query_as::<_, LectureResource>(&format!(
        "insert into lecture_resources (id, session_id, resource_type, storage_key, content, checksum, expires_at) \
         values ($1, $2, $3, $4, $5, $6, $7) returning {RESOURCE_COLUMNS}"
    ))
    .bind(Uuid::new_v4())
    .bind(session.id)
    .bind(input.resource_type)
    .bind(input.storage_key)
    .bind(input.content)
    .bind(input.checksum)
    .bind(input.expires_at)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::service_unavailable("resource_persistence_failed"))?;
    Ok((StatusCode::CREATED, Json(resource)))
}

pub async fn list_student_archive(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
) -> Result<Json<Vec<LectureResource>>, ApiError> {
    let pool = state
        .production_database()
        .ok_or_else(|| ApiError::service_unavailable("database_not_configured"))?;
    let resources = sqlx::query_as::<_, LectureResource>(&format!(
        "select distinct {RESOURCE_SELECT_COLUMNS} from lecture_resources resource \
         join resource_access_grants grant on grant.resource_id = resource.id \
         where grant.student_account_id = $1 and (resource.expires_at is null or resource.expires_at > now()) \
         order by resource.created_at desc"
    ))
    .bind(student.id)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::service_unavailable("archive_lookup_failed"))?;
    Ok(Json(resources))
}

fn validate_resource(input: &CreateLectureResourceRequest) -> Result<(), ApiError> {
    let allowed = [
        "audio",
        "recording",
        "transcript",
        "summary",
        "flashcards",
        "notes",
    ];
    if !allowed.contains(&input.resource_type.as_str()) {
        return Err(ApiError::bad_request("invalid_resource_type"));
    }
    if input.storage_key.is_none() && input.content.is_none() {
        return Err(ApiError::bad_request("resource_content_required"));
    }
    Ok(())
}
