//! Durable execution for the `ai_jobs` queue.
//!
//! The HTTP endpoint is intentionally lecturer-triggered for now. The same
//! `process_job` function can be called by a background worker later without
//! duplicating ownership, retry, storage, or adapter logic.

use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    ai::AiWorkItem,
    api::error::ApiError,
    auth::guard::AuthenticatedLecturer,
    state::AppState,
};

const MAX_ATTEMPTS: i32 = 3;

#[derive(Debug, FromRow)]
struct PendingJob {
    id: Uuid,
    session_id: Uuid,
    job_type: String,
    input_resource_id: Option<Uuid>,
    attempts: i32,
}

#[derive(Debug, FromRow)]
struct InputResource {
    resource_type: String,
    storage_key: Option<String>,
    content: Option<String>,
    content_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExecutionResponse {
    pub job_id: Uuid,
    pub status: &'static str,
    pub output_resource_id: Option<Uuid>,
    pub attempts: i32,
}

pub async fn process_job(
    state: &AppState,
    job_id: Uuid,
    lecturer_id: Uuid,
) -> Result<ExecutionResponse, ApiError> {
    if let Some(redis) = &state.redis {
        let lock = redis.try_lock("ai-job", &job_id.to_string(), 300).await
            .map_err(|_| ApiError::service_unavailable())?
            .ok_or_else(|| ApiError::conflict("This AI job is already being processed."))?;
        let result = process_job_unlocked(state, job_id, lecturer_id).await;
        let _ = lock.release().await;
        return result;
    }
    process_job_unlocked(state, job_id, lecturer_id).await
}

async fn process_job_unlocked(
    state: &AppState,
    job_id: Uuid,
    lecturer_id: Uuid,
) -> Result<ExecutionResponse, ApiError> {
    let pool = state
        .production_database()
        .ok_or_else(|| ApiError::service_unavailable())?;

    let job = sqlx::query_as::<_, PendingJob>(
        "update ai_jobs set status = 'running', started_at = now(), attempts = attempts + 1, error_message = null
         where id = $1 and requested_by = $2 and status in ('queued', 'failed') and attempts < $3
         returning id, session_id, job_type, input_resource_id, attempts",
    )
    .bind(job_id)
    .bind(lecturer_id)
    .bind(MAX_ATTEMPTS)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?
    .ok_or_else(|| ApiError::conflict("AI job is not available for processing"))?;

    let result = execute_claimed_job(state, pool, &job).await;
    match result {
        Ok(output_resource_id) => {
            sqlx::query(
                "update ai_jobs set status = 'completed', output_resource_id = $2, completed_at = now(), error_message = null where id = $1",
            )
            .bind(job.id)
            .bind(output_resource_id)
            .execute(pool)
            .await
            .map_err(|_| ApiError::service_unavailable())?;
            Ok(ExecutionResponse {
                job_id: job.id,
                status: "completed",
                output_resource_id: Some(output_resource_id),
                attempts: job.attempts,
            })
        }
        Err(error) => {
            let message = error.to_string();
            sqlx::query("update ai_jobs set status = 'failed', error_message = $2 where id = $1")
                .bind(job.id)
                .bind(&message)
                .execute(pool)
                .await
                .map_err(|_| ApiError::service_unavailable())?;
            Err(error)
        }
    }
}

async fn execute_claimed_job(
    state: &AppState,
    pool: &sqlx::PgPool,
    job: &PendingJob,
) -> Result<Uuid, ApiError> {
    let resource = match job.input_resource_id {
        Some(resource_id) => sqlx::query_as::<_, InputResource>(
            "select resource_type, storage_key, content, content_type from lecture_resources where id = $1 and session_id = $2",
        )
        .bind(resource_id)
        .bind(job.session_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::service_unavailable())?
        .ok_or_else(|| ApiError::not_found("Resource for AI job input not found"))?,
        None => InputResource {
            resource_type: "text".to_owned(),
            storage_key: None,
            content: Some(String::new()),
            content_type: None,
        },
    };

    let input = if let Some(key) = resource.storage_key.as_deref() {
        let bytes = state
            .storage
            .get(key)
            .await
            .map_err(|_| ApiError::service_unavailable())?;
        if job.job_type == "transcribe" {
            let format = resource
                .content_type
                .as_deref()
                .and_then(|value| value.split('/').nth(1))
                .unwrap_or("wav");
            crate::ai::transcription_input(&bytes, format, Some("en"))
        } else {
            serde_json::json!({ "text": String::from_utf8_lossy(&bytes).to_string() })
        }
    } else {
        serde_json::json!({ "text": resource.content.unwrap_or_default() })
    };

    let work = AiWorkItem {
        job_id: job.id,
        session_id: job.session_id,
        job_type: job.job_type.clone(),
        input,
    };
    let output = state
        .ai
        .execute(work)
        .await
        .map_err(|_| ApiError::service_unavailable())?;
    let usage = output.metadata.get("usage");
    let model = output.metadata.get("model").and_then(|value| value.as_str());
    let input_tokens = usage.and_then(|value| value.get("input_tokens")).and_then(|value| value.as_i64());
    let output_tokens = usage.and_then(|value| value.get("output_tokens")).and_then(|value| value.as_i64());
    let cost_usd = usage.and_then(|value| value.get("cost")).and_then(|value| value.as_f64());
    sqlx::query(
        "update ai_jobs set provider = $2, model = $3, input_tokens = $4, output_tokens = $5, cost_usd = $6 where id = $1",
    )
    .bind(job.id)
    .bind(state.ai.provider_name())
    .bind(model)
    .bind(input_tokens)
    .bind(output_tokens)
    .bind(cost_usd)
    .execute(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?;
    let content = serde_json::to_string(&output.content)
        .map_err(|_| ApiError::service_unavailable())?;
    let resource_type = output_type(&job.job_type);
    let output_id = Uuid::now_v7();
    sqlx::query(
        "insert into lecture_resources (id, session_id, resource_type, content, checksum) values ($1, $2, $3, $4, $5)",
    )
    .bind(output_id)
    .bind(job.session_id)
    .bind(resource_type)
    .bind(content)
    .bind(output.metadata.to_string())
    .execute(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?;
    Ok(output_id)
}

/// Polls the durable queue so a deployment can process jobs without requiring
/// a lecturer to dispatch each one manually. Claiming remains atomic inside
/// `process_job`, so multiple instances may run this loop safely.
pub async fn run_loop(state: AppState) {
    let mut ticker = tokio::time::interval(std::time::Duration::from_secs(5));
    let consumer = format!("worker-{}", Uuid::now_v7());
    let mut redis_group_ready = false;
    loop {
        ticker.tick().await;
        if let Some(redis) = &state.redis {
            if !redis_group_ready {
                match redis.ensure_ai_consumer_group().await {
                    Ok(()) => redis_group_ready = true,
                    Err(error) => tracing::warn!(%error, "Unable to initialize AI Redis consumer group"),
                }
            }
            if redis_group_ready {
                match redis.read_ai_job(&consumer).await {
                    Ok(Some((message_id, job_id))) => {
                        if let Ok(job_id) = Uuid::parse_str(&job_id) {
                            let pool = state.production_database();
                            if let Some(pool) = pool {
                                if let Ok(Some(lecturer_id)) = sqlx::query_scalar::<_, Uuid>("select requested_by from ai_jobs where id = $1").bind(job_id).fetch_optional(pool).await {
                                    if let Err(error) = process_job(&state, job_id, lecturer_id).await {
                                        tracing::warn!(%job_id, error = %error, "AI Redis stream job failed");
                                    }
                                }
                            }
                        }
                        let _ = redis.acknowledge_ai_job(&message_id).await;
                        continue;
                    }
                    Ok(None) => {}
                    Err(error) => tracing::warn!(%error, "Unable to read AI Redis stream; using database queue"),
                }
            }
        }
        let Some(pool) = state.production_database() else { continue; };
        let candidate = sqlx::query_as::<_, (Uuid, Uuid)>(
            "select id, requested_by from ai_jobs where status = 'queued' and attempts < $1 order by created_at asc limit 1",
        )
        .bind(MAX_ATTEMPTS)
        .fetch_optional(pool)
        .await;
        let Ok(Some((job_id, lecturer_id))) = candidate else { continue; };
        if let Err(error) = process_job(&state, job_id, lecturer_id).await {
            tracing::warn!(%job_id, error = %error, "AI job processing failed");
        }
    }
}

fn output_type(job_type: &str) -> &'static str {
    match job_type {
        "transcribe" => "transcript",
        "summarize" => "summary",
        "flashcards" => "flashcards",
        "lecture_qa_index" => "notes",
        _ => "notes",
    }
}

pub async fn dispatch(
    state: axum::extract::State<AppState>,
    lecturer: AuthenticatedLecturer,
    path: axum::extract::Path<(String, Uuid)>,
) -> Result<axum::Json<ExecutionResponse>, ApiError> {
    let (short_code, job_id) = path.0;
    let pool = state
        .production_database()
        .ok_or_else(|| ApiError::service_unavailable())?;
    let owns_job: bool = sqlx::query_scalar(
        "select exists(select 1 from ai_jobs j join lecture_sessions s on s.id = j.session_id where j.id = $1 and j.requested_by = $2 and s.short_code = upper($3) and s.lecturer_id = $2)",
    )
    .bind(job_id)
    .bind(lecturer.id)
    .bind(short_code)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?;
    if !owns_job {
        return Err(ApiError::not_found("AI job not found"));
    }
    Ok(axum::Json(process_job(&state, job_id, lecturer.id).await?))
}
