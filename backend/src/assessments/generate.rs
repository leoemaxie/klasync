//! The `assessment_generate` and `assessment_grade` jobs. They run on the
//! shared `ai_jobs` queue through the existing worker and gateway, so budget,
//! usage logging and retries behave like every other AI feature.

use std::collections::HashMap;

use futures_util::future::join_all;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    model::{AssessmentSettings, QuestionType, ScoringMode, SnapshotQuestion},
    prompts::{
        self, dedupe, generation_input, generation_system, grading_input, grading_system,
        looks_like_injection, parse_generated, parse_grade, solve_agrees, solve_input,
        solve_system, GeneratedQuestion, GenerationSpec, PROMPT_VERSION,
    },
    store::{self, AssessmentRow},
    Res, ServiceError,
};
use crate::{
    ai::{extract_json, Task, TextRequest, Thinking},
    ai_worker::PendingJob,
    state::AppState,
    study,
};

/// Cap on answers graded by one job run, so one job cannot hold the worker for long.
const MAX_ANSWERS_PER_RUN: i64 = 400;
/// Answers graded at the same time.
const GRADE_CONCURRENCY: usize = 4;
/// Objective questions independently re-solved per generation.
const MAX_SOLVE_BACK: usize = 12;

pub async fn run_job(
    state: &AppState,
    pool: &PgPool,
    job: &PendingJob,
) -> Result<Option<Uuid>, String> {
    match job.job_type.as_str() {
        "assessment_generate" => run_generate(state, pool, job).await,
        "assessment_grade" => run_grade(state, pool, job).await,
        other => Err(format!("unknown assessment job `{other}`")),
    }
}

// ---- enqueueing -------------------------------------------------------------------------------------

/// Queues question generation. Only one runs per assessment at a time.
pub async fn enqueue_generation(
    pool: &PgPool,
    assessment: &AssessmentRow,
    requested_by: Uuid,
    spec: &GenerationSpecInput,
) -> Res<Uuid> {
    if matches!(assessment.status.as_str(), "finalized" | "archived") {
        return Err(ServiceError::Conflict(
            "This assessment can no longer be edited".to_owned(),
        ));
    }
    let params = json!({
        "assessment_id": assessment.id,
        "count": spec.count.unwrap_or(5).clamp(1, 30),
        "types": spec.types,
        "difficulty_mix": spec.difficulty_mix,
        "topics": spec.topics,
        "chapters": spec.chapters,
        "points": spec.points_per_question.unwrap_or(1.0),
    });
    store::enqueue_job(
        pool,
        assessment.session_id,
        requested_by,
        "assessment_generate",
        params,
        &format!("gen:{}", assessment.id),
    )
    .await?
    .ok_or_else(|| {
        ServiceError::Conflict(
            "Question generation is already running for this assessment".to_owned(),
        )
    })
}

/// Queues AI grading of the answers that have no grade yet. A repeat while
/// one is queued or running is a no-op (`Ok(None)`).
pub async fn enqueue_grading(
    pool: &PgPool,
    assessment: &AssessmentRow,
    requested_by: Uuid,
    question: Option<Uuid>,
) -> Res<Option<Uuid>> {
    Ok(store::enqueue_job(
        pool,
        assessment.session_id,
        requested_by,
        "assessment_grade",
        json!({"assessment_id": assessment.id, "question_id": question}),
        &format!("grade:{}", assessment.id),
    )
    .await?)
}

#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct GenerationSpecInput {
    pub count: Option<usize>,
    #[serde(default)]
    pub types: Vec<String>,
    #[serde(default)]
    pub difficulty_mix: Option<[f64; 3]>,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub chapters: Vec<i32>,
    #[serde(default)]
    pub points_per_question: Option<f64>,
}

