//! Lecturer-side lifecycle: create, edit, questions, publish (versioned),
//! close, accommodations and the roster view. Grading, finalise and release
//! live in `grading.rs`; student attempts in `attempts.rs`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    attempts,
    model::{
        blank_payloads, validate_question, AssessmentSettings, QuestionType, Snapshot,
        SnapshotQuestion,
    },
    prompts::GeneratedQuestion,
    store::{self, AssessmentRow, QuestionRow},
    Res, ServiceError,
};
use crate::materials::store::SessionInfo;

/// Distinguishes a missing field from an explicit `null` (which clears a date).
pub fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Ok(Some(Option::deserialize(deserializer)?))
}

#[derive(Debug, Deserialize)]
pub struct NewAssessment {
    pub title: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub instructions_md: Option<String>,
}

pub async fn create(
    pool: &PgPool,
    session: &SessionInfo,
    lecturer_id: Uuid,
    input: NewAssessment,
) -> Res<AssessmentRow> {
    let title = input.title.trim();
    if title.is_empty() || title.chars().count() > 200 {
        return Err(ServiceError::Invalid(
            "Give the assessment a title (up to 200 characters)".to_owned(),
        ));
    }
    let kind = input.kind.as_deref().unwrap_or("quiz");
    if !matches!(kind, "quiz" | "homework" | "practice") {
        return Err(ServiceError::Invalid(
            "Kind must be quiz, homework or practice".to_owned(),
        ));
    }
    let settings = serde_json::to_value(AssessmentSettings::default())
        .map_err(|e| ServiceError::Internal(e.to_string()))?;
    let mut tx = pool.begin().await?;
    let row = sqlx::query_as::<_, AssessmentRow>(&format!(
        "insert into assessments (course_id, session_id, lecturer_id, title, instructions_md, kind, settings)
         values ($1, $2, $3, $4, $5, $6, $7) returning {}",
        store::ASSESSMENT_COLS
    ))
    .bind(session.course_id)
    .bind(session.id)
    .bind(lecturer_id)
    .bind(title)
    .bind(input.instructions_md.unwrap_or_default())
    .bind(kind)
    .bind(settings)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query("insert into assessment_sources (assessment_id, session_id, is_primary) values ($1, $2, true)")
        .bind(row.id)
        .bind(session.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(row)
}

#[derive(Debug, Default, Deserialize)]
pub struct UpdateAssessment {
    pub title: Option<String>,
    pub instructions_md: Option<String>,
    pub kind: Option<String>,
    pub settings: Option<Value>,
    #[serde(default, deserialize_with = "double_option")]
    pub opens_at: Option<Option<DateTime<Utc>>>,
    #[serde(default, deserialize_with = "double_option")]
    pub due_at: Option<Option<DateTime<Utc>>>,
    #[serde(default, deserialize_with = "double_option")]
    pub closes_at: Option<Option<DateTime<Utc>>>,
}

/// Edits the assessment. `base_rev` must be the revision the editor loaded.
pub async fn update(
    pool: &PgPool,
    assessment: &AssessmentRow,
    patch: UpdateAssessment,
    base_rev: i32,
) -> Res<i32> {
    ensure_editable(assessment)?;
    let title = match patch.title {
        Some(t) if t.trim().is_empty() => {
            return Err(ServiceError::Invalid(
                "The title cannot be empty".to_owned(),
            ))
        }
        Some(t) => t.trim().to_owned(),
        None => assessment.title.clone(),
    };
    let kind = patch.kind.unwrap_or_else(|| assessment.kind.clone());
    if !matches!(kind.as_str(), "quiz" | "homework" | "practice") {
        return Err(ServiceError::Invalid(
            "Kind must be quiz, homework or practice".to_owned(),
        ));
    }
    let settings = match patch.settings {
        Some(value) => {
            let parsed: AssessmentSettings = serde_json::from_value(value)
                .map_err(|e| ServiceError::Invalid(format!("Invalid settings: {e}")))?;
            parsed.validate().map_err(ServiceError::Invalid)?;
            serde_json::to_value(parsed).map_err(|e| ServiceError::Internal(e.to_string()))?
        }
        None => assessment.settings.clone(),
    };
    let opens_at = patch.opens_at.unwrap_or(assessment.opens_at);
    let due_at = patch.due_at.unwrap_or(assessment.due_at);
    let closes_at = patch.closes_at.unwrap_or(assessment.closes_at);
    if let (Some(o), Some(d)) = (opens_at, due_at) {
        if d <= o {
            return Err(ServiceError::Invalid(
                "The due time must be after the opening time".to_owned(),
            ));
        }
    }
    if let (Some(d), Some(c)) = (due_at, closes_at) {
        if c < d {
            return Err(ServiceError::Invalid(
                "The closing time cannot be before the due time".to_owned(),
            ));
        }
    }
    if let (Some(o), Some(c)) = (opens_at, closes_at) {
        if c <= o {
            return Err(ServiceError::Invalid(
                "The closing time must be after the opening time".to_owned(),
            ));
        }
    }
    let rev: Option<i32> = sqlx::query_scalar(
        "update assessments set title = $2, instructions_md = $3, kind = $4, settings = $5,
                opens_at = $6, due_at = $7, closes_at = $8, draft_rev = draft_rev + 1, updated_at = now()
         where id = $1 and draft_rev = $9 returning draft_rev",
    )
    .bind(assessment.id)
    .bind(title)
    .bind(patch.instructions_md.unwrap_or_else(|| assessment.instructions_md.clone()))
    .bind(kind)
    .bind(settings)
    .bind(opens_at)
    .bind(due_at)
    .bind(closes_at)
    .bind(base_rev)
    .fetch_optional(pool)
    .await?;
    rev.ok_or_else(|| {
        ServiceError::Conflict(
            "The assessment changed since you opened it. Reload to see the latest.".to_owned(),
        )
    })
}

fn ensure_editable(assessment: &AssessmentRow) -> Res<()> {
    if matches!(assessment.status.as_str(), "finalized" | "archived") {
        return Err(ServiceError::Conflict(
            "This assessment is finalised and can no longer be edited".to_owned(),
        ));
    }
    Ok(())
}

// ---- questions ------------------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct QuestionInput {
    pub question_type: String,
    pub prompt_md: String,
    #[serde(default)]
    pub points: Option<f64>,
    #[serde(default)]
    pub difficulty: Option<String>,
    #[serde(default)]
    pub topic_tag: Option<String>,
    #[serde(default)]
    pub chapter_index: Option<i32>,
    #[serde(default)]
    pub public_payload: Option<Value>,
    #[serde(default)]
    pub key_payload: Option<Value>,
    #[serde(default)]
    pub explanation_md: Option<String>,
    #[serde(default)]
    pub locked: Option<bool>,
}

