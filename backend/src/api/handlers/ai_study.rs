use axum::{extract::{Path, State}, http::StatusCode, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{api::error::ApiError, auth::guard::AuthenticatedLecturer, state::AppState};

#[derive(Debug, Serialize)]
pub struct StudyJobResponse { pub job_id: Uuid, pub status: &'static str }

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
    State(state): State<AppState>, lecturer: AuthenticatedLecturer, Path(session_id): Path<Uuid>,
) -> Result<(StatusCode, Json<StudyJobResponse>), ApiError> {
    enqueue(&state, lecturer.id, session_id, "chapters").await
}

pub async fn generate_flashcards(
    State(state): State<AppState>, lecturer: AuthenticatedLecturer, Path(session_id): Path<Uuid>,
) -> Result<(StatusCode, Json<StudyJobResponse>), ApiError> {
    enqueue(&state, lecturer.id, session_id, "flashcards").await
}

pub async fn chapters(
    State(state): State<AppState>, lecturer: AuthenticatedLecturer, Path(session_id): Path<Uuid>,
) -> Result<Json<Vec<SessionChapter>>, ApiError> {
    let pool = owned_session(&state, lecturer.id, session_id).await?;
    let rows = sqlx::query_as::<_, SessionChapter>("select id, chapter_index, title, summary, start_timestamp_sec, end_timestamp_sec, created_at from session_chapters where session_id = $1 order by chapter_index")
        .bind(session_id).fetch_all(pool).await.map_err(|_| ApiError::service_unavailable())?;
    Ok(Json(rows))
}

pub async fn flashcards(
    State(state): State<AppState>, lecturer: AuthenticatedLecturer, Path(session_id): Path<Uuid>,
) -> Result<Json<Vec<SessionFlashcard>>, ApiError> {
    let pool = owned_session(&state, lecturer.id, session_id).await?;
    let rows = sqlx::query_as::<_, SessionFlashcard>("select id, prompt, answer, topic_tag, difficulty, created_at from session_flashcards where session_id = $1 order by created_at")
        .bind(session_id).fetch_all(pool).await.map_err(|_| ApiError::service_unavailable())?;
    Ok(Json(rows))
}

async fn enqueue(
    state: &AppState, lecturer_id: Uuid, session_id: Uuid, job_type: &str,
) -> Result<(StatusCode, Json<StudyJobResponse>), ApiError> {
    let pool = owned_session(state, lecturer_id, session_id).await?;
    let input_resource: Option<Uuid> = sqlx::query_scalar("select id from lecture_resources where session_id = $1 and resource_type = 'transcript' order by created_at desc limit 1")
        .bind(session_id).fetch_optional(pool).await.map_err(|_| ApiError::service_unavailable())?;
    let job_id = Uuid::now_v7();
    sqlx::query("insert into ai_jobs (id, session_id, requested_by, job_type, input_resource_id) values ($1, $2, $3, $4, $5)")
        .bind(job_id).bind(session_id).bind(lecturer_id).bind(job_type).bind(input_resource)
        .execute(pool).await.map_err(|_| ApiError::service_unavailable())?;
    if let Some(redis) = &state.redis {
        if let Err(error) = redis.enqueue_ai_job(&job_id.to_string()).await {
            if state.config.redis_required { return Err(ApiError::service_unavailable()); }
            tracing::warn!(%error, "Managed Redis AI queue unavailable; database worker will poll");
        }
    }
    Ok((StatusCode::ACCEPTED, Json(StudyJobResponse { job_id, status: "processing" })))
}

async fn owned_session(state: &AppState, lecturer_id: Uuid, session_id: Uuid) -> Result<&sqlx::PgPool, ApiError> {
    let pool = state.production_database().ok_or_else(|| ApiError::service_unavailable())?;
    let owns: bool = sqlx::query_scalar("select exists(select 1 from lecture_sessions where id = $1 and lecturer_id = $2 and deleted_at is null)")
        .bind(session_id).bind(lecturer_id).fetch_one(pool).await.map_err(|_| ApiError::service_unavailable())?;
    if !owns { return Err(ApiError::not_found("Session not found.")); }
    Ok(pool)
}