fn spec_from_params(params: &Value) -> GenerationSpec {
    let mut spec = GenerationSpec::default();
    if let Some(n) = params.get("count").and_then(Value::as_u64) {
        spec.count = (n as usize).clamp(1, 30);
    }
    let types: Vec<QuestionType> = params
        .get("types")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|v| v.as_str().and_then(QuestionType::parse))
                .collect()
        })
        .unwrap_or_default();
    if !types.is_empty() {
        spec.types = types;
    }
    if let Some([e, m, h]) = params
        .get("difficulty_mix")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_f64).collect::<Vec<_>>())
        .as_deref()
        .and_then(|v| <&[f64; 3]>::try_from(v).ok())
        .copied()
    {
        spec.difficulty_mix = (e, m, h);
    }
    spec.topics = params
        .get("topics")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    spec.chapters = params
        .get("chapters")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_i64().map(|i| i as i32))
                .collect()
        })
        .unwrap_or_default();
    if let Some(p) = params.get("points").and_then(Value::as_f64) {
        spec.points_per_question = p.clamp(0.0, 100.0);
    }
    spec
}

fn assessment_id_of(job: &PendingJob) -> Result<Uuid, String> {
    job.params
        .get("assessment_id")
        .and_then(Value::as_str)
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| "the job has no assessment".to_owned())
}

// ---- generation -------------------------------------------------------------------------------------

/// Keeps only the transcript lines whose `[mm:ss]` falls inside one of `ranges`
/// (inclusive seconds). Lines without a stamp stay with the previous line.
pub fn slice_transcript(text: &str, ranges: &[(i32, i32)]) -> String {
    if ranges.is_empty() {
        return text.to_owned();
    }
    let mut keep = false;
    let mut out = Vec::new();
    for line in text.lines() {
        if let Some(stamp) = study::leading_stamp(line) {
            let seconds = stamp.split_once(':').and_then(|(m, s)| {
                Some(m.trim().parse::<i32>().ok()? * 60 + s.trim().parse::<i32>().ok()?)
            });
            if let Some(sec) = seconds {
                keep = ranges.iter().any(|(a, b)| sec >= *a && sec <= *b);
            }
        }
        if keep {
            out.push(line);
        }
    }
    out.join("\n")
}

