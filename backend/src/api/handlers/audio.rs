use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    api::{error::ApiError, handlers::sessions::database_session_by_code},
    auth::guard::AuthenticatedLecturer,
    state::AppState,
};

#[derive(Debug, Serialize)]
pub struct AudioIngestionResponse {
    pub resource_id: Uuid,
    pub ai_job_id: Uuid,
    pub status: &'static str,
}

pub async fn upload(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(short_code): Path<String>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<AudioIngestionResponse>), ApiError> {
    let pool = state.db_pool();
    let session = database_session_by_code(pool, &short_code).await?;
    let owns_session = sqlx::query_scalar!(
        r#"select exists(select 1 from lecture_sessions where id = $1 and lecturer_id = $2) as "exists!""#,
        session.id,
        lecturer.id
    )
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?;
    if !owns_session {
        return Err(ApiError::not_found("Session not found"));
    }

    let field = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::bad_request("Invalid upload payload"))?
        .ok_or_else(|| ApiError::bad_request("Audio file is required"))?;
    let file_name = field.file_name().unwrap_or("lecture-audio.wav").to_owned();
    let content_type = field.content_type().unwrap_or("audio/wav").to_owned();
    if !content_type.starts_with("audio/") {
        return Err(ApiError::bad_request("Uploaded file must be an audio file"));
    }
    let bytes = field
        .bytes()
        .await
        .map_err(|_| ApiError::bad_request("Failed to read uploaded audio file"))?;
    if bytes.is_empty() {
        return Err(ApiError::bad_request("Audio file cannot be empty"));
    }
    if bytes.len() > 50 * 1024 * 1024 {
        return Err(ApiError::bad_request(
            "Audio file exceeds maximum allowed size (50MB)",
        ));
    }

    let stored = state
        .storage
        .put(&file_name, bytes.to_vec())
        .await
        .map_err(|error| {
            tracing::error!(%error, "Audio file storage put failed");
            ApiError::service_unavailable()
        })?;
    let resource_id = Uuid::new_v4();
    sqlx::query!(
        "insert into lecture_resources (id, session_id, resource_type, storage_key, original_filename, content_type, byte_size) values ($1, $2, 'audio', $3, $4, $5, $6)",
        resource_id,
        session.id,
        stored.key,
        file_name,
        content_type,
        stored.bytes as i64
    )
    .execute(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to insert audio lecture resource into database");
        ApiError::service_unavailable()
    })?;

    let ai_job_id = Uuid::now_v7();
    sqlx::query!(
        "insert into ai_jobs (id, session_id, requested_by, job_type, input_resource_id) values ($1, $2, $3, 'transcribe', $4)",
        ai_job_id,
        session.id,
        lecturer.id,
        resource_id
    )
    .execute(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to insert transcribe AI job into database");
        ApiError::service_unavailable()
    })?;

    Ok((
        StatusCode::ACCEPTED,
        Json(AudioIngestionResponse {
            resource_id,
            ai_job_id,
            status: "queued",
        }),
    ))
}
