//! Loads class-level reports. Everything returned is aggregated: no student
//! id, name or matric number, so it is safe to show before identities are
//! revealed. Named per-student results live in `grading::named_results`.

use std::collections::HashMap;

use serde::Serialize;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    model::{string_list, QuestionType, Snapshot},
    report::{
        histogram, item_analysis, summarise, topic_mastery, AnswerRow, Bin, Funnel, ItemStat,
        QuestionInfo, Summary, TopicStat,
    },
    scoring::{counted_fraction, is_answered, AttemptScore},
    store::{self, AssessmentRow},
    Res, ServiceError,
};

const PASS_MARK: f64 = 0.5;

fn question_infos(snapshot: &Snapshot) -> Vec<QuestionInfo> {
    snapshot
        .questions
        .iter()
        .map(|q| QuestionInfo {
            id: q.id,
            position: q.position as usize,
            prompt: q.prompt_md.clone(),
            topic_tag: q.topic_tag.clone(),
            chapter_index: q.chapter_index,
            is_choice: matches!(
                q.question_type,
                QuestionType::McqSingle | QuestionType::McqMulti | QuestionType::TrueFalse
            ),
            correct_options: match q.question_type {
                QuestionType::TrueFalse => vec![q
                    .key_payload
                    .get("correct")
                    .and_then(Value::as_bool)
                    .map_or("true", |b| if b { "true" } else { "false" })
                    .to_owned()],
                _ => string_list(q.key_payload.get("correct")),
            },
            option_ids: match q.question_type {
                QuestionType::TrueFalse => vec!["true".to_owned(), "false".to_owned()],
                QuestionType::McqSingle | QuestionType::McqMulti => {
                    super::model::declared_options(&q.public_payload)
                        .into_iter()
                        .map(|o| o.id)
                        .collect()
                }
                _ => Vec::new(),
            },
            points: q.points,
        })
        .collect()
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub assessment_id: Uuid,
    /// True until the lecturer finalises: unconfirmed grades are not counted yet.
    pub provisional: bool,
    pub pending_grades: i64,
    pub funnel: Funnel,
    pub summary: Option<Summary>,
    pub histogram: Vec<Bin>,
    pub items: Vec<ItemStat>,
    pub topics: Vec<TopicStat>,
    pub integrity: IntegritySummary,
}

#[derive(Debug, Default, Serialize)]
pub struct IntegritySummary {
    pub attempts_with_flags: i64,
    pub offline_submits: i64,
    pub late_submits: i64,
}

