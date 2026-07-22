use axum::{extract::{Multipart, Path, State}, http::StatusCode, Json};
use uuid::Uuid;

use crate::{
    api::{error::ApiError, handlers::sessions::database_session_by_code},
    auth::guard::AuthenticatedLecturer,
    models::LectureResource,
    state::AppState,
};

const RESOURCE_COLUMNS: &str = "id, session_id, resource_type, storage_key, content, checksum, created_at, expires_at";

pub async fn upload(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path((short_code, resource_type)): Path<(String, String)>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<LectureResource>), ApiError> {
    if !["audio", "recording", "transcript", "notes"].contains(&resource_type.as_str()) {
        return Err(ApiError::bad_request("invalid_upload_resource_type"));
    }
    let file = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::bad_request("invalid_multipart_upload"))?
        .ok_or_else(|| ApiError::bad_request("upload_file_required"))?;
    let file_name = file.file_name().unwrap_or("upload.bin").to_owned();
    let content_type = file.content_type().map(ToOwned::to_owned);
    let bytes = file.bytes().await.map_err(|_| ApiError::bad_request("upload_file_read_failed"))?;
    let pool = state.production_database().ok_or_else(|| ApiError::service_unavailable("database_not_configured"))?;
    let session = database_session_by_code(pool, &short_code).await?;
    let owns_session: bool = sqlx::query_scalar("select exists(select 1 from lecture_sessions where id = $1 and lecturer_id = $2)")
        .bind(session.id).bind(lecturer.id).fetch_one(pool).await
        .map_err(|_| ApiError::service_unavailable("session_lookup_failed"))?;
    if !owns_session { return Err(ApiError::not_found("session_not_found")); }
    let stored = state.storage.put(&file_name, bytes.to_vec()).await
        .map_err(|_| ApiError::service_unavailable("object_storage_write_failed"))?;
    let resource = sqlx::query_as::<_, LectureResource>(&format!(
        "insert into lecture_resources (id, session_id, resource_type, storage_key, original_filename, content_type, byte_size) \
         values ($1, $2, $3, $4, $5, $6, $7) returning {RESOURCE_COLUMNS}"
    ))
    .bind(Uuid::new_v4())
    .bind(session.id)
    .bind(resource_type)
    .bind(stored.key)
    .bind(file_name)
    .bind(content_type)
    .bind(stored.bytes as i64)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::service_unavailable("resource_persistence_failed"))?;
    Ok((StatusCode::CREATED, Json(resource)))
}
