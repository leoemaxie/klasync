//! The generation pipeline, built on the shared `ai_jobs` queue and worker.
//!
//! ```text
//! post_lecture / API -> material_outline -> N x material_section -> material_assemble
//! ```
//!
//! * **outline** plans the sections from the transcript, chapters and concept map.
//! * **section** jobs fan out, one per outline section, each reading only its
//!   slice of the transcript; results land in `lecture_materials.gen_state`.
//! * **assemble** (queued by whichever section finishes last) merges sections,
//!   runs the post-stage generators (concept-map figure, summary, review
//!   questions, key terms), repairs and validates the document and saves
//!   version 1 with origin `ai`.
//!
//! Every job is keyed on material, generation and section, so a retry or a
//! double trigger never creates duplicates. Jobs of a superseded generation
//! exit quietly.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    blocks::{self, Block, BlockKind, CalloutVariant, MaterialDoc, Origin},
    compare::token_jaccard,
    prompts::{self, PROMPT_VERSION},
    registry::{
        is_enabled, registry, GenerationContext, Generator, Outline, PluginError, PluginKind,
        SectionPlan, Stage,
    },
    store::{self, MaterialRow, MaterialSettings},
};
use crate::{
    ai::{extract_json, Task, TextRequest, Thinking},
    ai_worker::{lecture_text, PendingJob, MAX_ATTEMPTS},
    state::AppState,
    study,
};

#[derive(Debug, Deserialize)]
struct JobParams {
    material_id: Uuid,
    #[serde(default)]
    generation: i32,
    #[serde(default)]
    section: Option<usize>,
    #[serde(default)]
    instruction: Option<String>,
    /// `regenerate` for a lecturer-requested rewrite of one section.
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    heading_id: Option<String>,
    #[serde(default)]
    version_id: Option<Uuid>,
}

// ---- entry points -----------------------------------------------------------------

