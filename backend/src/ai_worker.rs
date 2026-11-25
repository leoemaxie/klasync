//! Durable execution for the `ai_jobs` queue.
//!
//! Jobs are created by lecturers (`/ai-jobs`) and by students opening their
//! archive studio (flashcards, chapters, visualizations). `process_job` claims a
//! job atomically, builds a prompt from the lecture transcript, calls the AI
//! adapter, persists structured output and records a human-readable failure
//! reason when something goes wrong, so the UI never has to guess why nothing
//! happened.

use serde::Serialize;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::{
    ai::{extract_json, AiWorkItem},
    api::error::ApiError,
    auth::guard::AuthenticatedLecturer,
    state::AppState,
    study,
};

pub const MAX_ATTEMPTS: i32 = 3;
/// Keep prompts within the small context windows of free models.
const MAX_TRANSCRIPT_CHARS: usize = 28_000;

#[derive(Debug, FromRow)]
struct PendingJob {
    id: Uuid,
    session_id: Uuid,
    job_type: String,
    input_resource_id: Option<Uuid>,
    attempts: i32,
    params: serde_json::Value,
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
    requester_id: Uuid,
) -> Result<ExecutionResponse, ApiError> {
    if let Some(redis) = &state.redis {
        let lock = redis
            .try_lock("ai-job", &job_id.to_string(), 300)
            .await
            .map_err(|_| ApiError::service_unavailable())?
            .ok_or_else(|| ApiError::conflict("This AI job is already being processed."))?;
        let result = process_job_unlocked(state, job_id, requester_id).await;
        let _ = lock.release().await;
        return result;
    }
    process_job_unlocked(state, job_id, requester_id).await
}

async fn process_job_unlocked(
    state: &AppState,
    job_id: Uuid,
    requester_id: Uuid,
) -> Result<ExecutionResponse, ApiError> {
    let pool = state
        .production_database()
        .ok_or_else(ApiError::service_unavailable)?;

    let job = sqlx::query_as::<_, PendingJob>(
        "update ai_jobs set status = 'running', started_at = now(), attempts = attempts + 1, error_message = null
         where id = $1 and requested_by = $2 and status in ('queued', 'failed') and attempts < $3
         returning id, session_id, job_type, input_resource_id, attempts, params",
    )
    .bind(job_id)
    .bind(requester_id)
    .bind(MAX_ATTEMPTS)
    .fetch_optional(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to claim AI job");
        ApiError::service_unavailable()
    })?
    .ok_or_else(|| ApiError::conflict("AI job is not available for processing"))?;

    match execute_claimed_job(state, pool, &job).await {
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
                output_resource_id,
                attempts: job.attempts,
            })
        }
        Err(message) => {
            tracing::warn!(job_id = %job.id, job_type = %job.job_type, attempt = job.attempts, error = %message, "AI job failed");
            // Out of attempts => 'failed' stays terminal; otherwise the worker
            // loop retries it after a short delay.
            sqlx::query("update ai_jobs set status = 'failed', error_message = $2 where id = $1")
                .bind(job.id)
                .bind(message.chars().take(500).collect::<String>())
                .execute(pool)
                .await
                .map_err(|_| ApiError::service_unavailable())?;
            Err(ApiError::service_unavailable())
        }
    }
}

/// Resolves the text an AI job should work on.
async fn lecture_text(
    state: &AppState,
    pool: &PgPool,
    job: &PendingJob,
    resource: &Option<InputResource>,
) -> Result<study::Transcript, String> {
    // 1. Timestamped captions are the best source: chapters need real offsets.
    let captions = study::load_transcript(pool, job.session_id)
        .await
        .map_err(|error| format!("could not read the lecture captions: {error}"))?;
    if !captions.text.trim().is_empty() {
        return Ok(captions);
    }

    // 2. A transcript resource (e.g. produced by the transcribe job, or uploaded).
    let mut candidate: Option<String> = None;
    if let Some(resource) = resource {
        if resource.resource_type != "audio" && resource.resource_type != "recording" {
            if let Some(key) = resource.storage_key.as_deref() {
                if let Ok(bytes) = state.storage.get(key).await {
                    candidate = Some(String::from_utf8_lossy(&bytes).to_string());
                }
            } else if let Some(content) = resource.content.as_deref() {
                candidate = Some(study::text_from_resource_json(content));
            }
        }
    }
    if candidate
        .as_deref()
        .map(|t| t.trim().is_empty())
        .unwrap_or(true)
    {
        let latest = sqlx::query_scalar::<_, String>(
            "select content::text from lecture_resources
             where session_id = $1 and resource_type = 'transcript' and content is not null
             order by created_at desc limit 1",
        )
        .bind(job.session_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| error.to_string())?;
        candidate = latest.map(|content| study::text_from_resource_json(&content));
    }
    match candidate {
        Some(text) if !text.trim().is_empty() => Ok(study::Transcript {
            text,
            duration_sec: 0,
        }),
        _ => Err(
            "No transcript is available for this lecture yet. Captions are recorded while the lecturer is live."
                .to_owned(),
        ),
    }
}

