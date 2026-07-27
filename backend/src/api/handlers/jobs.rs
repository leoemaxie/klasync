use axum::{extract::{Path, State}, http::StatusCode, Json};
use uuid::Uuid;

use crate::{
    api::{error::ApiError, handlers::sessions::database_session_by_code},
    auth::guard::AuthenticatedLecturer,
    models::{AiJob, CreateAiJobRequest},
    state::AppState,
};

const JOB_COLUMNS: &str = "id, session_id, job_type, status::text as status, input_resource_id, output_resource_id, error_message, attempts, created_at, started_at, completed_at";

pub async fn create(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(short_code): Path<String>,
    Json(input): Json<CreateAiJobRequest>,
) -> Result<(StatusCode, Json<AiJob>), ApiError> {
    if !["transcribe", "summarize", "flashcards", "lecture_qa_index", "explain", "question_answer"].contains(&input.job_type.as_str()) {
        return Err(ApiError::bad_request("Invalid AI job type specified"));
    }
    let pool = state.production_database().ok_or_else(|| ApiError::service_unavailable())?;
    let session = database_session_by_code(pool, &short_code).await?;
    let owns_session: bool = sqlx::query_scalar("select exists(select 1 from lecture_sessions where id = $1 and lecturer_id = $2)")
        .bind(session.id).bind(lecturer.id).fetch_one(pool).await
        .map_err(|_| ApiError::service_unavailable())?;
    if !owns_session { return Err(ApiError::not_found("Session not found")); }
    let job = sqlx::query_as::<_, AiJob>(&format!(
        "insert into ai_jobs (id, session_id, requested_by, job_type, input_resource_id) values ($1, $2, $3, $4, $5) returning {JOB_COLUMNS}"
    ))
    .bind(Uuid::now_v7())
    .bind(session.id)
    .bind(lecturer.id)
    .bind(input.job_type)
    .bind(input.input_resource_id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?;
    Ok((StatusCode::ACCEPTED, Json(job)))
}

pub async fn list(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(short_code): Path<String>,
) -> Result<Json<Vec<AiJob>>, ApiError> {
    let pool = state.production_database().ok_or_else(|| ApiError::service_unavailable())?;
    let session = database_session_by_code(pool, &short_code).await?;
    let jobs = sqlx::query_as::<_, AiJob>(&format!(
        "select {JOB_COLUMNS} from ai_jobs where session_id = $1 and requested_by = $2 order by created_at desc"
    ))
    .bind(session.id)
    .bind(lecturer.id)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?;
    Ok(Json(jobs))
}
