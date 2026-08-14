use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    api::error::{ApiError, LogApiError},
    auth::guard::OptionalStudent,
    state::AppState,
};

#[derive(Debug, Serialize)]
pub struct StudyJobResponse {
    pub job_id: Uuid,
    pub status: &'static str,
}

#[derive(Debug, Serialize, FromRow)]
pub struct SessionChapter {
    pub id: Uuid,
    pub chapter_index: i32,
    pub title: String,
    pub summary: String,
    pub start_timestamp_sec: i32,
    pub end_timestamp_sec: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct SessionFlashcard {
    pub id: Uuid,
    pub prompt: String,
    pub answer: String,
    pub topic_tag: Option<String>,
    pub difficulty: String,
    pub created_at: DateTime<Utc>,
}

pub async fn generate_chapters(
    State(state): State<AppState>,
    OptionalStudent(student): OptionalStudent,
    Path(session_id): Path<Uuid>,
) -> Result<(StatusCode, Json<StudyJobResponse>), ApiError> {
    let requester_id = student.map(|s| s.id).unwrap_or_else(Uuid::now_v7);
    enqueue(&state, requester_id, session_id, "chapters").await
}

pub async fn generate_flashcards(
    State(state): State<AppState>,
    OptionalStudent(student): OptionalStudent,
    Path(session_id): Path<Uuid>,
) -> Result<(StatusCode, Json<StudyJobResponse>), ApiError> {
    let requester_id = student.map(|s| s.id).unwrap_or_else(Uuid::now_v7);
    enqueue(&state, requester_id, session_id, "flashcards").await
}

pub async fn chapters(
    State(state): State<AppState>,
    OptionalStudent(_student): OptionalStudent,
    Path(session_id): Path<Uuid>,
) -> Result<Json<Vec<SessionChapter>>, ApiError> {
    let pool = state.db_pool();
    let rows = sqlx::query_as!(
        SessionChapter,
        "select id, chapter_index, title, summary, start_timestamp_sec, end_timestamp_sec, created_at from session_chapters where session_id = $1 order by chapter_index",
        session_id
    )
    .fetch_all(pool)
    .await
    .log_internal_error("Failed to query session chapters")?;
    Ok(Json(rows))
}

pub async fn flashcards(
    State(state): State<AppState>,
    OptionalStudent(_student): OptionalStudent,
    Path(session_id): Path<Uuid>,
) -> Result<Json<Vec<SessionFlashcard>>, ApiError> {
    let pool = state.db_pool();
    let rows = sqlx::query_as!(
        SessionFlashcard,
        "select id, prompt, answer, topic_tag, difficulty, created_at from session_flashcards where session_id = $1 order by created_at",
        session_id
    )
    .fetch_all(pool)
    .await
    .log_internal_error("Failed to query session flashcards")?;
    Ok(Json(rows))
}

async fn enqueue(
    state: &AppState,
    requester_id: Uuid,
    session_id: Uuid,
    job_type: &str,
) -> Result<(StatusCode, Json<StudyJobResponse>), ApiError> {
    let pool = state.db_pool();
    let input_resource = sqlx::query_scalar!(
        "select id from lecture_resources where session_id = $1 and resource_type = 'transcript' order by created_at desc limit 1",
        session_id
    )
    .fetch_optional(pool)
    .await
    .log_internal_error("Failed to query input transcript resource for AI job")?;
    let job_id = Uuid::now_v7();
    sqlx::query!(
        "insert into ai_jobs (id, session_id, requested_by, job_type, input_resource_id) values ($1, $2, $3, $4, $5)",
        job_id,
        session_id,
        requester_id,
        job_type,
        input_resource
    )
    .execute(pool)
    .await
    .log_internal_error("Failed to insert study AI job")?;
    if let Some(redis) = &state.redis {
        if let Err(error) = redis.enqueue_ai_job(&job_id.to_string()).await {
            if state.config.redis_required {
                tracing::error!(%error, "Managed Redis AI queue enqueue failed");
                return Err(ApiError::service_unavailable());
            }
            tracing::warn!(%error, "Managed Redis AI queue enqueue failed");
        }
    }
    Ok((
        StatusCode::ACCEPTED,
        Json(StudyJobResponse {
            job_id,
            status: "processing",
        }),
    ))
}
