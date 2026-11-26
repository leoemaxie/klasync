//! Durable execution for the `ai_jobs` queue.
//!
//! Jobs are created by lecturers (`/ai-jobs`), by students opening their
//! archive studio, and automatically after a lecture ends. `process_job` claims
//! a job atomically, builds a prompt from the lecture transcript, runs it
//! through the [`AiGateway`] (which picks the provider/model, retries and
//! escalates), persists structured output, and records a human-readable failure
//! reason when something goes wrong so the UI never has to guess.
//!
//! Jobs created in `batch` mode are handled by `batch.rs` and share
//! `prepare_text_job` / `finish_text_job` with this module.

use futures_util::{stream, StreamExt};
use serde::Serialize;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::{
    activity,
    ai::{extract_json, Task, TextOutput, TextRequest, Thinking, TranscribeRequest},
    api::error::ApiError,
    auth::guard::AuthenticatedLecturer,
    embeddings,
    state::AppState,
    study,
};

pub const MAX_ATTEMPTS: i32 = 3;

#[derive(Debug, FromRow, Clone)]
pub struct PendingJob {
    pub id: Uuid,
    pub session_id: Uuid,
    pub job_type: String,
    pub input_resource_id: Option<Uuid>,
    pub attempts: i32,
    pub params: serde_json::Value,
    pub requested_by: Uuid,
}

#[derive(Debug, FromRow)]
struct InputResource {
    resource_type: String,
    storage_key: Option<String>,
    content: Option<String>,
    content_type: Option<String>,
    duration_sec: Option<i32>,
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

    // Claiming also switches the job to interactive so a batch job that a
    // student (or the dispatcher) asked for now is not picked up twice.
    let job = sqlx::query_as::<_, PendingJob>(
        "update ai_jobs set status = 'running', started_at = now(), attempts = attempts + 1,
                error_message = null, mode = 'interactive'
         where id = $1 and requested_by = $2 and status in ('queued', 'failed') and attempts < $3
         returning id, session_id, job_type, input_resource_id, attempts, params, requested_by",
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
            mark_completed(pool, job.id, output_resource_id).await?;
            Ok(ExecutionResponse {
                job_id: job.id,
                status: "completed",
                output_resource_id,
                attempts: job.attempts,
            })
        }
        Err(message) => {
            tracing::warn!(job_id = %job.id, job_type = %job.job_type, attempt = job.attempts, error = %message, "AI job failed");
            mark_failed(pool, job.id, &message).await?;
            Err(ApiError::service_unavailable())
        }
    }
}