pub async fn build(pool: &PgPool, assessment: &AssessmentRow) -> Res<Report> {
    let version_id = assessment
        .published_version_id
        .ok_or(ServiceError::Conflict(
            "Publish the assessment to see a report".to_owned(),
        ))?;
    let snapshot = store::load_snapshot(pool, version_id)
        .await?
        .ok_or_else(|| ServiceError::Internal("the published version is missing".to_owned()))?;

    let questions = question_infos(&snapshot);
    let points: HashMap<Uuid, f64> = snapshot
        .questions
        .iter()
        .map(|q| (q.id, q.points))
        .collect();
    let kinds: HashMap<Uuid, QuestionType> = snapshot
        .questions
        .iter()
        .map(|q| (q.id, q.question_type))
        .collect();

    // Attempts that were submitted. The student id is used only to pick the
    // counted attempt and never leaves this function.
    let attempts = sqlx::query_as::<_, (Uuid, Uuid, i32, String, Option<f64>, Option<f64>, bool, bool, Value)>(
        "select id, student_account_id, attempt_no, status, final_score, max_score, late, submitted_offline, integrity_summary
         from assessment_attempts
         where assessment_id = $1 and status in ('submitted', 'auto_submitted', 'graded')",
    )
    .bind(assessment.id)
    .fetch_all(pool)
    .await?;

    let enrolled: i64 =
        sqlx::query_scalar("select count(*) from student_course_enrollments where course_id = $1")
            .bind(assessment.course_id)
            .fetch_one(pool)
            .await?;
    let started: i64 = sqlx::query_scalar(
        "select count(distinct student_account_id) from assessment_attempts where assessment_id = $1",
    )
    .bind(assessment.id)
    .fetch_one(pool)
    .await?;
    let mut students: HashMap<Uuid, Vec<AttemptScore>> = HashMap::new();
    let mut graded_students = std::collections::HashSet::new();
    let mut integrity = IntegritySummary::default();
    for (_, student, no, status, score, max, late, offline, summary) in &attempts {
        if status == "graded" {
            graded_students.insert(*student);
            students.entry(*student).or_default().push(AttemptScore {
                attempt_no: *no,
                score: score.unwrap_or(0.0),
                max: max.unwrap_or(0.0),
            });
        }
        if *late {
            integrity.late_submits += 1;
        }
        if *offline {
            integrity.offline_submits += 1;
        }
        if summary
            .get("violations")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            > 0
        {
            integrity.attempts_with_flags += 1;
        }
    }
    let keep = assessment.settings().keep;
    let fractions: Vec<f64> = students
        .values()
        .filter_map(|a| counted_fraction(a, keep))
        .collect();

    // Answers with a confirmed grade, for item analysis.
    let attempt_ids: Vec<Uuid> = attempts.iter().map(|a| a.0).collect();
    let rows = sqlx::query_as::<_, (Uuid, Uuid, Value, i32, Option<f64>, Option<String>)>(
        "select aa.attempt_id, aa.question_id, aa.response, aa.time_spent_ms, g.score, g.status
         from attempt_answers aa left join answer_grades g on g.id = aa.current_grade_id
         where aa.attempt_id = any($1)",
    )
    .bind(&attempt_ids)
    .fetch_all(pool)
    .await?;
    let mut answer_rows = Vec::new();
    let mut totals_sum: HashMap<Uuid, f64> = HashMap::new();
    for (attempt, question, response, time_ms, score, status) in rows {
        let confirmed = matches!(status.as_deref(), Some("confirmed" | "overridden"));
        let max = points.get(&question).copied().unwrap_or(0.0);
        let kind = kinds
            .get(&question)
            .copied()
            .unwrap_or(QuestionType::ShortAnswer);
        let score = if confirmed { score.unwrap_or(0.0) } else { 0.0 };
        let selected = match kind {
            QuestionType::TrueFalse => response
                .get("value")
                .and_then(Value::as_bool)
                .map(|b| vec![if b { "true" } else { "false" }.to_owned()])
                .unwrap_or_default(),
            _ => string_list(response.get("selected")),
        };
        *totals_sum.entry(attempt).or_default() += score;
        answer_rows.push(AnswerRow {
            attempt,
            question,
            score,
            max,
            time_spent_ms: i64::from(time_ms),
            answered: is_answered(kind, &response),
            selected,
        });
    }
    let max_total = snapshot.max_score().max(f64::EPSILON);
    let totals: HashMap<Uuid, f64> = totals_sum
        .into_iter()
        .map(|(attempt, sum)| (attempt, (sum.max(0.0) / max_total).min(1.0)))
        .collect();

    Ok(Report {
        assessment_id: assessment.id,
        provisional: assessment.status != "finalized",
        pending_grades: super::grading::pending_grades(pool, assessment.id).await?,
        funnel: Funnel {
            assigned: enrolled as usize,
            started: started as usize,
            submitted: attempts
                .iter()
                .map(|a| a.1)
                .collect::<std::collections::HashSet<_>>()
                .len(),
            graded: graded_students.len(),
        },
        summary: summarise(&fractions, PASS_MARK),
        histogram: histogram(&fractions, 10),
        items: item_analysis(&questions, &answer_rows, &totals),
        topics: topic_mastery(&questions, &answer_rows),
        integrity,
    })
}

