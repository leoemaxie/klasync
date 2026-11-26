//! Grading: the blind queue, lecturer and AI grades (append-only), finalise
//! (which reveals identities) and release. Every grader-facing query selects
//! the attempt's `grading_alias` and never a student column.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    model::{GraderAnswer, QuestionType, Snapshot},
    scoring::{apply_penalty, counted_fraction, total_score, AttemptScore},
    store::{self, AssessmentRow},
    Res, ServiceError,
};

// ---- per-attempt totals ------------------------------------------------------------------------

/// Recomputes an attempt's total from its confirmed grades and moves it to
/// `graded` once every answer has one. Proposals count as pending.
pub async fn recompute_attempt(pool: &PgPool, attempt_id: Uuid) -> Res<()> {
    let attempt = store::find_attempt(pool, attempt_id)
        .await?
        .ok_or(ServiceError::NotFound("Attempt not found"))?;
    if !attempt.is_submitted() {
        return Ok(());
    }
    let snapshot = store::load_snapshot(pool, attempt.version_id)
        .await?
        .ok_or_else(|| ServiceError::Internal("the attempt's version is missing".to_owned()))?;
    let rows: Vec<(Option<String>, Option<f64>)> = sqlx::query_as(
        "select g.status, g.score from attempt_answers aa
         left join answer_grades g on g.id = aa.current_grade_id where aa.attempt_id = $1",
    )
    .bind(attempt_id)
    .fetch_all(pool)
    .await?;
    let mut scores = Vec::new();
    let mut pending = 0;
    for (status, score) in &rows {
        match (status.as_deref(), score) {
            (Some("confirmed" | "overridden"), Some(s)) => scores.push(*s),
            _ => pending += 1,
        }
    }
    let penalty = attempt
        .integrity_summary
        .get("late_penalty_pct")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let total = apply_penalty(total_score(&scores), penalty);
    let status = if pending == 0 {
        "graded"
    } else if attempt.status == "graded" {
        "submitted"
    } else {
        attempt.status.as_str()
    };
    sqlx::query(
        "update assessment_attempts set final_score = $2, max_score = $3, status = $4 where id = $1",
    )
    .bind(attempt_id)
    .bind(total)
    .bind(snapshot.max_score())
    .bind(status)
    .execute(pool)
    .await?;
    Ok(())
}

// ---- the blind queue ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct GradingQueue {
    pub answers: Vec<GraderAnswer>,
    /// Non-objective answers still waiting for a confirmed grade.
    pub pending: usize,
    pub total: usize,
    /// The questions that need a person or AI, so the workbench can step through them.
    pub questions: Vec<QueueQuestion>,
}

#[derive(Debug, Serialize)]
pub struct QueueQuestion {
    pub id: Uuid,
    pub position: i32,
    pub question_type: QuestionType,
    pub prompt_md: String,
    pub points: f64,
    pub rubric: Value,
    pub reference: Value,
    pub pending: usize,
}

#[derive(sqlx::FromRow)]
struct QueueRow {
    answer_id: Uuid,
    version_id: Uuid,
    grading_alias: String,
    question_id: Uuid,
    response: Value,
    score: Option<f64>,
    grade_status: Option<String>,
    grade_source: Option<String>,
    feedback_md: Option<String>,
    rubric_scores: Option<Value>,
    confidence: Option<f64>,
    evidence: Option<Value>,
}

