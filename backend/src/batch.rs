//! Batch execution for after-lecture study material.
//!
//! Provider batch APIs run asynchronously at roughly half price, which suits
//! work nobody is waiting on. This coordinator claims `mode = 'batch'` jobs,
//! submits them as one batch per model, polls until they finish, and feeds the
//! results through the same persistence path as interactive jobs. Anything that
//! fails, expires or returns unusable output is re-queued as an interactive job,
//! so a slow or broken batch never leaves a lecture without study material.

use std::{collections::HashMap, time::Duration};

use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    activity,
    ai::{BatchItem, BatchState},
    ai_worker::{
        check_output, finish_text_job, lecture_text, mark_completed, mark_failed, prepare_text_job,
        PendingJob, MAX_ATTEMPTS,
    },
    state::AppState,
};

/// Past this age a still-running batch is abandoned and re-run interactively.
const MAX_BATCH_AGE_SECS: i64 = 6 * 3600;
const MAX_JOBS_PER_TICK: i64 = 40;

#[derive(Debug, FromRow)]
struct RunningBatch {
    id: Uuid,
    provider_batch_id: String,
    age_secs: i64,
}

async fn requeue_interactive(pool: &sqlx::PgPool, job_id: Uuid, reason: &str) {
    let _ = sqlx::query(
        "update ai_jobs set status = 'queued', mode = 'interactive', batch_id = null,
                error_message = $2, run_after = now()
         where id = $1 and status = 'running'",
    )
    .bind(job_id)
    .bind(reason.chars().take(300).collect::<String>())
    .execute(pool)
    .await;
}

pub async fn tick(state: &AppState) -> Result<(), String> {
    let pool = state.db_pool();
    poll_running(state).await?;
    submit_queued(state).await?;
    let pending: i64 =
        sqlx::query_scalar("select count(*) from ai_batches where status = 'running'")
            .fetch_one(pool)
            .await
            .map_err(|error| error.to_string())?;
    activity::set_pending_batches(pending.max(0) as usize);
    Ok(())
}

async fn submit_queued(state: &AppState) -> Result<(), String> {
    let pool = state.db_pool();
    let Some(provider) = state.ai.batch_provider() else {
        // No batch support (e.g. OpenRouter only): run these interactively.
        sqlx::query(
            "update ai_jobs set mode = 'interactive' where mode = 'batch' and status = 'queued'",
        )
        .execute(pool)
        .await
        .map_err(|error| error.to_string())?;
        return Ok(());
    };

    let jobs = sqlx::query_as::<_, PendingJob>(
        "update ai_jobs set status = 'running', started_at = now(), attempts = attempts + 1, error_message = null
         where id in (
           select id from ai_jobs
           where mode = 'batch' and status = 'queued' and run_after <= now() and attempts < $1
           order by created_at limit $2 for update skip locked)
         returning id, session_id, job_type, input_resource_id, attempts, params, requested_by",
    )
    .bind(MAX_ATTEMPTS)
    .bind(MAX_JOBS_PER_TICK)
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?;
    if jobs.is_empty() {
        return Ok(());
    }

    let mut groups: HashMap<String, Vec<(PendingJob, BatchItem)>> = HashMap::new();
    for job in jobs {
        match prepare_text_job(state, pool, &job).await {
            Ok(prepared) => {
                let model = provider.model_for(prepared.request.task, prepared.request.tier);
                let item = BatchItem {
                    key: job.id.to_string(),
                    request: prepared.request,
                };
                groups.entry(model).or_default().push((job, item));
            }
            Err(message) => {
                let _ = mark_failed(pool, job.id, &message).await;
            }
        }
    }

    for (model, group) in groups {
        let items: Vec<BatchItem> = group.iter().map(|(_, item)| item.clone()).collect();
        match provider.submit(&model, &items).await {
            Ok(provider_batch_id) => {
                let batch_id: Uuid = sqlx::query_scalar(
                    "insert into ai_batches (provider, provider_batch_id, model, request_count)
                     values ($1, $2, $3, $4) returning id",
                )
                .bind(provider.name())
                .bind(&provider_batch_id)
                .bind(&model)
                .bind(items.len() as i32)
                .fetch_one(pool)
                .await
                .map_err(|error| error.to_string())?;
                let ids: Vec<Uuid> = group.iter().map(|(job, _)| job.id).collect();
                sqlx::query("update ai_jobs set batch_id = $1 where id = any($2)")
                    .bind(batch_id)
                    .bind(&ids)
                    .execute(pool)
                    .await
                    .map_err(|error| error.to_string())?;
                tracing::info!(%provider_batch_id, %model, jobs = ids.len(), "Submitted AI batch");
            }
            Err(error) => {
                tracing::warn!(%error, %model, "Batch submission failed; running jobs interactively");
                for (job, _) in &group {
                    requeue_interactive(pool, job.id, &format!("batch unavailable: {error}")).await;
                }
            }
        }
    }
    Ok(())
}

