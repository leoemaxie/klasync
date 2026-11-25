use axum::{
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::{
    api::error::{ApiError, LogApiError},
    auth::guard::AuthenticatedStudent,
    state::AppState,
    student_sync,
};

#[derive(Debug, Serialize)]
pub struct StudyJobResponse {
    pub job_id: Uuid,
    pub status: &'static str,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct GenerateInput {
    /// Optional focus topic ("Key formulas"). A topic always creates new cards.
    pub topic: Option<String>,
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

#[derive(Debug, Serialize, FromRow)]
pub struct JobStatus {
    pub id: Uuid,
    pub job_type: String,
    pub status: String,
    pub error_message: Option<String>,
    pub attempts: i32,
}

#[derive(Debug, Serialize, FromRow)]
pub struct SessionVisualization {
    pub id: Uuid,
    pub kind: String,
    pub title: String,
    pub spec: serde_json::Value,
    pub source: String,
    pub created_at: DateTime<Utc>,
}

pub(crate) async fn require_access(
    pool: &PgPool,
    student: &AuthenticatedStudent,
    session_id: Uuid,
) -> Result<(), ApiError> {
    let mut allowed = student_sync::student_can_access_session(pool, student.id, session_id)
        .await
        .log_internal_error("Failed to check student access to session")?;
    if !allowed {
        // The student may have joined moments ago on another device.
        student_sync::reconcile_student_logged(pool, student.id).await;
        allowed = student_sync::student_can_access_session(pool, student.id, session_id)
            .await
            .log_internal_error("Failed to check student access to session")?;
    }
    if allowed {
        Ok(())
    } else {
        Err(ApiError::not_found("Session not found in your archive"))
    }
}

pub async fn generate_chapters(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Path(session_id): Path<Uuid>,
) -> Result<(StatusCode, Json<StudyJobResponse>), ApiError> {
    let pool = state.db_pool();
    require_access(pool, &student, session_id).await?;
    let has_existing = exists_for(pool, "session_chapters", session_id).await;
    if has_existing {
        return Ok(ready());
    }
    enqueue(
        &state,
        student.id,
        session_id,
        "chapters",
        serde_json::json!({}),
        true,
    )
    .await
}

pub async fn generate_flashcards(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Path(session_id): Path<Uuid>,
    body: Bytes,
) -> Result<(StatusCode, Json<StudyJobResponse>), ApiError> {
    let pool = state.db_pool();
    require_access(pool, &student, session_id).await?;
    let topic = topic_from(&body);
    if topic.is_none() && exists_for(pool, "session_flashcards", session_id).await {
        return Ok(ready());
    }
    let params = serde_json::json!({ "topic": topic });
    enqueue(
        &state,
        student.id,
        session_id,
        "flashcards",
        params,
        topic.is_none(),
    )
    .await
}

pub async fn generate_visualization(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Path(session_id): Path<Uuid>,
    body: Bytes,
) -> Result<(StatusCode, Json<StudyJobResponse>), ApiError> {
    let pool = state.db_pool();
    require_access(pool, &student, session_id).await?;
    let topic = topic_from(&body);
    let has_existing = exists_for(pool, "session_visualizations", session_id).await;
    if topic.is_none() && has_existing {
        return Ok(ready());
    }
    let params = serde_json::json!({ "topic": topic });
    enqueue(
        &state,
        student.id,
        session_id,
        "visualization",
        params,
        topic.is_none(),
    )
    .await
}

/// Lenient body parsing: an empty body, `{}` or a missing content type all mean
/// "no topic".
fn topic_from(body: &Bytes) -> Option<String> {
    serde_json::from_slice::<GenerateInput>(body)
        .ok()
        .and_then(|input| input.topic)
        .map(|t| t.trim().to_owned())
        .filter(|t| !t.is_empty())
}

fn ready() -> (StatusCode, Json<StudyJobResponse>) {
    (
        StatusCode::OK,
        Json(StudyJobResponse {
            job_id: Uuid::nil(),
            status: "ready",
        }),
    )
}

async fn exists_for(pool: &PgPool, table: &'static str, session_id: Uuid) -> bool {
    // `table` is always one of our own constants.
    let sql = format!("select exists(select 1 from {table} where session_id = $1)");
    sqlx::query_scalar::<_, bool>(&sql)
        .bind(session_id)
        .fetch_one(pool)
        .await
        .unwrap_or(false)
}

pub async fn chapters(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Path(session_id): Path<Uuid>,
) -> Result<Json<Vec<SessionChapter>>, ApiError> {
    let pool = state.db_pool();
    require_access(pool, &student, session_id).await?;
    let rows = sqlx::query_as::<_, SessionChapter>(
        "select id, chapter_index, title, summary, start_timestamp_sec, end_timestamp_sec, created_at
         from session_chapters where session_id = $1 order by chapter_index",
    )
    .bind(session_id)
    .fetch_all(pool)
    .await
    .log_internal_error("Failed to query session chapters")?;
    Ok(Json(rows))
}

pub async fn flashcards(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Path(session_id): Path<Uuid>,
) -> Result<Json<Vec<SessionFlashcard>>, ApiError> {
    let pool = state.db_pool();
    require_access(pool, &student, session_id).await?;
    let rows = sqlx::query_as::<_, SessionFlashcard>(
        "select id, prompt, answer, topic_tag, difficulty, created_at
         from session_flashcards where session_id = $1 order by created_at, id",
    )
    .bind(session_id)
    .fetch_all(pool)
    .await
    .log_internal_error("Failed to query session flashcards")?;
    Ok(Json(rows))
}

pub async fn visualizations(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Path(session_id): Path<Uuid>,
) -> Result<Json<Vec<SessionVisualization>>, ApiError> {
    let pool = state.db_pool();
    require_access(pool, &student, session_id).await?;
    let rows = sqlx::query_as::<_, SessionVisualization>(
        "select id, kind, title, spec, source, created_at
         from session_visualizations where session_id = $1 order by created_at desc",
    )
    .bind(session_id)
    .fetch_all(pool)
    .await
    .log_internal_error("Failed to query session visualizations")?;
    Ok(Json(rows))
}

/// Polled by the studio while a generation job is running so the UI can show
/// progress and, crucially, the real failure reason.
pub async fn job_status(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Path(job_id): Path<Uuid>,
) -> Result<Json<JobStatus>, ApiError> {
    let pool = state.db_pool();
    let job = sqlx::query_as::<_, JobStatus>(
        "select j.id, j.job_type, j.status::text as status, j.error_message, j.attempts
         from ai_jobs j where j.id = $1",
    )
    .bind(job_id)
    .fetch_optional(pool)
    .await
    .log_internal_error("Failed to query AI job status")?
    .ok_or_else(|| ApiError::not_found("AI job not found"))?;
    let session_id = sqlx::query_scalar::<_, Uuid>("select session_id from ai_jobs where id = $1")
        .bind(job_id)
        .fetch_one(pool)
        .await
        .log_internal_error("Failed to query AI job session")?;
    require_access(pool, &student, session_id).await?;
    Ok(Json(job))
}

/// Queues a job, re-using an in-flight one, and revives a recently failed job
/// when the student asks again so "Generate" actually retries.
async fn enqueue(
    state: &AppState,
    requester_id: Uuid,
    session_id: Uuid,
    job_type: &str,
    params: serde_json::Value,
    reuse_inflight: bool,
) -> Result<(StatusCode, Json<StudyJobResponse>), ApiError> {
    let pool = state.db_pool();

    if reuse_inflight {
        // A batch job that has not been submitted yet is promoted to run now:
        // someone is waiting for it.
        let promoted = sqlx::query_scalar::<_, Uuid>(
            "update ai_jobs set mode = 'interactive', run_after = now()
             where id = (select id from ai_jobs where session_id = $1 and job_type = $2
                         and mode = 'batch' and status = 'queued'
                         order by created_at desc limit 1)
             returning id",
        )
        .bind(session_id)
        .bind(job_type)
        .fetch_optional(pool)
        .await
        .log_internal_error("Failed to promote a queued batch job")?;
        if let Some(job_id) = promoted {
            return Ok((
                StatusCode::ACCEPTED,
                Json(StudyJobResponse {
                    job_id,
                    status: "processing",
                }),
            ));
        }
        // Only interactive jobs are reused: a job waiting inside a provider
        // batch can take hours, so the student gets their own on-demand run.
        let existing = sqlx::query_scalar::<_, Uuid>(
            "select id from ai_jobs where session_id = $1 and job_type = $2
               and status in ('queued', 'running') and mode = 'interactive'
               and created_at > now() - interval '15 minutes'
             order by created_at desc limit 1",
        )
        .bind(session_id)
        .bind(job_type)
        .fetch_optional(pool)
        .await
        .log_internal_error("Failed to look for an in-flight AI job")?;
        if let Some(job_id) = existing {
            return Ok((
                StatusCode::ACCEPTED,
                Json(StudyJobResponse {
                    job_id,
                    status: "processing",
                }),
            ));
        }
    }

    let input_resource = sqlx::query_scalar::<_, Uuid>(
        "select id from lecture_resources
         where session_id = $1 and resource_type = 'transcript'
         order by created_at desc limit 1",
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await
    .log_internal_error("Failed to query input transcript resource for AI job")?;

    let job_id = Uuid::now_v7();
    sqlx::query(
        "insert into ai_jobs (id, session_id, requested_by, job_type, input_resource_id, params)
         values ($1, $2, $3, $4, $5, $6)",
    )
    .bind(job_id)
    .bind(session_id)
    .bind(requester_id)
    .bind(job_type)
    .bind(input_resource)
    .bind(&params)
    .execute(pool)
    .await
    .log_internal_error("Failed to insert study AI job")?;

    if let Some(redis) = &state.redis {
        if let Err(error) = redis.enqueue_ai_job(&job_id.to_string()).await {
            // The database poller picks the job up either way.
            tracing::warn!(%error, "Managed Redis AI queue enqueue failed; database worker will poll");
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