fn check_difficulty(value: &str) -> Res<()> {
    if matches!(value, "easy" | "medium" | "hard") {
        Ok(())
    } else {
        Err(ServiceError::Invalid(
            "Difficulty must be easy, medium or hard".to_owned(),
        ))
    }
}

pub async fn add_question(
    pool: &PgPool,
    assessment: &AssessmentRow,
    input: QuestionInput,
) -> Res<QuestionRow> {
    ensure_editable(assessment)?;
    let kind = QuestionType::parse(&input.question_type)
        .ok_or_else(|| ServiceError::Invalid("Unknown question type".to_owned()))?;
    let (blank_public, blank_key) = blank_payloads(kind);
    let public = input.public_payload.unwrap_or(blank_public);
    let key = input.key_payload.unwrap_or(blank_key);
    let points = input.points.unwrap_or(1.0);
    let difficulty = input.difficulty.unwrap_or_else(|| "medium".to_owned());
    check_difficulty(&difficulty)?;
    // New questions start incomplete (blank options); they are validated strictly on publish.
    validate_lenient(kind, &input.prompt_md, points, &public, &key)?;
    let row = sqlx::query_as::<_, QuestionRow>(&format!(
        "insert into assessment_questions
           (assessment_id, position, question_type, prompt_md, points, difficulty, topic_tag, chapter_index,
            public_payload, key_payload, explanation_md)
         values ($1, coalesce((select max(position) + 1 from assessment_questions where assessment_id = $1), 0),
                 $2, $3, $4::float8::numeric, $5, $6, $7, $8, $9, $10)
         returning {}",
        store::QUESTION_COLS
    ))
    .bind(assessment.id)
    .bind(kind.as_str())
    .bind(input.prompt_md.trim())
    .bind(points)
    .bind(difficulty)
    .bind(input.topic_tag.filter(|t| !t.trim().is_empty()))
    .bind(input.chapter_index)
    .bind(public)
    .bind(key)
    .bind(input.explanation_md.unwrap_or_default())
    .fetch_one(pool)
    .await?;
    bump_rev(pool, assessment.id).await?;
    Ok(row)
}

