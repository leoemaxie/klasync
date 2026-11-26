//! Automatic work after a lecture ends: transcribe sparse recordings, queue the
//! study material (as discounted batch jobs when the provider supports it) and
//! the search index, and backfill the index for lectures that predate it.

use std::time::Duration;

use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::{activity, state::AppState};

/// Captions below this many characters are too sparse to build study material
/// from, so the recording is transcribed instead.
const SPARSE_CAPTION_CHARS: i64 = 3_000;

#[derive(Debug, FromRow)]
struct Candidate {
    id: Uuid,
    lecturer_id: Uuid,
    caption_chars: i64,
    has_audio: bool,
    has_transcript: bool,
    ended_minutes_ago: i64,
}

async fn insert_job(
    pool: &PgPool,
    session_id: Uuid,
    requested_by: Uuid,
    job_type: &str,
    mode: &str,
) -> Result<bool, sqlx::Error> {
    // Never stack duplicates: skip when one is already queued/running/done.
    let inserted = sqlx::query(
        "insert into ai_jobs (id, session_id, requested_by, job_type, mode)
         select $1, $2, $3, $4, $5
         where not exists (
           select 1 from ai_jobs j
           where j.session_id = $2 and j.job_type = $4
             and (j.status in ('queued', 'running', 'completed')
                  or (j.status = 'failed' and j.attempts < 3))
         )",
    )
    .bind(Uuid::now_v7())
    .bind(session_id)
    .bind(requested_by)
    .bind(job_type)
    .bind(mode)
    .execute(pool)
    .await?;
    Ok(inserted.rows_affected() > 0)
}

/// Queues chapters, flashcards, a concept map and the search index for a
/// lecture, once. Returns the number of jobs created.
pub async fn queue_study_jobs(
    state: &AppState,
    pool: &PgPool,
    session_id: Uuid,
    lecturer_id: Uuid,
) -> Result<usize, String> {
    let claimed = sqlx::query(
        "insert into session_post_processing (session_id) values ($1) on conflict do nothing",
    )
    .bind(session_id)
    .execute(pool)
    .await
    .map_err(|error| error.to_string())?
    .rows_affected()
        > 0;
    if !claimed {
        return Ok(0);
    }
    let mode = if state.ai.supports_batch() {
        "batch"
    } else {
        "interactive"
    };
    let mut created = 0;
    for job_type in ["chapters", "flashcards", "visualization"] {
        if insert_job(pool, session_id, lecturer_id, job_type, mode)
            .await
            .map_err(|error| error.to_string())?
        {
            created += 1;
        }
    }
    if state.ai.supports_embeddings()
        && insert_job(pool, session_id, lecturer_id, "embed", "interactive")
            .await
            .map_err(|error| error.to_string())?
    {
        created += 1;
    }
    tracing::info!(%session_id, created, mode, "Queued post-lecture study jobs");
    // The lecturer's structured material is drafted after the study jobs, so it can use their chapters.
    crate::materials::queue_after_lecture(state, pool, session_id, lecturer_id).await;
    Ok(created)
}

pub async fn prepare_ended_sessions(state: &AppState, pool: &PgPool) -> Result<(), String> {
    let candidates = sqlx::query_as::<_, Candidate>(
        "select s.id, s.lecturer_id,
                coalesce((select sum(length(c.text)) from caption_chunks c where c.session_id = s.id), 0)::bigint as caption_chars,
                exists(select 1 from lecture_resources r where r.session_id = s.id
                       and r.resource_type in ('audio', 'recording') and r.storage_key is not null) as has_audio,
                exists(select 1 from lecture_resources r where r.session_id = s.id
                       and r.resource_type = 'transcript') as has_transcript,
                (extract(epoch from (now() - s.ended_at)) / 60)::bigint as ended_minutes_ago
         from lecture_sessions s
         where s.status = 'ended' and s.deleted_at is null
           and s.ended_at < now() - interval '90 seconds'
           and s.ended_at > now() - interval '3 days'
           and not exists (select 1 from session_post_processing p where p.session_id = s.id)
         limit 10",
    )
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?;

    for session in candidates {
        let enough_text = session.caption_chars >= SPARSE_CAPTION_CHARS || session.has_transcript;
        if enough_text {
            queue_study_jobs(state, pool, session.id, session.lecturer_id).await?;
        } else if session.has_audio && state.ai.supports_transcription() {
            // Sparse captions: transcribe the recording first. Finishing that
            // job queues the study material (see `run_transcribe`).
            let _ = insert_job(
                pool,
                session.id,
                session.lecturer_id,
                "transcribe",
                "interactive",
            )
            .await;
            if session.caption_chars >= 400 {
                // Enough to show something now; the transcript will refine search.
                queue_study_jobs(state, pool, session.id, session.lecturer_id).await?;
            }
        } else if session.caption_chars >= 400 {
            queue_study_jobs(state, pool, session.id, session.lecturer_id).await?;
        } else if session.ended_minutes_ago > 60 {
            // Nothing was captured; stop looking at this lecture.
            let _ = sqlx::query(
                "insert into session_post_processing (session_id) values ($1) on conflict do nothing",
            )
            .bind(session.id)
            .execute(pool)
            .await;
        }
    }
    Ok(())
}

/// Indexes lectures recorded before embeddings existed (or after a model change).
pub async fn backfill_embeddings(
    state: &AppState,
    pool: &PgPool,
    limit: i64,
) -> Result<usize, String> {
    let Some((model, dimensions)) = state.ai.embedding_model() else {
        return Ok(0);
    };
    if dimensions != crate::embeddings::DIMENSIONS {
        return Ok(0);
    }
    let sessions = sqlx::query_as::<_, (Uuid, Uuid)>(
        "select s.id, s.lecturer_id from lecture_sessions s
         where s.status = 'ended' and s.deleted_at is null
           and not exists (select 1 from lecture_chunks c where c.session_id = s.id and c.model = $1)
           and not exists (
             select 1 from ai_jobs j where j.session_id = s.id and j.job_type = 'embed'
               and (j.status in ('queued', 'running')
                    or j.attempts >= 3
                    or j.created_at > now() - interval '1 hour'))
           and (exists (select 1 from caption_chunks c where c.session_id = s.id)
                or exists (select 1 from lecture_resources r where r.session_id = s.id and r.resource_type = 'transcript'))
         order by s.ended_at desc nulls last
         limit $2",
    )
    .bind(&model)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?;
    let mut queued = 0;
    for (session_id, lecturer_id) in sessions {
        // `insert_job` would refuse (an old completed embed job exists), so
        // create the job directly: the index for this model is genuinely missing.
        sqlx::query(
            "insert into ai_jobs (id, session_id, requested_by, job_type, mode) values ($1, $2, $3, 'embed', 'interactive')",
        )
        .bind(Uuid::now_v7())
        .bind(session_id)
        .bind(lecturer_id)
        .execute(pool)
        .await
        .map_err(|error| error.to_string())?;
        queued += 1;
    }
    if queued > 0 {
        tracing::info!(queued, "Queued embedding backfill jobs");
    }
    Ok(queued)
}

/// Background maintenance: runs only while the service is in use.
pub async fn run_loop(state: AppState) {
    let base = Duration::from_secs(30);
    loop {
        activity::sleep(base).await;
        if !activity::db_active() {
            continue;
        }
        let pool = state.db_pool();
        if let Err(error) = prepare_ended_sessions(&state, pool).await {
            tracing::warn!(%error, "Post-lecture preparation failed");
        }
        if let Err(error) = backfill_embeddings(&state, pool, 3).await {
            tracing::warn!(%error, "Embedding backfill failed");
        }
    }
}