async fn poll_running(state: &AppState) -> Result<(), String> {
    let pool = state.db_pool();
    let Some(provider) = state.ai.batch_provider() else {
        return Ok(());
    };
    let batches = sqlx::query_as::<_, RunningBatch>(
        "select id, provider_batch_id, extract(epoch from (now() - created_at))::bigint as age_secs
         from ai_batches where status = 'running' order by created_at limit 20",
    )
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?;

    for batch in batches {
        let _ = sqlx::query("update ai_batches set last_polled_at = now() where id = $1")
            .bind(batch.id)
            .execute(pool)
            .await;
        match provider.poll(&batch.provider_batch_id).await {
            Ok(BatchState::Running) => {
                if batch.age_secs > MAX_BATCH_AGE_SECS {
                    fail_batch(state, batch.id, "the batch took too long").await;
                }
            }
            Ok(BatchState::Succeeded(results)) => handle_results(state, batch.id, results).await,
            Ok(BatchState::Failed(message)) => fail_batch(state, batch.id, &message).await,
            Ok(BatchState::Cancelled) => {
                fail_batch(state, batch.id, "the batch was cancelled").await
            }
            Ok(BatchState::Expired) => fail_batch(state, batch.id, "the batch expired").await,
            Err(error) => {
                tracing::warn!(batch = %batch.provider_batch_id, %error, "Could not poll batch")
            }
        }
    }
    Ok(())
}

async fn fail_batch(state: &AppState, batch_id: Uuid, reason: &str) {
    let pool = state.db_pool();
    tracing::warn!(%batch_id, %reason, "AI batch failed; re-queuing its jobs");
    let _ = sqlx::query(
        "update ai_batches set status = 'failed', error = $2, completed_at = now() where id = $1",
    )
    .bind(batch_id)
    .bind(reason)
    .execute(pool)
    .await;
    let ids: Vec<Uuid> =
        sqlx::query_scalar("select id from ai_jobs where batch_id = $1 and status = 'running'")
            .bind(batch_id)
            .fetch_all(pool)
            .await
            .unwrap_or_default();
    for id in ids {
        requeue_interactive(pool, id, reason).await;
    }
}

async fn handle_results(
    state: &AppState,
    batch_id: Uuid,
    results: Vec<(String, Result<crate::ai::TextOutput, crate::ai::AiError>)>,
) {
    let pool = state.db_pool();
    let mut handled: Vec<Uuid> = Vec::new();
    for (key, outcome) in results {
        let Ok(job_id) = Uuid::parse_str(&key) else {
            continue;
        };
        let job = sqlx::query_as::<_, PendingJob>(
            "select id, session_id, job_type, input_resource_id, attempts, params, requested_by
             from ai_jobs where id = $1 and batch_id = $2 and status = 'running'",
        )
        .bind(job_id)
        .bind(batch_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();
        let Some(job) = job else { continue };
        handled.push(job.id);
        match outcome {
            Ok(output) => match check_output(&job.job_type, &output.text) {
                Ok(_) => {
                    let transcript =
                        lecture_text(state, pool, job.session_id, job.input_resource_id).await;
                    let finished = match transcript {
                        Ok(transcript) => {
                            finish_text_job(
                                state,
                                pool,
                                &job,
                                &transcript,
                                &output.text,
                                Some(&output),
                                "batch",
                            )
                            .await
                        }
                        Err(message) => Err(message),
                    };
                    match finished {
                        Ok(resource) => {
                            let _ = mark_completed(pool, job.id, resource).await;
                        }
                        Err(message) => requeue_interactive(pool, job.id, &message).await,
                    }
                }
                // The cheap model's batch answer was unusable: let the
                // interactive path escalate to a stronger model.
                Err(reason) => requeue_interactive(pool, job.id, &reason).await,
            },
            Err(error) => requeue_interactive(pool, job.id, &error.to_string()).await,
        }
    }
    // Anything the provider did not answer for gets re-run.
    let missing: Vec<Uuid> =
        sqlx::query_scalar("select id from ai_jobs where batch_id = $1 and status = 'running'")
            .bind(batch_id)
            .fetch_all(pool)
            .await
            .unwrap_or_default();
    for id in missing.into_iter().filter(|id| !handled.contains(id)) {
        requeue_interactive(pool, id, "the batch returned no answer for this job").await;
    }
    let _ = sqlx::query(
        "update ai_batches set status = 'succeeded', completed_at = now() where id = $1",
    )
    .bind(batch_id)
    .execute(pool)
    .await;
}

pub async fn run_loop(state: AppState) {
    let base = Duration::from_secs(20);
    loop {
        activity::sleep(base).await;
        if !activity::db_active() {
            continue;
        }
        if let Err(error) = tick(&state).await {
            tracing::warn!(%error, "Batch coordinator tick failed");
        }
    }
}