async fn execute_claimed_job(
    state: &AppState,
    pool: &PgPool,
    job: &PendingJob,
) -> Result<Option<Uuid>, String> {
    let resource = match job.input_resource_id {
        Some(resource_id) => sqlx::query_as::<_, InputResource>(
            "select resource_type, storage_key, content::text as content, content_type
             from lecture_resources where id = $1 and session_id = $2",
        )
        .bind(resource_id)
        .bind(job.session_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| error.to_string())?,
        None => None,
    };

    // ---- transcription of recorded audio -------------------------------
    if job.job_type == "transcribe" {
        let audio = match resource {
            Some(r) if r.storage_key.is_some() => r,
            _ => sqlx::query_as::<_, InputResource>(
                "select resource_type, storage_key, content::text as content, content_type
                 from lecture_resources
                 where session_id = $1 and resource_type in ('audio','recording') and storage_key is not null
                 order by created_at desc limit 1",
            )
            .bind(job.session_id)
            .fetch_optional(pool)
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "There is no audio recording to transcribe.".to_owned())?,
        };
        let key = audio.storage_key.as_deref().unwrap_or_default();
        let bytes = state
            .storage
            .get(key)
            .await
            .map_err(|error| format!("could not read the stored recording: {error}"))?;
        let format = audio_format(audio.content_type.as_deref());
        let input = crate::ai::transcription_input(&bytes, &format, Some("en"));
        let output = state
            .ai
            .execute(AiWorkItem {
                job_id: job.id,
                session_id: job.session_id,
                job_type: job.job_type.clone(),
                input,
            })
            .await
            .map_err(|error| error.to_string())?;
        record_usage(state, pool, job, &output.metadata).await;
        return insert_output(pool, job, "transcript", &output).await;
    }

    // ---- text based jobs ------------------------------------------------
    let transcript = lecture_text(state, pool, job, &resource).await;

    // Visualizations and chapters have a deterministic fallback so students
    // still get something useful when the model is rate-limited or offline.
    let transcript = match transcript {
        Ok(t) => t,
        Err(message) => return Err(message),
    };
    let text = study::clamp_text(&transcript.text, MAX_TRANSCRIPT_CHARS);

    let system = study::system_prompt(&job.job_type, &job.params);
    let work = AiWorkItem {
        job_id: job.id,
        session_id: job.session_id,
        job_type: job.job_type.clone(),
        input: serde_json::json!({ "text": text, "system": system }),
    };
    let ai_result = state.ai.execute(work).await;

    let (content, metadata, source) = match ai_result {
        Ok(output) => {
            record_usage(state, pool, job, &output.metadata).await;
            let raw = output
                .content
                .get("text")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_owned();
            (raw, output.metadata, "ai")
        }
        Err(error) => match job.job_type.as_str() {
            "chapters" | "visualization" => {
                tracing::warn!(%error, job_type = %job.job_type, "AI unavailable; using heuristic fallback");
                (
                    String::new(),
                    serde_json::json!({"model": "heuristic", "reason": error.to_string()}),
                    "heuristic",
                )
            }
            _ => return Err(error.to_string()),
        },
    };

    let persisted = persist_output(pool, job, &content, &transcript, source).await?;
    if persisted == 0
        && !matches!(
            job.job_type.as_str(),
            "summarize" | "explain" | "lecture_qa_index" | "question_answer"
        )
    {
        return Err(
            "The AI model returned output that could not be understood. Please try again."
                .to_owned(),
        );
    }

    let resource_type = output_type(&job.job_type);
    let output = crate::ai::AiWorkResult {
        content: serde_json::json!({ "text": content, "items": persisted }),
        metadata,
    };
    insert_output(pool, job, resource_type, &output).await
}

async fn insert_output(
    pool: &PgPool,
    job: &PendingJob,
    resource_type: &str,
    output: &crate::ai::AiWorkResult,
) -> Result<Option<Uuid>, String> {
    let output_id = Uuid::now_v7();
    sqlx::query(
        "insert into lecture_resources (id, session_id, resource_type, content, checksum) values ($1, $2, $3, $4, $5)",
    )
    .bind(output_id)
    .bind(job.session_id)
    .bind(resource_type)
    .bind(&output.content)
    .bind(output.metadata.to_string())
    .execute(pool)
    .await
    .map_err(|error| error.to_string())?;
    Ok(Some(output_id))
}