/// While editing, only the shape is checked: a half-written question can be
/// saved. `validate_question` runs in full on publish.
fn validate_lenient(
    kind: QuestionType,
    prompt: &str,
    points: f64,
    public: &Value,
    key: &Value,
) -> Res<()> {
    if !(0.0..=1000.0).contains(&points) {
        return Err(ServiceError::Invalid(
            "Points must be between 0 and 1000".to_owned(),
        ));
    }
    if kind.has_options() && !public.get("options").is_some_and(Value::is_array) {
        return Err(ServiceError::Invalid("Options must be a list".to_owned()));
    }
    if !public.is_object() || !key.is_object() {
        return Err(ServiceError::Invalid(
            "Question payloads must be objects".to_owned(),
        ));
    }
    let _ = prompt;
    Ok(())
}

#[derive(Debug, Default, Deserialize)]
pub struct QuestionPatch {
    pub prompt_md: Option<String>,
    pub points: Option<f64>,
    pub difficulty: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub topic_tag: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub chapter_index: Option<Option<i32>>,
    pub public_payload: Option<Value>,
    pub key_payload: Option<Value>,
    pub explanation_md: Option<String>,
    pub locked: Option<bool>,
}

pub async fn update_question(
    pool: &PgPool,
    assessment: &AssessmentRow,
    id: Uuid,
    patch: QuestionPatch,
) -> Res<QuestionRow> {
    ensure_editable(assessment)?;
    let current = store::find_question(pool, assessment.id, id)
        .await?
        .ok_or(ServiceError::NotFound("Question not found"))?;
    let kind = QuestionType::parse(&current.question_type)
        .ok_or_else(|| ServiceError::Internal("stored question type is unknown".to_owned()))?;
    let public = patch
        .public_payload
        .unwrap_or(current.public_payload.clone());
    let key = patch.key_payload.unwrap_or(current.key_payload.clone());
    let points = patch.points.unwrap_or(current.points);
    let difficulty = patch.difficulty.unwrap_or(current.difficulty.clone());
    check_difficulty(&difficulty)?;
    let prompt = patch.prompt_md.unwrap_or(current.prompt_md.clone());
    validate_lenient(kind, &prompt, points, &public, &key)?;
    // Editing an AI question makes it the lecturer's: it stops counting as unreviewed.
    let origin = if current.origin == "ai" {
        "ai_edited"
    } else {
        current.origin.as_str()
    };
    let row = sqlx::query_as::<_, QuestionRow>(&format!(
        "update assessment_questions set prompt_md = $3, points = $4::float8::numeric, difficulty = $5,
                topic_tag = $6, chapter_index = $7, public_payload = $8, key_payload = $9,
                explanation_md = $10, locked = $11, origin = $12, low_confidence = false, updated_at = now()
         where assessment_id = $1 and id = $2 returning {}",
        store::QUESTION_COLS
    ))
    .bind(assessment.id)
    .bind(id)
    .bind(prompt.trim())
    .bind(points)
    .bind(difficulty)
    .bind(patch.topic_tag.unwrap_or(current.topic_tag.clone()))
    .bind(patch.chapter_index.unwrap_or(current.chapter_index))
    .bind(public)
    .bind(key)
    .bind(patch.explanation_md.unwrap_or(current.explanation_md.clone()))
    .bind(patch.locked.unwrap_or(current.locked))
    .bind(origin)
    .fetch_one(pool)
    .await?;
    bump_rev(pool, assessment.id).await?;
    Ok(row)
}

pub async fn delete_question(pool: &PgPool, assessment: &AssessmentRow, id: Uuid) -> Res<()> {
    ensure_editable(assessment)?;
    let done = sqlx::query("delete from assessment_questions where assessment_id = $1 and id = $2")
        .bind(assessment.id)
        .bind(id)
        .execute(pool)
        .await?;
    if done.rows_affected() == 0 {
        return Err(ServiceError::NotFound("Question not found"));
    }
    bump_rev(pool, assessment.id).await?;
    Ok(())
}