async fn run_generate(
    state: &AppState,
    pool: &PgPool,
    job: &PendingJob,
) -> Result<Option<Uuid>, String> {
    if !state.ai.is_configured() {
        return Err("AI is not configured".to_owned());
    }
    let assessment_id = assessment_id_of(job)?;
    let assessment = store::find_assessment(pool, assessment_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("the assessment no longer exists")?;
    if matches!(assessment.status.as_str(), "finalized" | "archived") {
        return Err("the assessment is finalised".to_owned());
    }
    let spec = spec_from_params(&job.params);

    let transcript = study::load_transcript(pool, assessment.session_id)
        .await
        .map_err(|e| e.to_string())?;
    if transcript.text.trim().is_empty() {
        return Err("the lecture has no transcript yet".to_owned());
    }
    let chapters: Vec<(i32, String, i32, i32)> = sqlx::query_as(
        "select chapter_index, title, start_timestamp_sec, end_timestamp_sec
         from session_chapters where session_id = $1 order by chapter_index",
    )
    .bind(assessment.session_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    let ranges: Vec<(i32, i32)> = chapters
        .iter()
        .filter(|c| spec.chapters.contains(&c.0))
        .map(|c| (c.2, c.3))
        .collect();
    let sliced = slice_transcript(&transcript.text, &ranges);
    let limit = state.ai.max_input_chars();
    let text = study::clamp_text(&sliced, limit);
    let title: String = sqlx::query_scalar("select title from lecture_sessions where id = $1")
        .bind(assessment.session_id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

    let request = TextRequest::new(
        Task::Assessment,
        generation_system(&spec),
        generation_input(&spec, &title, &chapters, &text),
    )
    .json()
    .thinking(Thinking::Low)
    .max_tokens(9000)
    .for_job(job.id, job.session_id);
    let allowed = spec.types.clone();
    let duration = transcript.duration_sec;
    let points = spec.points_per_question;
    let ((mut generated, report), output) = state
        .ai
        .generate_validated(&request, |output| {
            let json = extract_json(&output.text)
                .ok_or_else(|| "the answer contained no JSON".to_owned())?;
            let (questions, report) = parse_generated(&json, &allowed, points, duration);
            if questions.is_empty() {
                Err("no valid questions could be read from the answer".to_owned())
            } else {
                Ok((questions, report))
            }
        })
        .await
        .map_err(|e| e.to_string())?;

    // Drop repeats of questions already in the assessment.
    let existing: Vec<String> = store::questions(pool, assessment.id)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|q| q.prompt_md)
        .collect();
    let duplicates = dedupe(&mut generated, &existing);

    // Solve-back: independently answer each objective question from the
    // transcript. A disagreement marks it for the lecturer's attention.
    let mut disagreements = 0;
    for question in generated
        .iter_mut()
        .filter(|q| q.question_type.is_objective())
        .take(MAX_SOLVE_BACK)
    {
        let probe = probe_question(question);
        let request =
            TextRequest::new(Task::Assessment, solve_system(), solve_input(&probe, &text))
                .json()
                .thinking(Thinking::Minimal)
                .max_tokens(200)
                .for_job(job.id, job.session_id);
        let answer = state
            .ai
            .generate(&request)
            .await
            .ok()
            .and_then(|o| extract_json(&o.text));
        if let Some(answer) = answer {
            if !solve_agrees(&probe, &answer) {
                question.low_confidence = true;
                disagreements += 1;
            }
        }
    }

    let added = super::authoring::insert_generated(pool, assessment.id, &generated, job.id)
        .await
        .map_err(|e| e.to_string())?;
    let summary = json!({
        "added": added, "dropped_invalid": report.dropped, "duplicates": duplicates,
        "solve_back_disagreements": disagreements, "model": output.model, "prompt_version": PROMPT_VERSION,
    });
    let _ = sqlx::query("update ai_jobs set params = params || jsonb_build_object('result', $2::jsonb) where id = $1")
        .bind(job.id)
        .bind(summary)
        .execute(pool)
        .await;
    Ok(None)
}

fn probe_question(q: &GeneratedQuestion) -> SnapshotQuestion {
    SnapshotQuestion {
        id: Uuid::nil(),
        position: 0,
        question_type: q.question_type,
        prompt_md: q.prompt_md.clone(),
        points: q.points,
        difficulty: q.difficulty.clone(),
        topic_tag: None,
        chapter_index: None,
        public_payload: q.public_payload.clone(),
        key_payload: q.key_payload.clone(),
        explanation_md: String::new(),
        source_ranges: Value::Null,
    }
}

// ---- grading ---------------------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct PendingAnswer {
    answer_id: Uuid,
    attempt_id: Uuid,
    version_id: Uuid,
    question_id: Uuid,
    response: Value,
}

struct Graded {
    answer_id: Uuid,
    attempt_id: Uuid,
    proposal: prompts::GradeProposal,
    model: String,
}

async fn run_grade(
    state: &AppState,
    pool: &PgPool,
    job: &PendingJob,
) -> Result<Option<Uuid>, String> {
    if !state.ai.is_configured() {
        return Err("AI is not configured".to_owned());
    }
    let assessment_id = assessment_id_of(job)?;
    let assessment = store::find_assessment(pool, assessment_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("the assessment no longer exists")?;
    if assessment.status == "finalized" {
        return Ok(None);
    }
    let settings = assessment.settings();
    if settings.scoring_mode == ScoringMode::Manual
        && job.params.get("force").and_then(Value::as_bool) != Some(true)
    {
        return Ok(None);
    }
    let only_question = job
        .params
        .get("question_id")
        .and_then(Value::as_str)
        .and_then(|s| Uuid::parse_str(s).ok());

    // Answers on submitted attempts that have no grade at all yet.
    let pending = sqlx::query_as::<_, PendingAnswer>(
        "select aa.id as answer_id, aa.attempt_id, at.version_id, aa.question_id, aa.response
         from attempt_answers aa join assessment_attempts at on at.id = aa.attempt_id
         where at.assessment_id = $1 and at.status in ('submitted', 'auto_submitted', 'graded')
           and aa.current_grade_id is null and ($2::uuid is null or aa.question_id = $2)
         order by aa.id limit $3",
    )
    .bind(assessment.id)
    .bind(only_question)
    .bind(MAX_ANSWERS_PER_RUN)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    if pending.is_empty() {
        return Ok(None);
    }
    let mut snapshots = HashMap::new();
    for row in &pending {
        if let std::collections::hash_map::Entry::Vacant(slot) = snapshots.entry(row.version_id) {
            if let Some(s) = store::load_snapshot(pool, row.version_id)
                .await
                .map_err(|e| e.to_string())?
            {
                slot.insert(s);
            }
        }
    }

    let mut work = Vec::new();
    for row in &pending {
        let Some(q) = snapshots
            .get(&row.version_id)
            .and_then(|s| s.question(row.question_id))
        else {
            continue;
        };
        if !may_ai_grade(q, &settings) {
            continue;
        }
        let text = row
            .response
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        if text.trim().is_empty() {
            continue;
        }
        work.push((row, q.clone(), text));
    }

    let mut failures = 0usize;
    let mut graded: Vec<Graded> = Vec::new();
    for chunk in work.chunks(GRADE_CONCURRENCY) {
        let results = join_all(chunk.iter().map(|(row, q, text)| async move {
            let request =
                TextRequest::new(Task::Assessment, grading_system(), grading_input(q, text))
                    .json()
                    .thinking(Thinking::Low)
                    .max_tokens(1500)
                    .for_job(job.id, job.session_id);
            let (proposal, output) = state
                .ai
                .generate_validated(&request, |output| {
                    let json = extract_json(&output.text)
                        .ok_or_else(|| "no JSON in the grade".to_owned())?;
                    parse_grade(&json, q)
                })
                .await
                .map_err(|e| e.to_string())?;
            Ok::<Graded, String>(Graded {
                answer_id: row.answer_id,
                attempt_id: row.attempt_id,
                proposal,
                model: output.model,
            })
        }))
        .await;
        for result in results {
            match result {
                Ok(g) => graded.push(g),
                Err(error) => {
                    failures += 1;
                    tracing::warn!(%error, job = %job.id, "Could not AI-grade an answer");
                }
            }
        }
    }
    if graded.is_empty() && failures > 0 {
        return Err(format!("none of {failures} answers could be graded"));
    }

    let mut attempts = std::collections::HashSet::new();
    for g in graded {
        let text = pending
            .iter()
            .find(|p| p.answer_id == g.answer_id)
            .and_then(|p| p.response.get("text").and_then(Value::as_str))
            .unwrap_or("");
        let mut flag = g.proposal.flag.clone();
        if flag.is_none() && looks_like_injection(text) {
            flag = Some("addresses_grader".to_owned());
        }
        // Confirmed on the spot only when the lecturer chose AI-final, the
        // model was confident, and nothing about the answer looked like an attack.
        let confirm = settings.scoring_mode == ScoringMode::AiAuto
            && flag.is_none()
            && g.proposal.confidence >= 0.5;
        let status = if confirm { "confirmed" } else { "proposed" };
        let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
        // The lecturer may have graded this answer while the model was working.
        let current: Option<Uuid> = sqlx::query_scalar(
            "select current_grade_id from attempt_answers where id = $1 for update",
        )
        .bind(g.answer_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
        if current.is_some() {
            continue;
        }
        let grade_id: Uuid = sqlx::query_scalar(
            "insert into answer_grades (answer_id, source, status, score, max_score, feedback_md, rubric_scores,
                                        confidence, evidence, model, prompt_version, ai_job_id)
             values ($1, 'ai', $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) returning id",
        )
        .bind(g.answer_id)
        .bind(status)
        .bind(g.proposal.score)
        .bind(g.proposal.max)
        .bind(&g.proposal.feedback)
        .bind(Value::Array(g.proposal.rubric_scores.clone()))
        .bind(g.proposal.confidence)
        .bind(json!({"quotes": g.proposal.evidence, "flag": flag}))
        .bind(&g.model)
        .bind(PROMPT_VERSION)
        .bind(job.id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
        sqlx::query("update attempt_answers set current_grade_id = $2 where id = $1")
            .bind(g.answer_id)
            .bind(grade_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
        tx.commit().await.map_err(|e| e.to_string())?;
        attempts.insert(g.attempt_id);
    }
    for id in attempts {
        super::grading::recompute_attempt(pool, id)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(None)
}

/// Whether AI may grade this question: long answers always (the lecturer chose
/// an AI scoring mode to get here); short answers only when enabled, since
/// most are settled by the accepted-answer list.
fn may_ai_grade(question: &SnapshotQuestion, settings: &AssessmentSettings) -> bool {
    match question.question_type {
        QuestionType::LongAnswer => true,
        QuestionType::ShortAnswer => {
            settings.ai_short_answer
                || question
                    .key_payload
                    .get("ai_fallback")
                    .and_then(Value::as_bool)
                    == Some(true)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slices_the_transcript_by_time_range() {
        let text = "[00:10] one\n[00:50] two\ncontinued\n[01:30] three\n[02:30] four";
        assert_eq!(slice_transcript(text, &[]), text);
        let out = slice_transcript(text, &[(40, 100)]);
        assert_eq!(out, "[00:50] two\ncontinued\n[01:30] three");
        assert_eq!(
            slice_transcript(text, &[(0, 20), (150, 160)]),
            "[00:10] one\n[02:30] four"
        );
    }

    #[test]
    fn reads_a_generation_spec_with_safe_defaults() {
        let spec = spec_from_params(
            &json!({"count": 99, "types": ["mcq_single", "bogus"], "difficulty_mix": [1, 0, 3],
                                            "topics": ["heat"], "chapters": [0, 2], "points": 2.5}),
        );
        assert_eq!(spec.count, 30);
        assert_eq!(spec.types, vec![QuestionType::McqSingle]);
        assert_eq!(spec.difficulty_mix, (1.0, 0.0, 3.0));
        assert_eq!(spec.chapters, vec![0, 2]);
        let default = spec_from_params(&json!({}));
        assert_eq!(default.count, 5);
        assert!(!default.types.is_empty());
    }

    #[test]
    fn ai_grading_is_limited_to_the_right_questions() {
        let s = AssessmentSettings::default();
        let mut q = SnapshotQuestion {
            id: Uuid::nil(),
            position: 0,
            question_type: QuestionType::ShortAnswer,
            prompt_md: "p".to_owned(),
            points: 1.0,
            difficulty: String::new(),
            topic_tag: None,
            chapter_index: None,
            public_payload: json!({}),
            key_payload: json!({"accepted": ["x"]}),
            explanation_md: String::new(),
            source_ranges: Value::Null,
        };
        assert!(
            !may_ai_grade(&q, &s),
            "short answers use the accepted list unless AI is enabled"
        );
        q.key_payload = json!({"accepted": ["x"], "ai_fallback": true});
        assert!(may_ai_grade(&q, &s));
        q.question_type = QuestionType::McqSingle;
        assert!(!may_ai_grade(&q, &s));
        q.question_type = QuestionType::LongAnswer;
        assert!(may_ai_grade(&q, &s));
    }
}