/// Answers to grade, with aliases only. `question` narrows to one question
/// (grading question by question reduces drift); `pending_only` hides
/// answers that already have a confirmed grade.
pub async fn queue(
    pool: &PgPool,
    assessment: &AssessmentRow,
    question: Option<Uuid>,
    pending_only: bool,
) -> Res<GradingQueue> {
    let rows = sqlx::query_as::<_, QueueRow>(
        "select aa.id as answer_id, at.version_id, at.grading_alias, aa.question_id, aa.response,
                g.score, g.status as grade_status, g.source as grade_source, g.feedback_md,
                g.rubric_scores, g.confidence, g.evidence
         from attempt_answers aa
         join assessment_attempts at on at.id = aa.attempt_id
         left join answer_grades g on g.id = aa.current_grade_id
         where at.assessment_id = $1 and at.status in ('submitted', 'auto_submitted', 'graded')
         order by at.grading_alias, aa.question_id",
    )
    .bind(assessment.id)
    .fetch_all(pool)
    .await?;

    let mut snapshots: HashMap<Uuid, Snapshot> = HashMap::new();
    for row in &rows {
        if let std::collections::hash_map::Entry::Vacant(slot) = snapshots.entry(row.version_id) {
            if let Some(s) = store::load_snapshot(pool, row.version_id).await? {
                slot.insert(s);
            }
        }
    }
    let mut answers = Vec::new();
    let mut per_question: HashMap<Uuid, usize> = HashMap::new();
    let mut total = 0;
    let mut pending = 0;
    for row in rows {
        let Some(snapshot) = snapshots.get(&row.version_id) else {
            continue;
        };
        let Some(q) = snapshot.question(row.question_id) else {
            continue;
        };
        if q.question_type.is_objective() {
            continue;
        }
        total += 1;
        let confirmed = matches!(
            row.grade_status.as_deref(),
            Some("confirmed" | "overridden")
        );
        if !confirmed {
            pending += 1;
            *per_question.entry(q.id).or_default() += 1;
        }
        if question.is_some_and(|id| id != q.id) || (pending_only && confirmed) {
            continue;
        }
        let proposal = (row.grade_status.as_deref() == Some("proposed")).then(|| {
            json!({
                "score": row.score, "feedback_md": row.feedback_md, "confidence": row.confidence,
                "rubric_scores": row.rubric_scores, "evidence": row.evidence, "source": row.grade_source,
            })
        });
        answers.push(GraderAnswer {
            answer_id: row.answer_id,
            alias: row.grading_alias,
            question_id: q.id,
            response: row.response,
            score: if confirmed { row.score } else { None },
            max_score: q.points,
            grade_source: if confirmed { row.grade_source } else { None },
            grade_status: row.grade_status,
            feedback_md: if confirmed { row.feedback_md } else { None },
            rubric_scores: row.rubric_scores.unwrap_or(json!([])),
            confidence: row.confidence,
            proposal,
        });
    }
    let latest = match assessment.published_version_id {
        Some(id) => store::load_snapshot(pool, id).await?,
        None => None,
    };
    let questions = latest
        .map(|s| {
            s.questions
                .iter()
                .filter(|q| !q.question_type.is_objective())
                .map(|q| QueueQuestion {
                    id: q.id,
                    position: q.position,
                    question_type: q.question_type,
                    prompt_md: q.prompt_md.clone(),
                    points: q.points,
                    rubric: q.key_payload.get("rubric").cloned().unwrap_or(json!([])),
                    reference: q
                        .key_payload
                        .get("reference")
                        .cloned()
                        .or_else(|| q.key_payload.get("accepted").cloned())
                        .unwrap_or(Value::Null),
                    pending: per_question.get(&q.id).copied().unwrap_or(0),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(GradingQueue {
        answers,
        pending,
        total,
        questions,
    })
}

// ---- lecturer grades -----------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct GradeInput {
    pub score: f64,
    #[serde(default)]
    pub feedback_md: String,
    #[serde(default)]
    pub rubric_scores: Option<Value>,
}

fn ensure_gradable(assessment: &AssessmentRow) -> Res<()> {
    if matches!(assessment.status.as_str(), "published" | "closed") {
        Ok(())
    } else if assessment.status == "finalized" {
        Err(ServiceError::Conflict(
            "Grades are locked: this assessment is finalised".to_owned(),
        ))
    } else {
        Err(ServiceError::Conflict(
            "Publish the assessment before grading it".to_owned(),
        ))
    }
}

struct AnswerRef {
    attempt_id: Uuid,
    version_id: Uuid,
    question_id: Uuid,
    current_grade_id: Option<Uuid>,
}

async fn answer_ref(pool: &PgPool, assessment: &AssessmentRow, answer_id: Uuid) -> Res<AnswerRef> {
    let row: Option<(Uuid, Uuid, Uuid, Option<Uuid>)> = sqlx::query_as(
        "select aa.attempt_id, at.version_id, aa.question_id, aa.current_grade_id
         from attempt_answers aa join assessment_attempts at on at.id = aa.attempt_id
         where aa.id = $1 and at.assessment_id = $2 and at.status in ('submitted', 'auto_submitted', 'graded')",
    )
    .bind(answer_id)
    .bind(assessment.id)
    .fetch_optional(pool)
    .await?;
    let (attempt_id, version_id, question_id, current_grade_id) =
        row.ok_or(ServiceError::NotFound("Answer not found"))?;
    Ok(AnswerRef {
        attempt_id,
        version_id,
        question_id,
        current_grade_id,
    })
}

/// A lecturer's score. It supersedes whatever grade was current: a confirmed
/// one becomes `overridden`, an AI proposal `rejected`. Nothing is deleted.
pub async fn set_grade(
    pool: &PgPool,
    lecturer_id: Uuid,
    assessment: &AssessmentRow,
    answer_id: Uuid,
    input: GradeInput,
) -> Res<()> {
    ensure_gradable(assessment)?;
    let target = answer_ref(pool, assessment, answer_id).await?;
    let snapshot = store::load_snapshot(pool, target.version_id)
        .await?
        .ok_or_else(|| ServiceError::Internal("the attempt's version is missing".to_owned()))?;
    let question = snapshot
        .question(target.question_id)
        .ok_or(ServiceError::NotFound("Question not found"))?;
    if !input.score.is_finite() || input.score < 0.0 || input.score > question.points + 1e-9 {
        return Err(ServiceError::Invalid(format!(
            "The score must be between 0 and {}",
            question.points
        )));
    }
    let score = (input.score * 100.0).round() / 100.0;
    let mut tx = pool.begin().await?;
    if let Some(previous) = target.current_grade_id {
        sqlx::query(
            "update answer_grades set status = case when status = 'confirmed' then 'overridden' else 'rejected' end
             where id = $1 and status in ('confirmed', 'proposed')",
        )
        .bind(previous)
        .execute(&mut *tx)
        .await?;
    }
    let grade_id: Uuid = sqlx::query_scalar(
        "insert into answer_grades (answer_id, source, status, score, max_score, feedback_md, rubric_scores, created_by)
         values ($1, 'lecturer', 'confirmed', $2, $3, $4, $5, $6) returning id",
    )
    .bind(answer_id)
    .bind(score)
    .bind(question.points)
    .bind(input.feedback_md.trim())
    .bind(input.rubric_scores.unwrap_or(json!([])))
    .bind(lecturer_id)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query("update attempt_answers set current_grade_id = $2 where id = $1")
        .bind(answer_id)
        .bind(grade_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    recompute_attempt(pool, target.attempt_id).await
}

/// Accepts an AI proposal as it stands.
pub async fn confirm_proposal(
    pool: &PgPool,
    assessment: &AssessmentRow,
    answer_id: Uuid,
) -> Res<()> {
    ensure_gradable(assessment)?;
    let target = answer_ref(pool, assessment, answer_id).await?;
    let done = sqlx::query(
        "update answer_grades set status = 'confirmed' where id = $1 and status = 'proposed'",
    )
    .bind(target.current_grade_id)
    .execute(pool)
    .await?;
    if done.rows_affected() == 0 {
        return Err(ServiceError::Conflict(
            "There is no proposal to confirm for this answer".to_owned(),
        ));
    }
    recompute_attempt(pool, target.attempt_id).await
}

/// Accepts every AI proposal at or above a confidence. Returns how many.
pub async fn bulk_confirm(
    pool: &PgPool,
    assessment: &AssessmentRow,
    min_confidence: f64,
) -> Res<usize> {
    ensure_gradable(assessment)?;
    let attempts: Vec<Uuid> = sqlx::query_scalar(
        "with updated as (
           update answer_grades g set status = 'confirmed'
           from attempt_answers aa join assessment_attempts at on at.id = aa.attempt_id
           where g.id = aa.current_grade_id and g.status = 'proposed' and g.source = 'ai'
             and coalesce(g.confidence, 0) >= $2 and at.assessment_id = $1
             and coalesce(g.evidence->>'flag', '') = ''
           returning aa.attempt_id)
         select distinct attempt_id from updated",
    )
    .bind(assessment.id)
    .bind(min_confidence.clamp(0.0, 1.0))
    .fetch_all(pool)
    .await?;
    let count = attempts.len();
    for id in attempts {
        recompute_attempt(pool, id).await?;
    }
    Ok(count)
}

// ---- finalise and release ----------------------------------------------------------------------

/// How many non-confirmed answers remain on submitted attempts.
pub async fn pending_grades(pool: &PgPool, assessment_id: Uuid) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "select count(*) from attempt_answers aa
         join assessment_attempts at on at.id = aa.attempt_id
         left join answer_grades g on g.id = aa.current_grade_id
         where at.assessment_id = $1 and at.status in ('submitted', 'auto_submitted', 'graded')
           and coalesce(g.status, 'none') not in ('confirmed', 'overridden')",
    )
    .bind(assessment_id)
    .fetch_one(pool)
    .await
}

/// Locks every grade and reveals identities to the lecturer. Irreversible.
pub async fn finalize(pool: &PgPool, assessment: &AssessmentRow, lecturer_id: Uuid) -> Res<()> {
    if assessment.status != "closed" {
        return Err(ServiceError::Conflict(
            "Close the assessment before finalising it".to_owned(),
        ));
    }
    let pending = pending_grades(pool, assessment.id).await?;
    if pending > 0 {
        return Err(ServiceError::Conflict(format!(
            "{pending} answer{} still need a confirmed grade",
            if pending == 1 { "" } else { "s" }
        )));
    }
    let mut tx = pool.begin().await?;
    let done = sqlx::query(
        "update assessments set status = 'finalized', identity_revealed_at = now(), updated_at = now()
         where id = $1 and status = 'closed'",
    )
    .bind(assessment.id)
    .execute(&mut *tx)
    .await?;
    if done.rows_affected() == 0 {
        return Err(ServiceError::Conflict(
            "The assessment was already finalised".to_owned(),
        ));
    }
    store::emit(
        &mut *tx,
        "assessment.finalized",
        json!({"assessment_id": assessment.id, "course_id": assessment.course_id, "by": lecturer_id}),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn set_release(
    pool: &PgPool,
    assessment: &AssessmentRow,
    release: bool,
) -> Res<Option<DateTime<Utc>>> {
    if !matches!(
        assessment.status.as_str(),
        "published" | "closed" | "finalized"
    ) {
        return Err(ServiceError::Conflict(
            "Publish the assessment before releasing results".to_owned(),
        ));
    }
    let at: Option<DateTime<Utc>> = sqlx::query_scalar(
        "update assessments set results_released_at = case when $2 then now() else null end, updated_at = now()
         where id = $1 returning results_released_at",
    )
    .bind(assessment.id)
    .bind(release)
    .fetch_one(pool)
    .await?;
    if release {
        store::emit(
            pool,
            "results.released",
            json!({"assessment_id": assessment.id, "course_id": assessment.course_id}),
        )
        .await?;
    }
    Ok(at)
}

// ---- named results (after the reveal only) ---------------------------------------------------

#[derive(Debug, Serialize)]
pub struct StudentScoreRow {
    pub student_account_id: Uuid,
    pub display_name: String,
    pub matric_number: String,
    pub attempts: usize,
    /// The score that counts under the keep policy, as a percentage.
    pub percent: Option<f64>,
    pub late: bool,
}

/// Per-student results. Refused until the lecturer has finalised: this is the
/// only place a score is joined to a name.
pub async fn named_results(pool: &PgPool, assessment: &AssessmentRow) -> Res<Vec<StudentScoreRow>> {
    if assessment.identity_revealed_at.is_none() {
        return Err(ServiceError::Forbidden(
            "Names are shown only after you finalise the assessment",
        ));
    }
    let keep = assessment.settings().keep;
    let rows = sqlx::query_as::<_, (Uuid, String, String, i32, Option<f64>, Option<f64>, bool)>(
        "select s.id, s.display_name, s.matric_number, a.attempt_no, a.final_score, a.max_score, a.late
         from assessment_attempts a join student_accounts s on s.id = a.student_account_id
         where a.assessment_id = $1 and a.status = 'graded' order by s.display_name, a.attempt_no",
    )
    .bind(assessment.id)
    .fetch_all(pool)
    .await?;
    let mut by_student: Vec<(Uuid, String, String, Vec<AttemptScore>, bool)> = Vec::new();
    for (id, name, matric, no, score, max, late) in rows {
        let entry = AttemptScore {
            attempt_no: no,
            score: score.unwrap_or(0.0),
            max: max.unwrap_or(0.0),
        };
        match by_student.iter_mut().find(|s| s.0 == id) {
            Some(existing) => {
                existing.3.push(entry);
                existing.4 |= late;
            }
            None => by_student.push((id, name, matric, vec![entry], late)),
        }
    }
    Ok(by_student
        .into_iter()
        .map(|(id, name, matric, attempts, late)| StudentScoreRow {
            student_account_id: id,
            display_name: name,
            matric_number: matric,
            attempts: attempts.len(),
            percent: counted_fraction(&attempts, keep).map(|f| (f * 1000.0).round() / 10.0),
            late,
        })
        .collect())
}