async fn record_usage(
    state: &AppState,
    pool: &PgPool,
    job: &PendingJob,
    metadata: &serde_json::Value,
) {
    let usage = metadata.get("usage").filter(|value| !value.is_null());
    let model = metadata.get("model").and_then(|value| value.as_str());
    let input_tokens = usage
        .and_then(|value| value.get("input_tokens"))
        .and_then(|value| value.as_i64());
    let output_tokens = usage
        .and_then(|value| value.get("output_tokens"))
        .and_then(|value| value.as_i64());
    let cost_usd = usage
        .and_then(|value| value.get("cost"))
        .and_then(|value| value.as_f64());
    let _ = sqlx::query(
        "update ai_jobs set provider = $2, model = $3, input_tokens = $4, output_tokens = $5, cost_usd = ($6::float8)::numeric where id = $1",
    )
    .bind(job.id)
    .bind(state.ai.provider_name())
    .bind(model)
    .bind(input_tokens)
    .bind(output_tokens)
    .bind(cost_usd)
    .execute(pool)
    .await;
}

/// Maps `audio/webm;codecs=opus` & friends to the short format names the
/// transcription endpoint understands.
fn audio_format(content_type: Option<&str>) -> String {
    let subtype = content_type
        .and_then(|value| value.split(';').next())
        .and_then(|value| value.split('/').nth(1))
        .unwrap_or("wav")
        .trim()
        .to_ascii_lowercase();
    match subtype.as_str() {
        "mpeg" | "mpeg3" | "x-mpeg" => "mp3".to_owned(),
        "mp4" | "x-m4a" | "aac" | "x-aac" => "m4a".to_owned(),
        "x-wav" | "wave" | "vnd.wave" => "wav".to_owned(),
        "opus" => "ogg".to_owned(),
        other => other.to_owned(),
    }
}

/// Persists structured study rows. Returns the number of rows created.
async fn persist_output(
    pool: &PgPool,
    job: &PendingJob,
    raw: &str,
    transcript: &study::Transcript,
    source: &str,
) -> Result<usize, String> {
    let json = extract_json(raw);
    match job.job_type.as_str() {
        "chapters" => {
            let chapters = match json.as_ref().and_then(study::parse_chapters) {
                Some(chapters) if !chapters.is_empty() => chapters,
                _ => study::heuristic_chapters(&transcript.text, transcript.duration_sec),
            };
            let mut count = 0;
            for (index, chapter) in chapters.iter().enumerate() {
                sqlx::query(
                    "insert into session_chapters (id, session_id, chapter_index, title, summary, start_timestamp_sec, end_timestamp_sec)
                     values ($1, $2, $3, $4, $5, $6, $7)
                     on conflict (session_id, chapter_index) do update set title = excluded.title,
                       summary = excluded.summary, start_timestamp_sec = excluded.start_timestamp_sec,
                       end_timestamp_sec = excluded.end_timestamp_sec",
                )
                .bind(Uuid::now_v7())
                .bind(job.session_id)
                .bind(index as i32 + 1)
                .bind(chapter.title.chars().take(250).collect::<String>())
                .bind(&chapter.summary)
                .bind(chapter.start_sec)
                .bind(chapter.end_sec)
                .execute(pool)
                .await
                .map_err(|error| error.to_string())?;
                count += 1;
            }
            Ok(count)
        }
        "flashcards" => {
            let cards = json
                .as_ref()
                .map(study::parse_flashcards)
                .unwrap_or_default();
            let mut count = 0;
            for card in cards {
                let inserted = sqlx::query(
                    "insert into session_flashcards (id, session_id, prompt, answer, topic_tag, difficulty)
                     select $1, $2, $3, $4, $5, $6
                     where not exists (select 1 from session_flashcards
                                       where session_id = $2 and lower(prompt) = lower($3))",
                )
                .bind(Uuid::now_v7())
                .bind(job.session_id)
                .bind(&card.prompt)
                .bind(&card.answer)
                .bind(card.topic_tag.as_deref().map(|t| t.chars().take(100).collect::<String>()))
                .bind(&card.difficulty)
                .execute(pool)
                .await
                .map_err(|error| error.to_string())?;
                count += inserted.rows_affected() as usize;
            }
            Ok(count)
        }
        "visualization" => {
            let spec = json
                .as_ref()
                .and_then(study::normalise_concept_map)
                .unwrap_or_else(|| study::heuristic_concept_map(&transcript.text));
            let title = spec
                .get("title")
                .and_then(|value| value.as_str())
                .unwrap_or("Lecture concept map")
                .to_owned();
            if spec
                .get("nodes")
                .and_then(|nodes| nodes.as_array())
                .map(|nodes| nodes.is_empty())
                .unwrap_or(true)
            {
                return Ok(0);
            }
            let source = if json.is_some() && source == "ai" {
                "ai"
            } else {
                "heuristic"
            };
            sqlx::query(
                "insert into session_visualizations (id, session_id, kind, title, spec, source)
                 values ($1, $2, 'concept_map', $3, $4, $5)",
            )
            .bind(Uuid::now_v7())
            .bind(job.session_id)
            .bind(title)
            .bind(&spec)
            .bind(source)
            .execute(pool)
            .await
            .map_err(|error| error.to_string())?;
            Ok(1)
        }
        _ => Ok(if raw.trim().is_empty() { 0 } else { 1 }),
    }
}