/// `ids` is the complete new order.
pub async fn reorder_questions(pool: &PgPool, assessment: &AssessmentRow, ids: &[Uuid]) -> Res<()> {
    ensure_editable(assessment)?;
    let existing = store::questions(pool, assessment.id).await?;
    let mut have: Vec<Uuid> = existing.iter().map(|q| q.id).collect();
    let mut want = ids.to_vec();
    have.sort();
    want.sort();
    if have != want {
        return Err(ServiceError::Invalid(
            "The new order must list every question exactly once".to_owned(),
        ));
    }
    let mut tx = pool.begin().await?;
    for (position, id) in ids.iter().enumerate() {
        sqlx::query(
            "update assessment_questions set position = $3 where assessment_id = $1 and id = $2",
        )
        .bind(assessment.id)
        .bind(id)
        .bind(position as i32)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    bump_rev(pool, assessment.id).await?;
    Ok(())
}

async fn bump_rev(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "update assessments set draft_rev = draft_rev + 1, updated_at = now() where id = $1",
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Appends AI-written questions (never touching existing or locked ones).
pub async fn insert_generated(
    pool: &PgPool,
    assessment_id: Uuid,
    questions: &[GeneratedQuestion],
    ai_job_id: Uuid,
) -> Result<usize, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let mut position: i32 = sqlx::query_scalar(
        "select coalesce(max(position) + 1, 0) from assessment_questions where assessment_id = $1",
    )
    .bind(assessment_id)
    .fetch_one(&mut *tx)
    .await?;
    for q in questions {
        let ranges: Vec<Value> = q
            .source_ranges
            .iter()
            .map(|(start, end)| json!({"start_sec": start, "end_sec": end}))
            .collect();
        sqlx::query(
            "insert into assessment_questions
               (assessment_id, position, question_type, prompt_md, points, difficulty, topic_tag, chapter_index,
                public_payload, key_payload, explanation_md, origin, source_ranges, low_confidence, ai_job_id)
             values ($1, $2, $3, $4, $5::float8::numeric, $6, $7, $8, $9, $10, $11, 'ai', $12, $13, $14)",
        )
        .bind(assessment_id)
        .bind(position)
        .bind(q.question_type.as_str())
        .bind(&q.prompt_md)
        .bind(q.points)
        .bind(&q.difficulty)
        .bind(&q.topic_tag)
        .bind(q.chapter_index)
        .bind(&q.public_payload)
        .bind(&q.key_payload)
        .bind(&q.explanation_md)
        .bind(Value::Array(ranges))
        .bind(q.low_confidence)
        .bind(ai_job_id)
        .execute(&mut *tx)
        .await?;
        position += 1;
    }
    sqlx::query(
        "update assessments set draft_rev = draft_rev + 1, updated_at = now() where id = $1",
    )
    .bind(assessment_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(questions.len())
}

// ---- publish ----------------------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct Published {
    pub version_id: Uuid,
    pub version_no: i32,
    /// False when the content was identical to the latest version, so no new one was cut.
    pub new_version: bool,
}

/// Validates everything strictly and cuts an immutable version. Publishing an
/// unchanged assessment again is a no-op; attempts already started keep their version.
pub async fn publish(
    pool: &PgPool,
    assessment: &AssessmentRow,
    lecturer_id: Uuid,
) -> Res<Published> {
    if !matches!(assessment.status.as_str(), "draft" | "published") {
        return Err(ServiceError::Conflict(
            "Only a draft or published assessment can be published".to_owned(),
        ));
    }
    let settings = assessment.settings();
    settings.validate().map_err(ServiceError::Invalid)?;
    let rows = store::questions(pool, assessment.id).await?;
    if rows.is_empty() {
        return Err(ServiceError::Invalid(
            "Add at least one question before publishing".to_owned(),
        ));
    }
    let mut questions = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        let kind = QuestionType::parse(&row.question_type)
            .ok_or_else(|| ServiceError::Internal("stored question type is unknown".to_owned()))?;
        validate_question(
            kind,
            &row.prompt_md,
            row.points,
            &row.public_payload,
            &row.key_payload,
        )
        .map_err(|m| ServiceError::Invalid(format!("Question {}: {m}", index + 1)))?;
        questions.push(SnapshotQuestion {
            id: row.id,
            position: index as i32,
            question_type: kind,
            prompt_md: row.prompt_md.clone(),
            points: row.points,
            difficulty: row.difficulty.clone(),
            topic_tag: row.topic_tag.clone(),
            chapter_index: row.chapter_index,
            public_payload: row.public_payload.clone(),
            key_payload: row.key_payload.clone(),
            explanation_md: row.explanation_md.clone(),
            source_ranges: row.source_ranges.clone(),
        });
    }
    if questions.iter().map(|q| q.points).sum::<f64>() <= 0.0 {
        return Err(ServiceError::Invalid(
            "The questions must be worth some points".to_owned(),
        ));
    }
    let snapshot = Snapshot {
        title: assessment.title.clone(),
        instructions_md: assessment.instructions_md.clone(),
        kind: assessment.kind.clone(),
        settings,
        questions,
    };
    let value =
        serde_json::to_value(&snapshot).map_err(|e| ServiceError::Internal(e.to_string()))?;
    let hash = crate::utils::hex_lower(Sha256::digest(value.to_string().as_bytes()));

    let mut tx = pool.begin().await?;
    // Serialise publishes of one assessment so version numbers cannot collide.
    sqlx::query("select id from assessments where id = $1 for update")
        .bind(assessment.id)
        .fetch_one(&mut *tx)
        .await?;
    let latest: Option<(Uuid, i32, String)> = sqlx::query_as(
        "select id, version_no, content_hash from assessment_versions
         where assessment_id = $1 order by version_no desc limit 1",
    )
    .bind(assessment.id)
    .fetch_optional(&mut *tx)
    .await?;
    let (version_id, version_no, new_version) = match latest {
        Some((id, no, existing_hash)) if existing_hash == hash => (id, no, false),
        other => {
            let no = other.map_or(1, |l| l.1 + 1);
            let id: Uuid = sqlx::query_scalar(
                "insert into assessment_versions (assessment_id, version_no, snapshot, content_hash, published_by)
                 values ($1, $2, $3, $4, $5) returning id",
            )
            .bind(assessment.id)
            .bind(no)
            .bind(&value)
            .bind(&hash)
            .bind(lecturer_id)
            .fetch_one(&mut *tx)
            .await?;
            (id, no, true)
        }
    };
    sqlx::query(
        "update assessments set status = 'published', published_version_id = $2, updated_at = now() where id = $1",
    )
    .bind(assessment.id)
    .bind(version_id)
    .execute(&mut *tx)
    .await?;
    if new_version || assessment.status == "draft" {
        store::emit(
            &mut *tx,
            "assessment.published",
            json!({"assessment_id": assessment.id, "course_id": assessment.course_id,
                   "version_id": version_id, "version_no": version_no}),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(Published {
        version_id,
        version_no,
        new_version,
    })
}

// ---- close -----------------------------------------------------------------------------------------

/// Stops new attempts, submits any still in progress, and queues AI grading
/// when the assessment uses it.
pub async fn close(pool: &PgPool, assessment: &AssessmentRow, requested_by: Uuid) -> Res<usize> {
    if assessment.status != "published" {
        return Err(ServiceError::Conflict(
            "Only a published assessment can be closed".to_owned(),
        ));
    }
    sqlx::query("update assessments set status = 'closed', updated_at = now() where id = $1")
        .bind(assessment.id)
        .execute(pool)
        .await?;
    let open: Vec<Uuid> = sqlx::query_scalar(
        "select id from assessment_attempts where assessment_id = $1 and status = 'in_progress'",
    )
    .bind(assessment.id)
    .fetch_all(pool)
    .await?;
    let mut forced = 0;
    for id in &open {
        if attempts::force_submit(pool, *id).await.is_ok() {
            forced += 1;
        }
    }
    store::emit(
        pool,
        "assessment.closed",
        json!({"assessment_id": assessment.id}),
    )
    .await?;
    if assessment.settings().scoring_mode != super::model::ScoringMode::Manual {
        super::generate::enqueue_grading(pool, assessment, requested_by, None)
            .await
            .ok();
    }
    Ok(forced)
}

// ---- accommodations ---------------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct AccommodationInput {
    pub student_account_id: Uuid,
    #[serde(default)]
    pub extra_time_pct: i32,
    #[serde(default)]
    pub extra_attempts: i32,
    #[serde(default, deserialize_with = "double_option")]
    pub due_override: Option<Option<DateTime<Utc>>>,
    #[serde(default)]
    pub reason: String,
}

pub async fn set_accommodation(
    pool: &PgPool,
    assessment: &AssessmentRow,
    lecturer_id: Uuid,
    input: AccommodationInput,
) -> Res<()> {
    if !(0..=300).contains(&input.extra_time_pct) || !(0..=10).contains(&input.extra_attempts) {
        return Err(ServiceError::Invalid(
            "Extra time is 0-300% and extra attempts 0-10".to_owned(),
        ));
    }
    if !crate::materials::store::student_enrolled(
        pool,
        input.student_account_id,
        assessment.course_id,
    )
    .await?
    {
        return Err(ServiceError::NotFound(
            "That student is not enrolled in this course",
        ));
    }
    sqlx::query(
        "insert into assessment_accommodations
           (assessment_id, student_account_id, extra_time_pct, extra_attempts, due_override, reason, created_by)
         values ($1, $2, $3, $4, $5, $6, $7)
         on conflict (assessment_id, student_account_id) do update
           set extra_time_pct = excluded.extra_time_pct, extra_attempts = excluded.extra_attempts,
               due_override = excluded.due_override, reason = excluded.reason, created_by = excluded.created_by",
    )
    .bind(assessment.id)
    .bind(input.student_account_id)
    .bind(input.extra_time_pct)
    .bind(input.extra_attempts)
    .bind(input.due_override.flatten())
    .bind(input.reason.trim())
    .bind(lecturer_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn remove_accommodation(
    pool: &PgPool,
    assessment: &AssessmentRow,
    student_id: Uuid,
) -> Res<()> {
    sqlx::query("delete from assessment_accommodations where assessment_id = $1 and student_account_id = $2")
        .bind(assessment.id)
        .bind(student_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AccommodationRow {
    pub student_account_id: Uuid,
    pub display_name: String,
    pub matric_number: String,
    pub extra_time_pct: i32,
    pub extra_attempts: i32,
    pub due_override: Option<DateTime<Utc>>,
    pub reason: String,
}

pub async fn list_accommodations(pool: &PgPool, assessment_id: Uuid) -> Res<Vec<AccommodationRow>> {
    Ok(sqlx::query_as::<_, AccommodationRow>(
        "select a.student_account_id, s.display_name, s.matric_number, a.extra_time_pct, a.extra_attempts,
                a.due_override, a.reason
         from assessment_accommodations a join student_accounts s on s.id = a.student_account_id
         where a.assessment_id = $1 order by s.display_name",
    )
    .bind(assessment_id)
    .fetch_all(pool)
    .await?)
}

// ---- roster ---------------------------------------------------------------------------------------------

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct RosterEntry {
    pub student_account_id: Uuid,
    pub display_name: String,
    pub matric_number: String,
    /// not_started, in_progress or submitted. Never a score: scores stay blind until finalise.
    pub state: String,
    pub attempts: i64,
}

/// Who has started and submitted, for chasing missing work. This is roster
/// status only; it carries no scores and no link to any answer.
pub async fn roster_status(pool: &PgPool, assessment: &AssessmentRow) -> Res<Vec<RosterEntry>> {
    Ok(sqlx::query_as::<_, RosterEntry>(
        "select s.id as student_account_id, s.display_name, s.matric_number,
                case when bool_or(a.status in ('submitted', 'auto_submitted', 'graded')) then 'submitted'
                     when bool_or(a.status = 'in_progress') then 'in_progress'
                     else 'not_started' end as state,
                count(a.id) as attempts
         from student_course_enrollments e
         join student_accounts s on s.id = e.student_account_id
         left join assessment_attempts a on a.student_account_id = s.id and a.assessment_id = $1
         where e.course_id = $2
         group by s.id, s.display_name, s.matric_number
         order by s.display_name",
    )
    .bind(assessment.id)
    .bind(assessment.course_id)
    .fetch_all(pool)
    .await?)
}

// ---- lectures to create from ------------------------------------------------------------------------

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct LectureOption {
    pub id: Uuid,
    pub title: String,
    pub short_code: String,
    pub status: String,
    pub started_at: Option<DateTime<Utc>>,
    pub has_transcript: bool,
    /// How many assessments already hang off this lecture.
    pub assessments: i64,
}

/// A lecturer's lectures in a course, newest first.
pub async fn lecturer_lectures(
    pool: &PgPool,
    course_id: Uuid,
    lecturer_id: Uuid,
) -> Res<Vec<LectureOption>> {
    Ok(sqlx::query_as::<_, LectureOption>(
        "select s.id, s.title, s.short_code, s.status::text as status, s.started_at,
                exists(select 1 from caption_chunks c where c.session_id = s.id and not c.is_hidden) as has_transcript,
                (select count(*) from assessments a where a.session_id = s.id) as assessments
         from lecture_sessions s
         where s.course_id = $1 and s.deleted_at is null and s.lecturer_id = $2
         order by s.started_at desc nulls last, s.id desc limit 100",
    )
    .bind(course_id)
    .bind(lecturer_id)
    .fetch_all(pool)
    .await?)
}
