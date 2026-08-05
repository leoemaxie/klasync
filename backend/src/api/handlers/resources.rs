use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use sqlx::FromRow;
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
        .ok_or_else(|| ApiError::service_unavailable())?;
    let session = database_session_by_code(pool, &short_code).await?;
    let owns_session: bool = sqlx::query_scalar(
        "select exists(select 1 from lecture_sessions where id = $1 and lecturer_id = $2)",
    )
    .bind(session.id)
    .bind(lecturer.id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?;
    if !owns_session {
        return Err(ApiError::not_found("Session not found"));
    }
    let resource = sqlx::query_as::<_, LectureResource>(&format!(
        "insert into lecture_resources (id, session_id, resource_type, storage_key, content, checksum, expires_at) \
         values ($1, $2, $3, $4, $5, $6, $7) returning {RESOURCE_COLUMNS}"
    ))
    .bind(Uuid::now_v7())
    .bind(session.id)
    .bind(input.resource_type)
    .bind(input.storage_key)
    .bind(input.content)
    .bind(input.checksum)
    .bind(input.expires_at)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?;
    Ok((StatusCode::CREATED, Json(resource)))
}

pub async fn list_public_resources(
    State(state): State<AppState>,
) -> Result<Json<Vec<LectureResource>>, ApiError> {
    let pool = match state.production_database() {
        Some(p) => p,
        None => return Ok(Json(vec![])),
    };
    let resources = sqlx::query_as::<_, LectureResource>(&format!(
        "select {RESOURCE_SELECT_COLUMNS} from lecture_resources resource \
         where (resource.expires_at is null or resource.expires_at > now()) \
         order by resource.created_at desc limit 50"
    ))
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    Ok(Json(resources))
}

pub async fn list_student_archive(
    State(state): State<AppState>,
    student: Option<AuthenticatedStudent>,
) -> Result<Json<Vec<LectureResource>>, ApiError> {
    let pool = match state.production_database() {
        Some(p) => p,
        None => return Ok(Json(vec![])),
    };
    let student_id = match student {
        Some(s) => s.id,
        None => return Ok(Json(vec![])),
    };
    let resources = sqlx::query_as::<_, LectureResource>(&format!(
        "select distinct {RESOURCE_SELECT_COLUMNS} from lecture_resources resource \
         join resource_access_grants grant on grant.resource_id = resource.id \
         where grant.student_account_id = $1 and (resource.expires_at is null or resource.expires_at > now()) \
         order by resource.created_at desc"
    ))
    .bind(student_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
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
        return Err(ApiError::bad_request("Invalid resource type"));
    }
    if input.storage_key.is_none() && input.content.is_none() {
        return Err(ApiError::bad_request("Either storage key or resource content must be provided"));
    }
    Ok(())
}

#[derive(Debug, FromRow)]
struct DownloadableResource {
    storage_key: Option<String>,
    content: Option<String>,
    content_type: Option<String>,
    original_filename: Option<String>,
}

pub async fn download_for_student(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Path(resource_id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let pool = state.production_database().ok_or_else(|| ApiError::service_unavailable())?;
    let resource = sqlx::query_as::<_, DownloadableResource>(
        "select r.storage_key, r.content, r.content_type, r.original_filename
         from lecture_resources r join resource_access_grants g on g.resource_id = r.id
         where r.id = $1 and g.student_account_id = $2 and (r.expires_at is null or r.expires_at > now())",
    )
    .bind(resource_id)
    .bind(student.id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?
    .ok_or_else(|| ApiError::not_found("Resource not found"))?;
    response_from_resource(&state, resource).await
}

pub async fn download_for_lecturer(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path((short_code, resource_id)): Path<(String, Uuid)>,
) -> Result<Response, ApiError> {
    let pool = state.production_database().ok_or_else(|| ApiError::service_unavailable())?;
    let resource = sqlx::query_as::<_, DownloadableResource>(
        "select r.storage_key, r.content, r.content_type, r.original_filename
         from lecture_resources r join lecture_sessions s on s.id = r.session_id
         where r.id = $1 and s.short_code = upper($2) and s.lecturer_id = $3",
    )
    .bind(resource_id)
    .bind(short_code)
    .bind(lecturer.id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?
    .ok_or_else(|| ApiError::not_found("Resource not found"))?;
    response_from_resource(&state, resource).await
}

async fn response_from_resource(state: &AppState, resource: DownloadableResource) -> Result<Response, ApiError> {
    let bytes = if let Some(key) = resource.storage_key {
        state.storage.get(&key).await.map_err(|_| ApiError::service_unavailable())?
    } else {
        resource.content.unwrap_or_default().into_bytes()
    };
    let content_type = resource.content_type.unwrap_or_else(|| "application/octet-stream".to_owned());
    let filename = resource.original_filename.unwrap_or_else(|| "klasync-resource.bin".to_owned())
        .replace('\"', "_").replace('\r', "_").replace('\n', "_");
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_DISPOSITION, format!("attachment; filename=\"{filename}\""))
        .body(Body::from(bytes))
        .map_err(|_| ApiError::service_unavailable())
}
