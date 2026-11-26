//! SQL access shared by the assessment services: row types and finders.
//! Numeric columns are selected as `float8` so they bind to `f64`.

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::{FromRow, PgExecutor, PgPool};
use uuid::Uuid;

use super::{
    delivery::{Accommodation, Window},
    model::{AssessmentSettings, Snapshot},
};

pub const ASSESSMENT_COLS: &str = "id, course_id, session_id, lecturer_id, title, instructions_md, kind, status, \
     settings, draft_rev, published_version_id, opens_at, due_at, closes_at, results_released_at, \
     identity_revealed_at, created_at, updated_at";

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct AssessmentRow {
    pub id: Uuid,
    pub course_id: Uuid,
    pub session_id: Uuid,
    pub lecturer_id: Uuid,
    pub title: String,
    pub instructions_md: String,
    pub kind: String,
    pub status: String,
    pub settings: Value,
    pub draft_rev: i32,
    pub published_version_id: Option<Uuid>,
    pub opens_at: Option<DateTime<Utc>>,
    pub due_at: Option<DateTime<Utc>>,
    pub closes_at: Option<DateTime<Utc>>,
    pub results_released_at: Option<DateTime<Utc>>,
    pub identity_revealed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AssessmentRow {
    pub fn settings(&self) -> AssessmentSettings {
        AssessmentSettings::from_value(&self.settings)
    }

    pub fn window(&self) -> Window {
        Window {
            opens_at: self.opens_at,
            due_at: self.due_at,
            closes_at: self.closes_at,
        }
    }
}

pub const QUESTION_COLS: &str = "id, assessment_id, position, question_type, prompt_md, points::float8 as points, \
     difficulty, topic_tag, chapter_index, public_payload, key_payload, explanation_md, origin, \
     source_ranges, low_confidence, locked";

/// A draft question as the **lecturer** sees it (keys included).
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct QuestionRow {
    pub id: Uuid,
    pub assessment_id: Uuid,
    pub position: i32,
    pub question_type: String,
    pub prompt_md: String,
    pub points: f64,
    pub difficulty: String,
    pub topic_tag: Option<String>,
    pub chapter_index: Option<i32>,
    pub public_payload: Value,
    pub key_payload: Value,
    pub explanation_md: String,
    pub origin: String,
    pub source_ranges: Value,
    pub low_confidence: bool,
    pub locked: bool,
}

pub const ATTEMPT_COLS: &str = "id, assessment_id, version_id, student_account_id, attempt_no, status, grading_alias, \
     started_at, deadline_at, submitted_at, submitted_offline, claimed_submit_at, late, question_order, \
     option_orders, seed, device_id, integrity_summary, final_score, max_score, receipt";

#[derive(Debug, Clone, FromRow)]
pub struct AttemptRow {
    pub id: Uuid,
    pub assessment_id: Uuid,
    pub version_id: Uuid,
    pub student_account_id: Uuid,
    pub attempt_no: i32,
    pub status: String,
    pub grading_alias: String,
    pub started_at: DateTime<Utc>,
    pub deadline_at: Option<DateTime<Utc>>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub submitted_offline: bool,
    pub claimed_submit_at: Option<DateTime<Utc>>,
    pub late: bool,
    pub question_order: Value,
    pub option_orders: Value,
    pub seed: i64,
    pub device_id: Option<String>,
    pub integrity_summary: Value,
    pub final_score: Option<f64>,
    pub max_score: Option<f64>,
    pub receipt: Option<String>,
}

impl AttemptRow {
    pub fn is_open(&self) -> bool {
        self.status == "in_progress"
    }

    pub fn is_submitted(&self) -> bool {
        matches!(self.status.as_str(), "submitted" | "auto_submitted" | "graded")
    }
}

pub async fn find_assessment(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<AssessmentRow>, sqlx::Error> {
    sqlx::query_as::<_, AssessmentRow>(&format!(
        "select {ASSESSMENT_COLS} from assessments where id = $1"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn find_attempt(pool: &PgPool, id: Uuid) -> Result<Option<AttemptRow>, sqlx::Error> {
    sqlx::query_as::<_, AttemptRow>(&format!(
        "select {ATTEMPT_COLS} from assessment_attempts where id = $1"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn questions(
    pool: &PgPool,
    assessment_id: Uuid,
) -> Result<Vec<QuestionRow>, sqlx::Error> {
    sqlx::query_as::<_, QuestionRow>(&format!(
        "select {QUESTION_COLS} from assessment_questions where assessment_id = $1 order by position, created_at"
    ))
    .bind(assessment_id)
    .fetch_all(pool)
    .await
}

pub async fn find_question(
    pool: &PgPool,
    assessment_id: Uuid,
    id: Uuid,
) -> Result<Option<QuestionRow>, sqlx::Error> {
    sqlx::query_as::<_, QuestionRow>(&format!(
        "select {QUESTION_COLS} from assessment_questions where assessment_id = $1 and id = $2"
    ))
    .bind(assessment_id)
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn load_snapshot(pool: &PgPool, version_id: Uuid) -> Result<Option<Snapshot>, sqlx::Error> {
    let value: Option<Value> =
        sqlx::query_scalar("select snapshot from assessment_versions where id = $1")
            .bind(version_id)
            .fetch_optional(pool)
            .await?;
    Ok(value.and_then(|v| serde_json::from_value(v).ok()))
}

pub async fn accommodation(
    pool: &PgPool,
    assessment_id: Uuid,
    student_id: Uuid,
) -> Result<Accommodation, sqlx::Error> {
    let row: Option<(i32, i32, Option<DateTime<Utc>>)> = sqlx::query_as(
        "select extra_time_pct, extra_attempts, due_override from assessment_accommodations
         where assessment_id = $1 and student_account_id = $2",
    )
    .bind(assessment_id)
    .bind(student_id)
    .fetch_optional(pool)
    .await?;
    Ok(row
        .map(|(extra_time_pct, extra_attempts, due_override)| Accommodation {
            extra_time_pct,
            extra_attempts,
            due_override,
        })
        .unwrap_or_default())
}

/// Writes an event; call it on the same transaction as the change.
pub async fn emit<'e>(
    executor: impl PgExecutor<'e>,
    event_type: &str,
    payload: Value,
) -> Result<(), sqlx::Error> {
    sqlx::query("insert into assessment_events (event_type, payload) values ($1, $2)")
        .bind(event_type)
        .bind(payload)
        .execute(executor)
        .await?;
    Ok(())
}

/// Queues an assessment job. `dedupe_key` makes a repeat while one is queued or
/// running a no-op. Returns the job id when it was newly queued.
pub async fn enqueue_job(
    pool: &PgPool,
    session_id: Uuid,
    requested_by: Uuid,
    job_type: &str,
    params: Value,
    dedupe_key: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let id = Uuid::now_v7();
    let inserted = sqlx::query(
        "insert into ai_jobs (id, session_id, requested_by, job_type, mode, params)
         select $1, $2, $3, $4, 'interactive', jsonb_set($5::jsonb, '{dedupe}', to_jsonb($6::text))
         where not exists (
           select 1 from ai_jobs j
           where j.job_type = $4 and j.params->>'dedupe' = $6 and j.status in ('queued', 'running'))",
    )
    .bind(id)
    .bind(session_id)
    .bind(requested_by)
    .bind(job_type)
    .bind(&params)
    .bind(dedupe_key)
    .execute(pool)
    .await?;
    Ok((inserted.rows_affected() > 0).then_some(id))
}