#[derive(Debug, FromRow)]
struct JobCandidate {
    id: Uuid,
    requested_by: Uuid,
}

/// Polls the durable queue so a deployment can process jobs without requiring
/// a lecturer to dispatch each one manually. Claiming remains atomic inside
/// `process_job`, so multiple instances may run this loop safely.
pub async fn run_loop(state: AppState) {
    let mut ticker = tokio::time::interval(std::time::Duration::from_secs(3));
    let consumer = format!("worker-{}", Uuid::now_v7());
    let mut redis_group_ready = false;
    loop {
        ticker.tick().await;
        let Some(pool) = state.production_database() else {
            continue;
        };

        // A crashed worker leaves jobs 'running' forever; hand them back.
        let _ = sqlx::query(
            "update ai_jobs set status = 'queued', error_message = 'Recovered after worker restart'
             where status = 'running' and started_at < now() - interval '10 minutes'",
        )
        .execute(pool)
        .await;

        if let Some(redis) = &state.redis {
            if !redis_group_ready {
                match redis.ensure_ai_consumer_group().await {
                    Ok(()) => redis_group_ready = true,
                    Err(error) => {
                        tracing::warn!(%error, "Unable to initialize AI Redis consumer group")
                    }
                }
            }
            if redis_group_ready {
                match redis.read_ai_job(&consumer).await {
                    Ok(Some((message_id, job_id))) => {
                        if let Ok(job_id) = Uuid::parse_str(&job_id) {
                            if let Ok(Some(requester)) = sqlx::query_scalar::<_, Uuid>(
                                "select requested_by from ai_jobs where id = $1",
                            )
                            .bind(job_id)
                            .fetch_optional(pool)
                            .await
                            {
                                if let Err(error) = process_job(&state, job_id, requester).await {
                                    tracing::warn!(%job_id, error = %error, "AI Redis stream job failed");
                                }
                            }
                        }
                        // Always acknowledge: failures are recorded on the row and
                        // retried by the database poll below. Leaving the message
                        // unacknowledged stranded the job as 'failed' forever.
                        let _ = redis.acknowledge_ai_job(&message_id).await;
                        continue;
                    }
                    Ok(None) => {}
                    Err(error) => {
                        tracing::warn!(%error, "Unable to read AI Redis stream; using database queue")
                    }
                }
            }
        }

        // Queued jobs first, then failed jobs that still have attempts left
        // (after a short cool-down so a rate-limited model can recover).
        let candidate = sqlx::query_as::<_, JobCandidate>(
            "select id, requested_by from ai_jobs
             where attempts < $1
               and (status = 'queued'
                    or (status = 'failed' and started_at < now() - interval '20 seconds'))
             order by (status = 'queued') desc, created_at asc limit 1",
        )
        .bind(MAX_ATTEMPTS)
        .fetch_optional(pool)
        .await;
        let Ok(Some(job)) = candidate else {
            continue;
        };
        if let Err(error) = process_job(&state, job.id, job.requested_by).await {
            tracing::warn!(job_id = %job.id, error = %error, "AI job processing failed");
        }
    }
}

fn output_type(job_type: &str) -> &'static str {
    match job_type {
        "transcribe" => "transcript",
        "summarize" => "summary",
        "flashcards" => "flashcards",
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
        .ok_or_else(ApiError::service_unavailable)?;
    let owns_job = sqlx::query_scalar::<_, bool>(
        "select exists(select 1 from ai_jobs j join lecture_sessions s on s.id = j.session_id
                       where j.id = $1 and j.requested_by = $2 and s.short_code = upper($3) and s.lecturer_id = $2)",
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