pub async fn mark_completed(
    pool: &PgPool,
    job_id: Uuid,
    output_resource_id: Option<Uuid>,
) -> Result<(), ApiError> {
    sqlx::query(
        "update ai_jobs set status = 'completed', output_resource_id = $2, completed_at = now(), error_message = null where id = $1",
    )
    .bind(job_id)
    .bind(output_resource_id)
    .execute(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?;
    Ok(())
}

pub async fn mark_failed(pool: &PgPool, job_id: Uuid, message: &str) -> Result<(), ApiError> {
    // Out of attempts => 'failed' stays terminal; otherwise the worker loop
    // retries it after a short cool-down.
    sqlx::query("update ai_jobs set status = 'failed', error_message = $2 where id = $1")
        .bind(job_id)
        .bind(message.chars().take(500).collect::<String>())
        .execute(pool)
        .await
        .map_err(|_| ApiError::service_unavailable())?;
    Ok(())
}

// ---- transcript sourcing -----------------------------------------------------

/// The best available text for a lecture: live captions, or the transcript of
/// the recording when that holds clearly more (captions can be sparse).
pub async fn lecture_text(
    state: &AppState,
    pool: &PgPool,
    session_id: Uuid,
    input_resource_id: Option<Uuid>,
) -> Result<study::Transcript, String> {
    let captions = study::load_transcript(pool, session_id)
        .await
        .map_err(|error| format!("could not read the lecture captions: {error}"))?;

    let mut candidate: Option<String> = None;
    if let Some(resource_id) = input_resource_id {
        let resource = sqlx::query_as::<_, InputResource>(
            "select resource_type, storage_key, content::text as content, content_type, duration_sec
             from lecture_resources where id = $1 and session_id = $2",
        )
        .bind(resource_id)
        .bind(session_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| error.to_string())?;
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
        .bind(session_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| error.to_string())?;
        candidate = latest.map(|content| study::text_from_resource_json(&content));
    }
    let candidate = candidate.filter(|text| !text.trim().is_empty());

    match candidate {
        Some(text) if captions.text.trim().is_empty() || text.len() > captions.text.len() * 2 => {
            let duration_sec = study::last_offset(&text).unwrap_or(captions.duration_sec);
            Ok(study::Transcript { text, duration_sec })
        }
        _ if !captions.text.trim().is_empty() => Ok(captions),
        _ => Err(
            "No transcript is available for this lecture yet. Captions are recorded while the lecturer is live."
                .to_owned(),
        ),
    }
}

/// Map-reduce for transcripts longer than a provider's context: the cheap tier
/// condenses each section, so the mid tier still sees the whole lecture.
pub(crate) async fn condense(
    state: &AppState,
    text: &str,
    limit: usize,
    session_id: Uuid,
) -> Result<String, String> {
    if text.chars().count() <= limit {
        return Ok(text.to_owned());
    }
    // Each section is condensed to roughly a quarter of its size.
    let window = (limit / 3).max(4_000);
    let mut sections: Vec<String> = Vec::new();
    let mut current = String::new();
    for line in text.lines() {
        if current.len() + line.len() > window && !current.is_empty() {
            sections.push(std::mem::take(&mut current));
        }
        current.push_str(line);
        current.push('\n');
    }
    if !current.trim().is_empty() {
        sections.push(current);
    }
    let system = "You condense a section of a university lecture transcript into dense study notes. \
        Keep every definition, formula, named concept, example and instruction. Keep the [mm:ss] markers \
        for the start of each topic. Write terse bullet points, no preamble. Aim for about a quarter of the original length.";
    let condensed = stream::iter(sections.into_iter().enumerate())
        .map(|(index, section)| async move {
            let request = TextRequest::new(Task::Condense, system, section.clone())
                .thinking(Thinking::Minimal)
                .max_tokens(2_500)
                .for_session(session_id);
            let range = format!(
                "[{} – {}]",
                section
                    .lines()
                    .find_map(study::leading_stamp)
                    .unwrap_or_default(),
                section
                    .lines()
                    .rev()
                    .find_map(study::leading_stamp)
                    .unwrap_or_default()
            );
            let output = state
                .ai
                .generate(&request)
                .await
                .map_err(|error| error.to_string())?;
            Ok::<_, String>((index, format!("{range}\n{}", output.text.trim())))
        })
        .buffer_unordered(4)
        .collect::<Vec<_>>()
        .await;
    let mut parts = Vec::new();
    for item in condensed {
        parts.push(item?);
    }
    parts.sort_by_key(|(index, _)| *index);
    Ok(parts
        .into_iter()
        .map(|(_, text)| text)
        .collect::<Vec<_>>()
        .join("\n\n"))
}

// ---- job execution -------------------------------------------------------------

pub struct PreparedText {
    pub request: TextRequest,
    pub transcript: study::Transcript,
}

fn is_json_job(job_type: &str) -> bool {
    matches!(job_type, "flashcards" | "chapters" | "visualization")
}

/// Builds the model request for a text job. Shared with batch submission.
pub async fn prepare_text_job(
    state: &AppState,
    pool: &PgPool,
    job: &PendingJob,
) -> Result<PreparedText, String> {
    let transcript = lecture_text(state, pool, job.session_id, job.input_resource_id).await?;
    let limit = state.ai.max_input_chars();
    let text = if transcript.text.chars().count() > limit {
        condense(state, &transcript.text, limit, job.session_id).await?
    } else {
        transcript.text.clone()
    };
    let task = Task::from_job_type(&job.job_type);
    let system = study::system_prompt(&job.job_type, &job.params);
    let mut request = TextRequest::new(task, system, text)
        .thinking(Thinking::Low)
        .for_job(job.id, job.session_id);
    if is_json_job(&job.job_type) {
        request = request.json().max_tokens(12_000);
    } else {
        request = request.max_tokens(4_000);
    }
    Ok(PreparedText {
        request,
        transcript,
    })
}

/// Does `raw` hold something we can store for this kind of job?
pub fn check_output(job_type: &str, raw: &str) -> Result<usize, String> {
    let json = extract_json(raw);
    match job_type {
        "flashcards" => {
            let cards = json
                .as_ref()
                .map(study::parse_flashcards)
                .unwrap_or_default();
            if cards.is_empty() {
                Err("no flashcards could be read from the answer".to_owned())
            } else {
                Ok(cards.len())
            }
        }
        "chapters" => match json.as_ref().and_then(study::parse_chapters) {
            Some(chapters) if !chapters.is_empty() => Ok(chapters.len()),
            _ => Err("no chapters could be read from the answer".to_owned()),
        },
        "visualization" => match json.as_ref().and_then(study::normalise_concept_map) {
            Some(_) => Ok(1),
            None => Err("the concept map was not valid".to_owned()),
        },
        _ => {
            if raw.trim().is_empty() {
                Err("empty answer".to_owned())
            } else {
                Ok(1)
            }
        }
    }
}

async fn execute_claimed_job(
    state: &AppState,
    pool: &PgPool,
    job: &PendingJob,
) -> Result<Option<Uuid>, String> {
    match job.job_type.as_str() {
        // Lecture materials (outline, section, assemble, embed) live in their own module.
        job_type if job_type.starts_with("material_") => {
            crate::materials::run_job(state, pool, job).await
        }
        "transcribe" => run_transcribe(state, pool, job).await,
        "embed" => embeddings::embed_session(state, pool, job.session_id)
            .await
            .map(|_| None),
        _ => run_text_job(state, pool, job).await,
    }
}

async fn run_text_job(
    state: &AppState,
    pool: &PgPool,
    job: &PendingJob,
) -> Result<Option<Uuid>, String> {
    let prepared = prepare_text_job(state, pool, job).await?;
    let job_type = job.job_type.clone();
    let result = state
        .ai
        .generate_validated(&prepared.request, |output| {
            check_output(&job_type, &output.text)
        })
        .await;
    match result {
        Ok((_, output)) => {
            finish_text_job(
                state,
                pool,
                job,
                &prepared.transcript,
                &output.text,
                Some(&output),
                "sync",
            )
            .await
        }
        // Chapters and concept maps have a deterministic fallback so students
        // still get something useful when every model fails.
        Err(error) if matches!(job.job_type.as_str(), "chapters" | "visualization") => {
            tracing::warn!(%error, job_type = %job.job_type, "AI unavailable; using heuristic fallback");
            finish_text_job(state, pool, job, &prepared.transcript, "", None, "sync").await
        }
        Err(error) => Err(error.to_string()),
    }
}

/// Persists a finished text job (from a sync call, or a batch result).
pub async fn finish_text_job(
    state: &AppState,
    pool: &PgPool,
    job: &PendingJob,
    transcript: &study::Transcript,
    raw: &str,
    output: Option<&TextOutput>,
    via: &'static str,
) -> Result<Option<Uuid>, String> {
    let source = if output.is_some() { "ai" } else { "heuristic" };
    let metadata = match output {
        Some(output) => {
            let cost = record_job_usage(state, pool, job, output, via).await;
            serde_json::json!({"model": output.model, "provider": output.provider, "via": via, "cost_usd": cost})
        }
        None => serde_json::json!({"model": "heuristic"}),
    };
    let persisted = persist_output(pool, job, raw, transcript, source).await?;
    if persisted == 0 && is_json_job(&job.job_type) {
        return Err(
            "The AI model returned output that could not be understood. Please try again."
                .to_owned(),
        );
    }
    let resource_type = output_type(&job.job_type);
    let content = serde_json::json!({ "text": raw, "items": persisted });
    insert_output(pool, job.session_id, resource_type, &content, &metadata).await
}

async fn run_transcribe(
    state: &AppState,
    pool: &PgPool,
    job: &PendingJob,
) -> Result<Option<Uuid>, String> {
    let preferred = match job.input_resource_id {
        Some(resource_id) => sqlx::query_as::<_, InputResource>(
            "select resource_type, storage_key, content::text as content, content_type, duration_sec
             from lecture_resources where id = $1 and session_id = $2 and storage_key is not null",
        )
        .bind(resource_id)
        .bind(job.session_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| error.to_string())?,
        None => None,
    };
    let audio = match preferred {
        Some(audio) => audio,
        None => sqlx::query_as::<_, InputResource>(
            "select resource_type, storage_key, content::text as content, content_type, duration_sec
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
    let output = state
        .ai
        .transcribe(&TranscribeRequest {
            audio: bytes,
            mime: audio
                .content_type
                .clone()
                .unwrap_or_else(|| "audio/webm".to_owned()),
            language: Some("English".to_owned()),
            duration_sec: audio.duration_sec.map(|d| d.max(0) as u32),
            timestamps: true,
            session_id: Some(job.session_id),
            job_id: Some(job.id),
        })
        .await
        .map_err(|error| error.to_string())?;
    let cost = record_job_usage(state, pool, job, &output, "sync").await;
    let metadata =
        serde_json::json!({"model": output.model, "provider": output.provider, "cost_usd": cost});
    let content = serde_json::json!({ "text": output.text, "source": "transcription" });
    let resource = insert_output(pool, job.session_id, "transcript", &content, &metadata).await?;
    // A transcript unlocks the study material and search index for this lecture.
    if let Err(error) =
        crate::post_lecture::queue_study_jobs(state, pool, job.session_id, job.requested_by).await
    {
        tracing::warn!(%error, "Could not queue study jobs after transcription");
    }
    Ok(resource)
}

async fn insert_output(
    pool: &PgPool,
    session_id: Uuid,
    resource_type: &str,
    content: &serde_json::Value,
    metadata: &serde_json::Value,
) -> Result<Option<Uuid>, String> {
    let output_id = Uuid::now_v7();
    sqlx::query(
        "insert into lecture_resources (id, session_id, resource_type, content, checksum) values ($1, $2, $3, $4, $5)",
    )
    .bind(output_id)
    .bind(session_id)
    .bind(resource_type)
    .bind(content)
    .bind(metadata.to_string())
    .execute(pool)
    .await
    .map_err(|error| error.to_string())?;
    Ok(Some(output_id))
}

pub(crate) async fn record_job_usage(
    state: &AppState,
    pool: &PgPool,
    job: &PendingJob,
    output: &TextOutput,
    via: &'static str,
) -> f64 {
    let task = Task::from_job_type(&job.job_type);
    let cost = state.ai.record_output(
        task,
        task.default_tier(),
        output,
        via,
        Some(job.session_id),
        Some(job.id),
    );
    let _ = sqlx::query(
        "update ai_jobs set provider = $2, model = $3, input_tokens = $4, output_tokens = $5, cost_usd = ($6::float8)::numeric where id = $1",
    )
    .bind(job.id)
    .bind(output.provider)
    .bind(&output.model)
    .bind(output.usage.input_tokens as i64)
    .bind(output.usage.output_tokens as i64)
    .bind(cost)
    .execute(pool)
    .await;
    cost
}

fn output_type(job_type: &str) -> &'static str {
    match job_type {
        "transcribe" => "transcript",
        "summarize" => "summary",
        "flashcards" => "flashcards",
        _ => "notes",
    }
}

/// Persists structured study rows. Returns the number of rows created.
pub async fn persist_output(
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
            // Cards that already existed (a retried or duplicate job) still count.
            Ok(count.max(usize::from(
                job.params.get("topic").map(|t| t.is_null()).unwrap_or(true),
            )))
        }
        "visualization" => {
            let has_topic = job
                .params
                .get("topic")
                .and_then(|t| t.as_str())
                .map(|t| !t.trim().is_empty())
                .unwrap_or(false);
            // A batch result arriving after the student already generated one on
            // demand should not add a second copy of the same map.
            if !has_topic {
                let exists = sqlx::query_scalar::<_, bool>(
                    "select exists(select 1 from session_visualizations where session_id = $1 and created_at > now() - interval '1 day')",
                )
                .bind(job.session_id)
                .fetch_one(pool)
                .await
                .unwrap_or(false);
                if exists {
                    return Ok(1);
                }
            }
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

// ---- queue loop -------------------------------------------------------------------

#[derive(Debug, FromRow)]
struct JobCandidate {
    id: Uuid,
    requested_by: Uuid,
}

/// Polls the durable queue so a deployment can process jobs without requiring
/// a lecturer to dispatch each one manually. Claiming is atomic inside
/// `process_job`, so multiple instances may run this loop safely. While the
/// service is idle only Redis is read, so a scale-to-zero database can sleep.
pub async fn run_loop(state: AppState) {
    let base = std::time::Duration::from_secs(3);
    let consumer = format!("worker-{}", Uuid::now_v7());
    let mut redis_group_ready = false;
    loop {
        activity::sleep(base).await;

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
                        if let (Ok(job_id), Some(pool)) =
                            (Uuid::parse_str(&job_id), state.production_database())
                        {
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
                        // retried by the database poll below.
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

        if !activity::db_active() {
            continue;
        }
        let Some(pool) = state.production_database() else {
            continue;
        };

        // A crashed worker leaves jobs 'running' forever; hand them back. Batch
        // jobs legitimately stay 'running' for hours, so they are excluded.
        let _ = sqlx::query(
            "update ai_jobs set status = 'queued', error_message = 'Recovered after worker restart'
             where status = 'running' and mode = 'interactive' and batch_id is null
               and started_at < now() - interval '10 minutes'",
        )
        .execute(pool)
        .await;

        let candidate = sqlx::query_as::<_, JobCandidate>(
            "select id, requested_by from ai_jobs
             where attempts < $1 and mode = 'interactive' and run_after <= now()
               and (status = 'queued'
                    or (status = 'failed' and started_at < now() - interval '20 seconds'))
             order by (status = 'queued') desc, (job_type like 'material\\_%') asc, created_at asc limit 1",
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