/// Starts (or restarts) generation for a material. Returns the new generation
/// number, or an error when a run is already in flight.
pub async fn start_generation(
    pool: &PgPool,
    material: &MaterialRow,
    requested_by: Uuid,
    delay_secs: u32,
) -> Result<i32, String> {
    let generation: Option<i32> = sqlx::query_scalar(
        "update lecture_materials
         set generation = generation + 1, generation_state = 'queued', generation_error = null,
             gen_state = '{}'::jsonb, updated_at = now()
         where id = $1
           and (generation_state not in ('queued', 'generating')
                or updated_at < now() - interval '30 minutes')
         returning generation",
    )
    .bind(material.id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;
    let generation = generation.ok_or_else(|| "generation is already running".to_owned())?;
    store::enqueue_job(
        pool,
        material.session_id,
        requested_by,
        "material_outline",
        json!({"material_id": material.id, "generation": generation, "delay_secs": delay_secs}),
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(generation)
}

/// Worker entry point (called from `ai_worker` for `material_*` jobs).
pub async fn run_job(
    state: &AppState,
    pool: &PgPool,
    job: &PendingJob,
) -> Result<Option<Uuid>, String> {
    let params: JobParams = serde_json::from_value(job.params.clone())
        .map_err(|e| format!("bad material job parameters: {e}"))?;
    let result = match job.job_type.as_str() {
        "material_outline" => run_outline(state, pool, job, &params).await,
        "material_section" if params.mode.as_deref() == Some("regenerate") => {
            run_regenerate(state, pool, job, &params).await
        }
        "material_section" => run_section(state, pool, job, &params, None).await,
        "material_assemble" => run_assemble(state, pool, job, &params).await,
        "material_embed" => {
            super::compare::run_embed_job(state, pool, job, params.version_id).await
        }
        other => Err(format!("unknown material job type {other}")),
    };
    if let Err(error) = &result {
        if job.attempts >= MAX_ATTEMPTS && params.mode.as_deref() != Some("regenerate") {
            let _ = sqlx::query(
                "update lecture_materials set generation_state = 'failed', generation_error = $2, updated_at = now()
                 where id = $1 and generation = $3 and generation_state <> 'ready'",
            )
            .bind(params.material_id)
            .bind(error.chars().take(300).collect::<String>())
            .bind(params.generation)
            .execute(pool)
            .await;
        }
    }
    result
}

async fn load_current(pool: &PgPool, params: &JobParams) -> Result<Option<MaterialRow>, String> {
    let material = store::find_material(pool, params.material_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "the material no longer exists".to_owned())?;
    if material.generation != params.generation {
        tracing::info!(material = %material.id, job_generation = params.generation, current = material.generation, "Skipping job from a superseded generation");
        return Ok(None);
    }
    Ok(Some(material))
}

// ---- shared AI call -----------------------------------------------------------------

/// One JSON-mode model call, validated by `parse`; returns the parsed value
/// and the model that produced it. Usage is recorded by the gateway.
pub(crate) async fn ask<T>(
    state: &AppState,
    job: &PendingJob,
    system: String,
    input: String,
    max_tokens: u32,
    parse: impl Fn(&Value) -> Result<T, String>,
) -> Result<(T, String), String> {
    let request = TextRequest::new(Task::Material, system, input)
        .json()
        .thinking(Thinking::Low)
        .max_tokens(max_tokens)
        .for_job(job.id, job.session_id);
    let (value, output) = state
        .ai
        .generate_validated(&request, |output| {
            let json = extract_json(&prompts::fix_json_latex_escapes(&output.text))
                .ok_or_else(|| "the answer contained no JSON".to_owned())?;
            parse(&json)
        })
        .await
        .map_err(|e| e.to_string())?;
    Ok((value, output.model))
}

async fn note_model(pool: &PgPool, material_id: Uuid, generation: i32, model: &str) {
    let _ = sqlx::query(
        "update lecture_materials
         set gen_state = jsonb_set(gen_state, '{models}',
               coalesce(gen_state->'models', '[]'::jsonb) || to_jsonb($2::text), true)
         where id = $1 and generation = $3 and not (coalesce(gen_state->'models', '[]'::jsonb) ? $2)",
    )
    .bind(material_id)
    .bind(model)
    .bind(generation)
    .execute(pool)
    .await;
}

// ---- inputs ---------------------------------------------------------------------------

async fn load_chapters(pool: &PgPool, session_id: Uuid) -> Vec<study::Chapter> {
    sqlx::query_as::<_, (String, String, i32, i32)>(
        "select title, summary, start_timestamp_sec, end_timestamp_sec
         from session_chapters where session_id = $1 order by chapter_index",
    )
    .bind(session_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(title, summary, start_sec, end_sec)| study::Chapter {
        title,
        summary,
        start_sec,
        end_sec,
    })
    .collect()
}

#[derive(Debug, Clone)]
pub(crate) struct ConceptMap {
    pub id: Uuid,
    pub spec: Value,
}

pub(crate) async fn load_concept_map(pool: &PgPool, session_id: Uuid) -> Option<ConceptMap> {
    let (id, spec) = sqlx::query_as::<_, (Uuid, Value)>(
        "select id, spec from session_visualizations
         where session_id = $1 and kind = 'concept_map' order by created_at desc limit 1",
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()?;
    Some(ConceptMap { id, spec })
}

/// `(label, group, importance)` of the concept map's nodes, most important first.
pub(crate) fn concept_nodes(map: &ConceptMap) -> Vec<(String, String, i64)> {
    let mut nodes: Vec<(String, String, i64)> = map
        .spec
        .get("nodes")
        .and_then(Value::as_array)
        .map(|nodes| {
            nodes
                .iter()
                .filter_map(|n| {
                    Some((
                        n.get("label")?.as_str()?.to_owned(),
                        n.get("group")
                            .and_then(Value::as_str)
                            .unwrap_or("Concepts")
                            .to_owned(),
                        n.get("importance").and_then(Value::as_i64).unwrap_or(2),
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    nodes.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
    nodes
}

/// The same topic's published material from an earlier offering of the course
/// (same lecturer and course code), matched by lecture-title similarity.
pub async fn find_previous_material(
    pool: &PgPool,
    material: &MaterialRow,
    lecture_title: &str,
) -> Option<MaterialDoc> {
    let rows = sqlx::query_as::<_, (String, Value)>(
        "select s.title, v.doc
         from lecture_materials m
         join courses c on c.id = m.course_id
         join lecture_sessions s on s.id = m.session_id
         join material_versions v on v.id = m.published_version_id
         join courses cur on cur.id = $1
         where m.status = 'published' and c.id <> cur.id and c.lecturer_id = cur.lecturer_id
           and lower(trim(c.code)) = lower(trim(cur.code))",
    )
    .bind(material.course_id)
    .fetch_all(pool)
    .await
    .ok()?;
    rows.into_iter()
        .map(|(title, doc)| (token_jaccard(&title, lecture_title), doc))
        .filter(|(score, _)| *score >= 0.3)
        .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        .and_then(|(_, doc)| store::parse_doc(&doc).ok())
}

/// A compact view of last time's material: headings and an opening line each.
fn previous_context(doc: &MaterialDoc, max_chars: usize) -> String {
    let mut out = String::new();
    for section in doc.sections() {
        if section.heading.is_empty() {
            continue;
        }
        let body: String = section.body_text().chars().take(240).collect();
        out.push_str(&format!(
            "- {}: {}\n",
            section.heading,
            body.replace('\n', " ")
        ));
        if out.len() > max_chars {
            break;
        }
    }
    out.chars().take(max_chars).collect()
}

fn input_hash(transcript: &str, settings: &MaterialSettings) -> String {
    let mut hasher = Sha256::new();
    hasher.update(transcript.as_bytes());
    hasher.update(
        serde_json::to_string(settings)
            .unwrap_or_default()
            .as_bytes(),
    );
    hasher.update(PROMPT_VERSION.as_bytes());
    crate::utils::hex_lower(hasher.finalize())
}

// ---- outline ---------------------------------------------------------------------------

async fn run_outline(
    state: &AppState,
    pool: &PgPool,
    job: &PendingJob,
    params: &JobParams,
) -> Result<Option<Uuid>, String> {
    let Some(material) = load_current(pool, params).await? else {
        return Ok(None);
    };
    let session = store::session_info(pool, material.session_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "the lecture no longer exists".to_owned())?;
    let course = store::course_info(pool, material.course_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "the course no longer exists".to_owned())?;
    let settings = store::load_settings(pool, material.course_id).await;
    let transcript = lecture_text(state, pool, material.session_id, job.input_resource_id).await?;
    let chapters = load_chapters(pool, material.session_id).await;
    let concepts = load_concept_map(pool, material.session_id).await;
    let previous = if settings.seed_from_previous {
        find_previous_material(pool, &material, &session.title).await
    } else {
        None
    };

    let limit = state.ai.max_input_chars();
    let body = if transcript.text.chars().count() > limit {
        crate::ai_worker::condense(state, &transcript.text, limit, material.session_id).await?
    } else {
        transcript.text.clone()
    };
    let mut input = format!(
        "Course: {} {}\nLecture title: {}\n",
        course.code, course.title, session.title
    );
    if !chapters.is_empty() {
        input.push_str(&format!(
            "\nChapters detected in the lecture:\n{}\n",
            prompts::chapters_prompt(&chapters)
        ));
    }
    if let Some(map) = &concepts {
        let labels: Vec<String> = concept_nodes(map)
            .into_iter()
            .take(20)
            .map(|n| n.0)
            .collect();
        if !labels.is_empty() {
            input.push_str(&format!("\nKey concepts: {}\n", labels.join(", ")));
        }
    }
    if let Some(previous) = &previous {
        input.push_str(&format!(
            "\nThe same topic last time (reuse its structure where still relevant, but follow THIS lecture):\n{}\n",
            previous_context(previous, 2_000)
        ));
    }
    if let Some(slides) = store::load_slides(pool, material.id).await {
        input.push_str(&format!(
            "\nThe lecturer's slides (follow their order and wording where they match the lecture):\n{}\n",
            super::slides::prompt_window(&slides, 8_000)
        ));
    }
    input.push_str(&format!("\nTranscript:\n{body}"));

    let duration = transcript.duration_sec;
    let title = session.title.clone();
    let asked = ask(
        state,
        job,
        prompts::outline_system(&settings),
        input,
        3_000,
        |value| {
            prompts::parse_outline(value, duration, &title)
                .ok_or_else(|| "the outline had no usable sections".to_owned())
        },
    )
    .await;
    let outline = match asked {
        Ok((outline, model)) => {
            note_model(pool, material.id, material.generation, &model).await;
            outline
        }
        // The structure is the one thing we can rebuild without a model.
        Err(error) => {
            tracing::warn!(%error, material = %material.id, "AI outline failed; using the lecture chapters");
            let chapters = if chapters.is_empty() {
                study::heuristic_chapters(&transcript.text, duration)
            } else {
                chapters
            };
            prompts::outline_from_chapters(&chapters, &session.title, duration)
        }
    };

    let saved = sqlx::query(
        "update lecture_materials
         set gen_state = jsonb_build_object('outline', $2::jsonb, 'sections', '{}'::jsonb,
                                            'duration', $3::int, 'input_hash', $4::text,
                                            'models', coalesce(gen_state->'models', '[]'::jsonb)),
             generation_state = 'generating', updated_at = now()
         where id = $1 and generation = $5",
    )
    .bind(material.id)
    .bind(serde_json::to_value(&outline).map_err(|e| e.to_string())?)
    .bind(duration)
    .bind(input_hash(&transcript.text, &settings))
    .bind(material.generation)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    if saved.rows_affected() == 0 {
        return Ok(None);
    }
    for section in &outline.sections {
        store::enqueue_job(
            pool,
            material.session_id,
            job.requested_by,
            "material_section",
            json!({"material_id": material.id, "generation": material.generation, "section": section.index}),
        )
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(None)
}

// ---- sections --------------------------------------------------------------------------

async fn enabled_generators(
    pool: &PgPool,
    stage: Stage,
    course_id: Uuid,
) -> Vec<Arc<dyn Generator>> {
    let mut out = Vec::new();
    for generator in registry().generators(stage) {
        if is_enabled(pool, PluginKind::Generator, generator.id(), Some(course_id)).await {
            out.push(generator.clone());
        }
    }
    out
}

fn outline_from_state(gen_state: &Value) -> Result<(Outline, i32), String> {
    let outline: Outline = serde_json::from_value(
        gen_state
            .get("outline")
            .cloned()
            .ok_or_else(|| "the outline is missing".to_owned())?,
    )
    .map_err(|e| format!("the stored outline is invalid: {e}"))?;
    let duration = gen_state
        .get("duration")
        .and_then(Value::as_i64)
        .unwrap_or(0) as i32;
    Ok((outline, duration))
}

/// Runs the section-stage generators for one plan and post-processes the result.
#[allow(clippy::too_many_arguments)]
async fn generate_section_blocks(
    state: &AppState,
    pool: &PgPool,
    job: &PendingJob,
    material: &MaterialRow,
    outline: &Outline,
    plan: &SectionPlan,
    settings: &MaterialSettings,
    duration: i32,
    instruction: Option<&str>,
    current_doc: Option<&MaterialDoc>,
    precomputed: Option<&str>,
) -> Result<Vec<Block>, String> {
    let transcript = lecture_text(state, pool, material.session_id, job.input_resource_id).await?;
    let session = store::session_info(pool, material.session_id)
        .await
        .map_err(|e| e.to_string())?;
    let previous = match (&session, settings.seed_from_previous) {
        (Some(session), true) => find_previous_material(pool, material, &session.title).await,
        _ => None,
    };
    let cx = GenerationContext {
        state,
        pool,
        job,
        material,
        settings,
        transcript: &transcript,
        outline,
        section: Some(plan),
        doc: current_doc,
        previous: previous.as_ref(),
        instruction,
        precomputed,
    };
    let mut blocks: Vec<Block> = Vec::new();
    for generator in enabled_generators(pool, Stage::Section, material.course_id).await {
        match generator.generate(&cx).await {
            Ok(mut produced) => blocks.append(&mut produced),
            Err(error) => return Err(format!("{}: {error}", generator.id())),
        }
    }
    prompts::repair_blocks(&mut blocks);
    let downgraded = prompts::ground_blocks(&mut blocks, duration.max(plan.end_sec));
    if downgraded > 0 {
        tracing::info!(
            section = plan.index,
            downgraded,
            "Blocks without transcript citations were marked supplemented"
        );
    }
    Ok(blocks)
}

/// The model input for one section: `(input, transcript excerpt)`.
fn section_writer_input(
    cx: &GenerationContext<'_>,
    plan: &SectionPlan,
    slides: Option<&str>,
) -> (String, String) {
    let limit = cx.state.ai.max_input_chars();
    let excerpt = prompts::slice_transcript(
        &cx.transcript.text,
        plan.start_sec,
        plan.end_sec,
        30,
        limit / 2,
    );
    let mut input =
        format!(
        "Handout: {}\nSection: {}\nWhat this section should teach: {}\nTranscript window: {}-{}\n",
        cx.outline.title,
        plan.title,
        if plan.objective.is_empty() { "(not specified)" } else { &plan.objective },
        study::format_offset(plan.start_sec as i64),
        study::format_offset(plan.end_sec as i64),
    );
    if let Some(previous) = cx.previous {
        input.push_str(&format!(
            "\nLast time this topic was taught (for structure only):\n{}\n",
            previous_context(previous, 1_200)
        ));
    }
    if let (Some(instruction), Some(doc)) = (cx.instruction, cx.doc) {
        if let Some(section) = doc.sections().into_iter().find(|s| s.heading == plan.title) {
            input.push_str(&format!(
                "\nCurrent text of this section (rewrite it):\n{}\n",
                section.body_text().chars().take(4_000).collect::<String>()
            ));
        }
        input.push_str(&format!("\nLecturer's instruction: {instruction}\n"));
    }
    if let Some(slides) = slides {
        input.push_str(&format!(
            "\nThe lecturer's slides (use their terminology and structure; the transcript stays the source of what was taught):\n{}\n",
            super::slides::prompt_window(slides, 6_000)
        ));
    }
    input.push_str(&format!("\nTranscript:\n{excerpt}"));
    (input, excerpt)
}

/// Should new section jobs be sent through the provider batch API (about half
/// price, minutes to hours of latency)? Opt in with `MATERIALS_BATCH=1`.
pub fn batch_sections_enabled() -> bool {
    matches!(
        std::env::var("MATERIALS_BATCH").as_deref(),
        Ok("1") | Ok("true")
    )
}

/// The model request for a queued `material_section` job, for batch
/// submission. `None` means the job cannot be batched (regeneration, a stale
/// generation) and should run interactively.
pub async fn prepare_section_request(
    state: &AppState,
    pool: &PgPool,
    job: &PendingJob,
) -> Result<Option<TextRequest>, String> {
    let params: JobParams = serde_json::from_value(job.params.clone())
        .map_err(|e| format!("bad material job parameters: {e}"))?;
    if params.mode.as_deref() == Some("regenerate") {
        return Ok(None);
    }
    let Some(material) = load_current(pool, &params).await? else {
        return Ok(None);
    };
    let index = params
        .section
        .ok_or_else(|| "no section index".to_owned())?;
    let (outline, _) = outline_from_state(&material.gen_state)?;
    let Some(plan) = outline.sections.get(index) else {
        return Ok(None);
    };
    let settings = store::load_settings(pool, material.course_id).await;
    let transcript = lecture_text(state, pool, material.session_id, job.input_resource_id).await?;
    let session = store::session_info(pool, material.session_id)
        .await
        .map_err(|e| e.to_string())?;
    let previous = match (&session, settings.seed_from_previous) {
        (Some(session), true) => find_previous_material(pool, &material, &session.title).await,
        _ => None,
    };
    let cx = GenerationContext {
        state,
        pool,
        job,
        material: &material,
        settings: &settings,
        transcript: &transcript,
        outline: &outline,
        section: Some(plan),
        doc: None,
        previous: previous.as_ref(),
        instruction: None,
        precomputed: None,
    };
    let slides = store::load_slides(pool, material.id).await;
    let (input, _) = section_writer_input(&cx, plan, slides.as_deref());
    Ok(Some(
        TextRequest::new(Task::Material, prompts::section_system(&settings), input)
            .json()
            .thinking(Thinking::Low)
            .max_tokens(8_000)
            .for_job(job.id, job.session_id),
    ))
}

/// Does this batch answer hold usable blocks?
pub fn check_section_answer(text: &str) -> Result<(), String> {
    let json = extract_json(&prompts::fix_json_latex_escapes(text))
        .ok_or_else(|| "the answer contained no JSON".to_owned())?;
    let (blocks, dropped) = prompts::parse_blocks(&json);
    if blocks.is_empty() || dropped > blocks.len() {
        Err("no usable blocks in the answer".to_owned())
    } else {
        Ok(())
    }
}

/// Stores a section from a provider batch answer and queues assembly when it
/// was the last one, exactly like an interactive section job.
pub async fn finish_section_job(
    state: &AppState,
    pool: &PgPool,
    job: &PendingJob,
    answer: &str,
) -> Result<(), String> {
    let params: JobParams = serde_json::from_value(job.params.clone())
        .map_err(|e| format!("bad material job parameters: {e}"))?;
    run_section(state, pool, job, &params, Some(answer))
        .await
        .map(|_| ())
}

async fn run_section(
    state: &AppState,
    pool: &PgPool,
    job: &PendingJob,
    params: &JobParams,
    precomputed: Option<&str>,
) -> Result<Option<Uuid>, String> {
    let Some(material) = load_current(pool, params).await? else {
        return Ok(None);
    };
    let index = params
        .section
        .ok_or_else(|| "no section index".to_owned())?;
    let (outline, duration) = outline_from_state(&material.gen_state)?;
    let plan = outline
        .sections
        .get(index)
        .ok_or_else(|| format!("the outline has no section {index}"))?;
    let settings = store::load_settings(pool, material.course_id).await;
    let mut blocks = generate_section_blocks(
        state,
        pool,
        job,
        &material,
        &outline,
        plan,
        &settings,
        duration,
        None,
        None,
        precomputed,
    )
    .await?;
    prompts::assign_section_ids(&mut blocks, index);

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let stored = sqlx::query(
        "update lecture_materials
         set gen_state = jsonb_set(gen_state, array['sections', $2::text], $3::jsonb, true), updated_at = now()
         where id = $1 and generation = $4",
    )
    .bind(material.id)
    .bind(index.to_string())
    .bind(serde_json::to_value(&blocks).map_err(|e| e.to_string())?)
    .bind(material.generation)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    if stored.rows_affected() == 0 {
        return Ok(None);
    }
    // The last section to finish queues assembly. The row lock serialises the
    // check; the enqueue is idempotent anyway.
    let (done, total): (i64, i64) = sqlx::query_as(
        "select (select count(*) from jsonb_object_keys(gen_state->'sections')),
                jsonb_array_length(gen_state->'outline'->'sections')::bigint
         from lecture_materials where id = $1 for update",
    )
    .bind(material.id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    if done >= total {
        store::enqueue_job(
            pool,
            material.session_id,
            job.requested_by,
            "material_assemble",
            json!({"material_id": material.id, "generation": material.generation}),
        )
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(None)
}

/// Lecturer-requested rewrite of one section of the working draft. Locked
/// blocks and the heading stay; the rest is replaced.
async fn run_regenerate(
    state: &AppState,
    pool: &PgPool,
    job: &PendingJob,
    params: &JobParams,
) -> Result<Option<Uuid>, String> {
    let material = store::find_material(pool, params.material_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "the material no longer exists".to_owned())?;
    let heading_id = params
        .heading_id
        .as_deref()
        .ok_or_else(|| "no section was named".to_owned())?;
    let draft_value = material
        .draft_doc
        .clone()
        .ok_or_else(|| "there is no draft to regenerate".to_owned())?;
    let mut doc = store::parse_doc(&draft_value)?;
    let section = doc
        .sections()
        .into_iter()
        .find(|s| s.block_id == heading_id)
        .ok_or_else(|| "that section is no longer in the draft".to_owned())?;
    let (mut start, mut end) = (i32::MAX, 0);
    for block in &section.blocks {
        for range in &block.source_ranges {
            start = start.min(range.start_sec);
            end = end.max(range.end_sec);
        }
    }
    let transcript = lecture_text(state, pool, material.session_id, job.input_resource_id).await?;
    if start == i32::MAX {
        start = 0;
        end = transcript.duration_sec.max(60);
    }
    let plan = SectionPlan {
        index: 0,
        title: section.heading.clone(),
        objective: String::new(),
        start_sec: start,
        end_sec: end,
    };
    let outline = Outline {
        title: doc.title.clone(),
        objectives: doc.meta.objectives.clone(),
        sections: vec![plan.clone()],
    };
    let settings = store::load_settings(pool, material.course_id).await;
    let fresh = generate_section_blocks(
        state,
        pool,
        job,
        &material,
        &outline,
        &plan,
        &settings,
        transcript.duration_sec,
        params.instruction.as_deref(),
        Some(&doc),
        None,
    )
    .await?;
    if fresh.is_empty() {
        return Err("the model returned no usable content".to_owned());
    }
    if !prompts::replace_section(&mut doc, heading_id, fresh) {
        return Err("that section is no longer in the draft".to_owned());
    }
    let updated = sqlx::query(
        "update lecture_materials set draft_doc = $2, draft_rev = draft_rev + 1, updated_at = now()
         where id = $1 and draft_rev = $3",
    )
    .bind(material.id)
    .bind(serde_json::to_value(&doc).map_err(|e| e.to_string())?)
    .bind(material.draft_rev)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    if updated.rows_affected() == 0 {
        return Err(
            "the draft changed while the section was being rewritten; trying again".to_owned(),
        );
    }
    Ok(None)
}

// ---- assemble --------------------------------------------------------------------------

async fn run_assemble(
    state: &AppState,
    pool: &PgPool,
    job: &PendingJob,
    params: &JobParams,
) -> Result<Option<Uuid>, String> {
    let Some(material) = load_current(pool, params).await? else {
        return Ok(None);
    };
    let (outline, _) = outline_from_state(&material.gen_state)?;
    let section_map = material
        .gen_state
        .get("sections")
        .and_then(Value::as_object)
        .ok_or_else(|| "no sections were generated".to_owned())?;
    let mut sections: Vec<(usize, Vec<Block>)> = Vec::new();
    for (key, value) in section_map {
        let index: usize = key.parse().map_err(|_| "bad section key".to_owned())?;
        let blocks: Vec<Block> = serde_json::from_value(value.clone())
            .map_err(|e| format!("bad stored section: {e}"))?;
        sections.push((index, blocks));
    }
    let settings = store::load_settings(pool, material.course_id).await;
    let mut doc = prompts::assemble(&outline, &sections, &settings.language);

    // Post stage over the merged document.
    let transcript = lecture_text(state, pool, material.session_id, job.input_resource_id).await?;
    let snapshot = doc.clone();
    let cx = GenerationContext {
        state,
        pool,
        job,
        material: &material,
        settings: &settings,
        transcript: &transcript,
        outline: &outline,
        section: None,
        doc: Some(&snapshot),
        previous: None,
        instruction: None,
        precomputed: None,
    };
    for generator in enabled_generators(pool, Stage::Post, material.course_id).await {
        match generator.generate(&cx).await {
            Ok(mut produced) => {
                for (n, block) in produced.iter_mut().enumerate() {
                    block.id = format!("p-{}-{}", generator.id(), n + 1);
                }
                doc.blocks.extend(produced);
            }
            // A missing summary must not cost the lecturer the whole handout.
            Err(error) => {
                tracing::warn!(generator = generator.id(), %error, "Post-stage generator failed; continuing")
            }
        }
    }

    // Quality pass: repair, validate; refuse to save something invalid.
    prompts::repair_blocks(&mut doc.blocks);
    doc.normalise_ids("b");
    doc.number_equations(1);
    if let Err(problems) = blocks::validate(&doc) {
        return Err(format!(
            "the assembled material is invalid: {}",
            problems.join("; ")
        ));
    }

    let models = material
        .gen_state
        .get("models")
        .and_then(Value::as_array)
        .map(|m| {
            m.iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|m| !m.is_empty());
    let hash = material
        .gen_state
        .get("input_hash")
        .and_then(Value::as_str)
        .map(str::to_owned);

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let locked: Option<(i32, Option<Value>)> = sqlx::query_as(
        "select generation, draft_doc from lecture_materials where id = $1 for update",
    )
    .bind(material.id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    let Some((generation, previous_draft)) = locked else {
        return Ok(None);
    };
    if generation != material.generation {
        return Ok(None);
    }
    if let Some(old) = previous_draft.and_then(|v| store::parse_doc(&v).ok()) {
        let kept = prompts::merge_locked(&mut doc, &old);
        if kept > 0 {
            tracing::info!(material = %material.id, kept, "Kept lecturer-locked blocks across regeneration");
        }
    }
    let (version_id, version_no) = store::insert_version(
        &mut tx,
        material.id,
        "ai",
        &doc,
        models.as_deref(),
        Some(PROMPT_VERSION),
        hash.as_deref(),
        false,
    )
    .await
    .map_err(|e| e.to_string())?;
    sqlx::query(
        "update lecture_materials
         set draft_doc = $2, draft_rev = draft_rev + 1, current_version_id = $3,
             generation_state = 'ready', generation_error = null, gen_state = '{}'::jsonb,
             updated_at = now()
         where id = $1",
    )
    .bind(material.id)
    .bind(serde_json::to_value(&doc).map_err(|e| e.to_string())?)
    .bind(version_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    store::emit(
        &mut *tx,
        "material.generated",
        json!({"material_id": material.id, "session_id": material.session_id,
               "course_id": material.course_id, "version_id": version_id, "version_no": version_no,
               "lecturer_id": material.lecturer_id}),
    )
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(None)
}

// ---- built-in generators -------------------------------------------------------------------

/// Writes the body of one outline section from its slice of the transcript.
pub struct SectionWriter;

#[async_trait]
impl Generator for SectionWriter {
    fn id(&self) -> &'static str {
        "section_writer"
    }
    fn label(&self) -> &'static str {
        "Section writer"
    }
    fn stage(&self) -> Stage {
        Stage::Section
    }
    async fn generate(&self, cx: &GenerationContext<'_>) -> Result<Vec<Block>, PluginError> {
        let plan = cx
            .section
            .ok_or_else(|| PluginError::failed("no section to write"))?;
        let slides = store::load_slides(cx.pool, cx.material.id).await;
        let (input, excerpt) = section_writer_input(cx, plan, slides.as_deref());
        let parse = |value: &Value| {
            let (blocks, dropped) = prompts::parse_blocks(value);
            if blocks.is_empty() {
                Err("no usable blocks in the answer".to_owned())
            } else if dropped > blocks.len() {
                Err("most blocks did not fit the schema".to_owned())
            } else {
                Ok(blocks)
            }
        };
        let asked = match cx.precomputed {
            // A provider batch already answered: parse it like a live answer.
            Some(text) => extract_json(&prompts::fix_json_latex_escapes(text))
                .ok_or_else(|| "the answer contained no JSON".to_owned())
                .and_then(|json| parse(&json))
                .map(|blocks| (blocks, "batch".to_owned())),
            None => {
                ask(
                    cx.state,
                    cx.job,
                    prompts::section_system(cx.settings),
                    input,
                    8_000,
                    parse,
                )
                .await
            }
        };
        match asked {
            Ok((blocks, model)) => {
                note_model(cx.pool, cx.material.id, cx.material.generation, &model).await;
                Ok(blocks)
            }
            // Only after the retries are spent: build from the transcript so
            // the lecturer still has something to edit.
            Err(error) if cx.job.attempts >= MAX_ATTEMPTS => {
                tracing::warn!(%error, section = plan.index, "AI section failed; using transcript text");
                Ok(prompts::fallback_section_blocks(plan, &excerpt))
            }
            Err(error) => Err(PluginError::failed(error)),
        }
    }
}

fn doc_text_for_model(cx: &GenerationContext<'_>) -> Result<String, PluginError> {
    let doc = cx
        .doc
        .ok_or_else(|| PluginError::failed("no document to read"))?;
    let mut text = format!("{}\n", doc.title);
    for block in &doc.blocks {
        text.push_str(&block.plain_text());
        text.push('\n');
    }
    Ok(study::clamp_text(&text, cx.state.ai.max_input_chars() / 2))
}

/// 5-8 key points. Falls back to the study pipeline's heuristic summary.
pub struct SummaryGenerator;

#[async_trait]
impl Generator for SummaryGenerator {
    fn id(&self) -> &'static str {
        "summary"
    }
    fn label(&self) -> &'static str {
        "Summary"
    }
    fn stage(&self) -> Stage {
        Stage::Post
    }
    async fn generate(&self, cx: &GenerationContext<'_>) -> Result<Vec<Block>, PluginError> {
        let text = doc_text_for_model(cx)?;
        let asked = ask(
            cx.state,
            cx.job,
            prompts::summary_system(cx.settings),
            text.clone(),
            1_500,
            |value| {
                let points = prompts::parse_summary(value);
                if points.is_empty() {
                    Err("no summary points".to_owned())
                } else {
                    Ok(points)
                }
            },
        )
        .await;
        let points = match asked {
            Ok((points, model)) => {
                note_model(cx.pool, cx.material.id, cx.material.generation, &model).await;
                points
            }
            Err(error) => {
                tracing::warn!(%error, "AI summary failed; using the heuristic summary");
                study::heuristic_summary(&text).1
            }
        };
        if points.is_empty() {
            return Ok(Vec::new());
        }
        Ok(vec![Block::new("", BlockKind::Summary { points })])
    }
}

/// 5-8 review questions with model answers. No fallback: no questions is fine.
pub struct ReviewQuestionsGenerator;

#[async_trait]
impl Generator for ReviewQuestionsGenerator {
    fn id(&self) -> &'static str {
        "review_questions"
    }
    fn label(&self) -> &'static str {
        "Review questions"
    }
    fn stage(&self) -> Stage {
        Stage::Post
    }
    async fn generate(&self, cx: &GenerationContext<'_>) -> Result<Vec<Block>, PluginError> {
        let text = doc_text_for_model(cx)?;
        let asked = ask(
            cx.state,
            cx.job,
            prompts::questions_system(cx.settings),
            text,
            3_000,
            |value| {
                let questions = prompts::parse_questions(value);
                if questions.is_empty() {
                    Err("no questions".to_owned())
                } else {
                    Ok(questions)
                }
            },
        )
        .await;
        match asked {
            Ok((questions, model)) => {
                note_model(cx.pool, cx.material.id, cx.material.generation, &model).await;
                Ok(vec![Block::new(
                    "",
                    BlockKind::ReviewQuestions { questions },
                )])
            }
            Err(error) => {
                tracing::warn!(%error, "AI review questions failed; omitting them");
                Ok(Vec::new())
            }
        }
    }
}

/// The lecture's concept map as a figure with a generated text alternative.
pub struct ConceptMapFigureGenerator;

/// Nodes and edges of a concept-map spec, for drawing.
pub(crate) fn concept_map_diagram(spec: &Value) -> Option<blocks::Diagram> {
    let text = |v: &Value| {
        v.as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| v.to_string())
    };
    let nodes: Vec<blocks::DiagramNode> = spec
        .get("nodes")?
        .as_array()?
        .iter()
        .filter_map(|n| {
            Some(blocks::DiagramNode {
                id: text(n.get("id")?),
                label: n.get("label")?.as_str()?.to_owned(),
                group: n.get("group").and_then(Value::as_str).map(str::to_owned),
            })
        })
        .collect();
    if nodes.is_empty() {
        return None;
    }
    let edges =
        spec.get("edges")
            .and_then(Value::as_array)
            .map(|edges| {
                edges
                    .iter()
                    .filter_map(|e| {
                        let from = text(e.get("from").or_else(|| e.get("source"))?);
                        let to = text(e.get("to").or_else(|| e.get("target"))?);
                        (nodes.iter().any(|n| n.id == from) && nodes.iter().any(|n| n.id == to))
                            .then(|| blocks::DiagramEdge {
                                from,
                                to,
                                label: e.get("label").and_then(Value::as_str).map(str::to_owned),
                            })
                    })
                    .collect()
            })
            .unwrap_or_default();
    Some(blocks::Diagram { nodes, edges })
}

/// Describes a concept map in words: used as alt text and as the text
/// fallback for formats without graphics.
pub(crate) fn describe_concept_map(spec: &Value) -> Option<(String, String)> {
    let nodes = spec.get("nodes")?.as_array()?;
    if nodes.is_empty() {
        return None;
    }
    let label_of = |id: &Value| -> Option<String> {
        let id = id
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| id.to_string());
        nodes
            .iter()
            .find(|n| {
                n.get("id").map(|v| {
                    v.as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| v.to_string())
                }) == Some(id.clone())
            })
            .and_then(|n| n.get("label").and_then(Value::as_str).map(str::to_owned))
    };
    let mut groups: Vec<(String, Vec<String>)> = Vec::new();
    for node in nodes {
        let (Some(label), group) = (
            node.get("label").and_then(Value::as_str),
            node.get("group")
                .and_then(Value::as_str)
                .unwrap_or("Concepts"),
        ) else {
            continue;
        };
        match groups.iter_mut().find(|(g, _)| g == group) {
            Some((_, labels)) => labels.push(label.to_owned()),
            None => groups.push((group.to_owned(), vec![label.to_owned()])),
        }
    }
    let mut lines: Vec<String> = groups
        .iter()
        .map(|(group, labels)| format!("{group}: {}", labels.join(", ")))
        .collect();
    let mut relations: Vec<String> = Vec::new();
    if let Some(edges) = spec.get("edges").and_then(Value::as_array) {
        for edge in edges.iter().take(30) {
            let from = edge
                .get("from")
                .or_else(|| edge.get("source"))
                .and_then(label_of);
            let to = edge
                .get("to")
                .or_else(|| edge.get("target"))
                .and_then(label_of);
            if let (Some(from), Some(to)) = (from, to) {
                let label = edge
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or("relates to");
                relations.push(format!("{from} {label} {to}"));
            }
        }
    }
    lines.extend(relations.iter().map(|r| r.to_owned()));
    let alt = format!(
        "Concept map with {} concepts in {} groups ({}).{}",
        nodes.len(),
        groups.len(),
        groups
            .iter()
            .map(|(g, _)| g.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        if relations.is_empty() {
            String::new()
        } else {
            format!(
                " Relationships include: {}.",
                relations
                    .iter()
                    .take(5)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        }
    );
    Some((alt, lines.join("\n")))
}

#[async_trait]
impl Generator for ConceptMapFigureGenerator {
    fn id(&self) -> &'static str {
        "concept_map_figure"
    }
    fn label(&self) -> &'static str {
        "Concept map figure"
    }
    fn stage(&self) -> Stage {
        Stage::Post
    }
    async fn generate(&self, cx: &GenerationContext<'_>) -> Result<Vec<Block>, PluginError> {
        let Some(map) = load_concept_map(cx.pool, cx.material.session_id).await else {
            return Ok(Vec::new());
        };
        let Some((alt, fallback)) = describe_concept_map(&map.spec) else {
            return Ok(Vec::new());
        };
        Ok(vec![Block::new(
            "",
            BlockKind::Figure {
                caption: "Concept map of this lecture".to_owned(),
                alt,
                visualization_id: Some(map.id),
                text_fallback: Some(fallback),
                diagram: concept_map_diagram(&map.spec),
            },
        )])
    }
}

/// Key terms: concept-map concepts plus defined terms, with where each is discussed.
pub struct GlossaryGenerator;

pub(crate) fn key_terms_table(
    doc: &MaterialDoc,
    concepts: &[(String, String, i64)],
) -> Option<Block> {
    let sections = doc.sections();
    let mut terms: Vec<String> = concepts.iter().take(20).map(|c| c.0.clone()).collect();
    for block in &doc.blocks {
        if let BlockKind::Definition { term, .. } = &block.kind {
            let term = blocks::strip_inline(term);
            if !terms.iter().any(|t| t.eq_ignore_ascii_case(&term)) {
                terms.push(term);
            }
        }
    }
    let mut rows: Vec<Vec<String>> = Vec::new();
    for term in terms {
        let needle = term.to_lowercase();
        let places: Vec<String> = sections
            .iter()
            .filter(|s| !s.heading.is_empty())
            .filter(|s| {
                s.body_text().to_lowercase().contains(&needle)
                    || s.heading.to_lowercase().contains(&needle)
            })
            .map(|s| s.heading.clone())
            .collect();
        rows.push(vec![
            term,
            if places.is_empty() {
                "\u{2014}".to_owned()
            } else {
                places.join("; ")
            },
        ]);
    }
    if rows.len() < 3 {
        return None;
    }
    Some(Block::new(
        "",
        BlockKind::Table {
            caption: Some("Key terms".to_owned()),
            header: vec!["Term".to_owned(), "Where it is discussed".to_owned()],
            rows,
        },
    ))
}

#[async_trait]
impl Generator for GlossaryGenerator {
    fn id(&self) -> &'static str {
        "glossary"
    }
    fn label(&self) -> &'static str {
        "Key terms"
    }
    fn stage(&self) -> Stage {
        Stage::Post
    }
    async fn generate(&self, cx: &GenerationContext<'_>) -> Result<Vec<Block>, PluginError> {
        let doc = cx.doc.ok_or_else(|| PluginError::failed("no document"))?;
        let concepts = match load_concept_map(cx.pool, cx.material.session_id).await {
            Some(map) => concept_nodes(&map),
            None => Vec::new(),
        };
        Ok(key_terms_table(doc, &concepts).into_iter().collect())
    }
}

#[allow(dead_code)]
fn _callout_is_used(_: CalloutVariant, _: Origin) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concept_map_is_described_in_words() {
        let spec = json!({
            "nodes": [
                {"id": "a", "label": "Conduction", "group": "Modes", "importance": 5},
                {"id": "b", "label": "Convection", "group": "Modes", "importance": 4},
                {"id": 3, "label": "Fourier's law", "group": "Laws", "importance": 3}
            ],
            "edges": [{"from": "a", "to": 3, "label": "is governed by"}]
        });
        let (alt, fallback) = describe_concept_map(&spec).unwrap();
        assert!(alt.starts_with("Concept map with 3 concepts in 2 groups (Modes, Laws)."));
        assert!(alt.contains("Conduction is governed by Fourier's law"));
        assert!(fallback.contains("Modes: Conduction, Convection"));
        assert!(describe_concept_map(&json!({"nodes": []})).is_none());
    }

    #[test]
    fn key_terms_lists_where_each_term_appears() {
        let mut doc = MaterialDoc::new("T");
        doc.blocks = vec![
            Block::new(
                "h1",
                BlockKind::Heading {
                    level: 1,
                    text: "Conduction".into(),
                },
            ),
            Block::new(
                "p1",
                BlockKind::Paragraph {
                    text: "Fourier's law links flux to gradient.".into(),
                },
            ),
            Block::new(
                "d1",
                BlockKind::Definition {
                    term: "Thermal conductivity".into(),
                    text: "A property.".into(),
                },
            ),
            Block::new(
                "h2",
                BlockKind::Heading {
                    level: 1,
                    text: "Convection".into(),
                },
            ),
            Block::new(
                "p2",
                BlockKind::Paragraph {
                    text: "Fluid motion carries heat.".into(),
                },
            ),
        ];
        let concepts = vec![
            ("Fourier's law".to_owned(), "Laws".to_owned(), 5),
            ("Convection".to_owned(), "Modes".to_owned(), 4),
        ];
        let table = key_terms_table(&doc, &concepts).unwrap();
        let BlockKind::Table { rows, .. } = &table.kind else {
            panic!()
        };
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], vec!["Fourier's law", "Conduction"]);
        assert_eq!(rows[1][1], "Convection");
        assert!(key_terms_table(&MaterialDoc::new("x"), &[]).is_none());
    }

    #[test]
    fn input_hash_changes_with_inputs() {
        let s = MaterialSettings::default();
        let a = input_hash("lecture", &s);
        assert_eq!(a, input_hash("lecture", &s));
        assert_ne!(a, input_hash("other", &s));
        let mut s2 = s.clone();
        s2.depth = "detailed".into();
        assert_ne!(a, input_hash("lecture", &s2));
    }

    #[test]
    fn previous_context_is_compact() {
        let mut doc = MaterialDoc::new("T");
        doc.blocks = vec![
            Block::new(
                "h",
                BlockKind::Heading {
                    level: 1,
                    text: "Topic".into(),
                },
            ),
            Block::new(
                "p",
                BlockKind::Paragraph {
                    text: "x".repeat(1000),
                },
            ),
        ];
        let ctx = previous_context(&doc, 500);
        assert!(ctx.starts_with("- Topic: "));
        assert!(ctx.len() < 400);
    }

    #[test]
    fn batch_answers_are_checked_like_live_ones() {
        let good = r#"{"blocks":[{"type":"paragraph","text":"Hello **world**","source_ranges":[{"start_sec":0,"end_sec":5}]}]}"#;
        assert!(check_section_answer(good).is_ok());
        assert!(check_section_answer("not json at all").is_err());
        assert!(check_section_answer(r#"{"blocks":[]}"#).is_err());
    }
}