// ---- a student's own progress ------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ProgressEntry {
    pub assessment_id: Uuid,
    pub title: String,
    pub kind: String,
    pub submitted_at: Option<chrono::DateTime<chrono::Utc>>,
    /// The score that counts under the assessment's keep policy, as a percentage.
    pub percent: Option<f64>,
    pub attempts: usize,
    pub late: bool,
}

#[derive(Debug, Serialize)]
pub struct StudentProgress {
    pub assessments: Vec<ProgressEntry>,
    /// Mastery by topic across the released, fully graded assessments, weakest first.
    pub topics: Vec<TopicStat>,
}

/// A student's results across a course. Only assessments whose results have
/// been released to students are included.
pub async fn student_progress(
    pool: &PgPool,
    student_id: Uuid,
    course_id: Uuid,
    now: chrono::DateTime<chrono::Utc>,
) -> Res<StudentProgress> {
    let assessments = sqlx::query_as::<_, AssessmentRow>(&format!(
        "select {} from assessments where course_id = $1 and status in ('published', 'closed', 'finalized')
         order by coalesce(due_at, created_at)",
        store::ASSESSMENT_COLS
    ))
    .bind(course_id)
    .fetch_all(pool)
    .await?;
    let mut entries = Vec::new();
    let mut all_questions: Vec<QuestionInfo> = Vec::new();
    let mut all_rows: Vec<AnswerRow> = Vec::new();
    for assessment in assessments {
        let settings = assessment.settings();
        if !super::attempts::results_released(
            &assessment.status,
            assessment.results_released_at,
            assessment.due_at,
            &settings,
            now,
        ) {
            continue;
        }
        let attempts = sqlx::query_as::<_, (Uuid, i32, Option<f64>, Option<f64>, bool, Option<chrono::DateTime<chrono::Utc>>)>(
            "select id, attempt_no, final_score, max_score, late, submitted_at from assessment_attempts
             where assessment_id = $1 and student_account_id = $2 and status = 'graded' order by attempt_no",
        )
        .bind(assessment.id)
        .bind(student_id)
        .fetch_all(pool)
        .await?;
        if attempts.is_empty() {
            continue;
        }
        let scores: Vec<AttemptScore> = attempts
            .iter()
            .map(|a| AttemptScore {
                attempt_no: a.1,
                score: a.2.unwrap_or(0.0),
                max: a.3.unwrap_or(0.0),
            })
            .collect();
        entries.push(ProgressEntry {
            assessment_id: assessment.id,
            title: assessment.title.clone(),
            kind: assessment.kind.clone(),
            submitted_at: attempts.last().and_then(|a| a.5),
            percent: counted_fraction(&scores, settings.keep).map(|f| (f * 1000.0).round() / 10.0),
            attempts: attempts.len(),
            late: attempts.iter().any(|a| a.4),
        });
        let Some(version_id) = assessment.published_version_id else {
            continue;
        };
        let Some(snapshot) = store::load_snapshot(pool, version_id).await? else {
            continue;
        };
        let points: HashMap<Uuid, f64> = snapshot
            .questions
            .iter()
            .map(|q| (q.id, q.points))
            .collect();
        all_questions.extend(question_infos(&snapshot));
        let ids: Vec<Uuid> = attempts.iter().map(|a| a.0).collect();
        let rows = sqlx::query_as::<_, (Uuid, Uuid, Option<f64>)>(
            "select aa.attempt_id, aa.question_id, g.score from attempt_answers aa
             join answer_grades g on g.id = aa.current_grade_id
             where aa.attempt_id = any($1) and g.status in ('confirmed', 'overridden')",
        )
        .bind(&ids)
        .fetch_all(pool)
        .await?;
        for (attempt, question, score) in rows {
            all_rows.push(AnswerRow {
                attempt,
                question,
                score: score.unwrap_or(0.0).max(0.0),
                max: points.get(&question).copied().unwrap_or(0.0),
                time_spent_ms: 0,
                answered: true,
                selected: Vec::new(),
            });
        }
    }
    Ok(StudentProgress {
        assessments: entries,
        topics: topic_mastery(&all_questions, &all_rows),
    })
}
