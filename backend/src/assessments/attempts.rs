//! Student side: listing, starting and resuming attempts, idempotent
//! (offline-safe) sync, submit, integrity events and released results.
//! Everything returned here is built from the student view types, so no answer
//! key can reach a student before the lecturer's visibility setting allows it.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    delivery::{
        can_start, compute_deadline, is_known_event, is_violation, judge_sync, plan_attempt,
        Accommodation, StartContext, SubmitClaim, SyncDecision,
    },
    grading,
    model::{
        revealable_answer, AssessmentSettings, QuestionType, ReleasePolicy, ResultQuestion,
        Snapshot, SnapshotQuestion, StudentQuestion, Visibility,
    },
    scoring::{late_outcome, score_answer},
    store::{self, AssessmentRow, AttemptRow},
    Res, ServiceError,
};

// ---- views ------------------------------------------------------------------------------------

/// The settings a client needs to behave correctly. Scoring and AI options are
/// not included: the student has no use for them.
#[derive(Debug, Serialize)]
pub struct StudentSettingsView {
    pub time_limit_secs: i64,
    pub grace_secs: i64,
    pub one_per_page: bool,
    pub allow_backtracking: bool,
    pub require_fullscreen: bool,
    pub block_paste: bool,
    pub records_integrity: bool,
    pub allow_offline_continue: bool,
    pub allow_offline_start: bool,
    pub max_offline_gap_secs: i64,
    pub auto_submit_after_violations: i32,
    pub max_attempts: i32,
}

impl StudentSettingsView {
    fn from(s: &AssessmentSettings) -> Self {
        Self {
            time_limit_secs: s.time_limit_secs,
            grace_secs: s.grace_secs,
            one_per_page: s.effective_one_per_page(),
            allow_backtracking: s.effective_backtracking(),
            require_fullscreen: s.effective_fullscreen(),
            block_paste: s.effective_block_paste(),
            records_integrity: s.records_integrity(),
            allow_offline_continue: s.allow_offline_continue,
            allow_offline_start: s.allow_offline_start,
            max_offline_gap_secs: s.max_offline_gap_secs,
            auto_submit_after_violations: s.auto_submit_after_violations,
            max_attempts: s.max_attempts,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SavedAnswer {
    pub question_id: Uuid,
    pub response: Value,
    pub revision: i32,
    pub flagged_for_review: bool,
}

#[derive(Debug, Serialize)]
pub struct AttemptView {
    pub attempt_id: Uuid,
    pub assessment_id: Uuid,
    pub title: String,
    pub instructions_md: String,
    pub kind: String,
    pub attempt_no: i32,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub deadline_at: Option<DateTime<Utc>>,
    /// Lets the client measure its offset from the server clock.
    pub server_now: DateTime<Utc>,
    pub settings: StudentSettingsView,
    pub questions: Vec<StudentQuestion>,
    pub answers: Vec<SavedAnswer>,
    pub receipt: Option<String>,
    pub submitted_at: Option<DateTime<Utc>>,
    /// The server closed the attempt (time ran out or a limit was reached).
    pub auto_submitted: bool,
}

async fn load_context(pool: &PgPool, attempt: &AttemptRow) -> Res<(AssessmentRow, Snapshot)> {
    let assessment = store::find_assessment(pool, attempt.assessment_id)
        .await?
        .ok_or(ServiceError::NotFound("Assessment not found"))?;
    let snapshot = store::load_snapshot(pool, attempt.version_id)
        .await?
        .ok_or_else(|| ServiceError::Internal("the attempt's version is missing".to_owned()))?;
    Ok((assessment, snapshot))
}

pub async fn attempt_view(
    pool: &PgPool,
    student_id: Uuid,
    attempt_id: Uuid,
    now: DateTime<Utc>,
) -> Res<AttemptView> {
    let attempt = owned_attempt(pool, student_id, attempt_id).await?;
    build_view(pool, &attempt, now).await
}

async fn owned_attempt(pool: &PgPool, student_id: Uuid, attempt_id: Uuid) -> Res<AttemptRow> {
    let attempt = store::find_attempt(pool, attempt_id)
        .await?
        .ok_or(ServiceError::NotFound("Attempt not found"))?;
    if attempt.student_account_id != student_id {
        return Err(ServiceError::NotFound("Attempt not found"));
    }
    Ok(attempt)
}

async fn build_view(pool: &PgPool, attempt: &AttemptRow, now: DateTime<Utc>) -> Res<AttemptView> {
    let (assessment, snapshot) = load_context(pool, attempt).await?;
    let order: Vec<Uuid> =
        serde_json::from_value(attempt.question_order.clone()).unwrap_or_default();
    let option_orders: HashMap<String, Vec<String>> =
        serde_json::from_value(attempt.option_orders.clone()).unwrap_or_default();
    let questions = order
        .iter()
        .enumerate()
        .filter_map(|(i, id)| {
            let q = snapshot.question(*id)?;
            let opts = option_orders
                .get(&id.to_string())
                .cloned()
                .unwrap_or_default();
            Some(StudentQuestion::build(q, i, &opts))
        })
        .collect();
    let answers = sqlx::query_as::<_, (Uuid, Value, i32, bool)>(
        "select question_id, response, revision, flagged_for_review from attempt_answers where attempt_id = $1",
    )
    .bind(attempt.id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(question_id, response, revision, flagged_for_review)| SavedAnswer {
        question_id,
        response,
        revision,
        flagged_for_review,
    })
    .collect();
    Ok(AttemptView {
        attempt_id: attempt.id,
        assessment_id: assessment.id,
        title: assessment.title.clone(),
        instructions_md: assessment.instructions_md.clone(),
        kind: assessment.kind.clone(),
        attempt_no: attempt.attempt_no,
        status: attempt.status.clone(),
        started_at: attempt.started_at,
        deadline_at: attempt.deadline_at,
        server_now: now,
        settings: StudentSettingsView::from(&assessment.settings()),
        questions,
        answers,
        receipt: attempt.receipt.clone(),
        submitted_at: attempt.submitted_at,
        auto_submitted: attempt.auto_submitted,
    })
}

// ---- listing --------------------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct StudentAssessment {
    pub id: Uuid,
    pub title: String,
    pub kind: String,
    pub status: String,
    pub course_id: Uuid,
    pub course_code: String,
    pub course_title: String,
    pub session_id: Uuid,
    pub lecture_title: String,
    pub instructions_md: String,
    pub opens_at: Option<DateTime<Utc>>,
    pub due_at: Option<DateTime<Utc>>,
    pub closes_at: Option<DateTime<Utc>>,
    pub time_limit_secs: i64,
    /// The rules a student agrees to before starting (integrity level and controls).
    pub rules: StudentSettingsView,
    pub integrity: super::model::IntegrityLevel,
    pub max_attempts: i32,
    pub attempts_used: i32,
    pub in_progress_attempt_id: Option<Uuid>,
    pub latest_attempt_id: Option<Uuid>,
    /// upcoming, open, in_progress, submitted or closed.
    pub state: &'static str,
    pub can_start: bool,
    pub results_available: bool,
}

#[derive(sqlx::FromRow)]
struct ListRow {
    id: Uuid,
    title: String,
    kind: String,
    status: String,
    course_id: Uuid,
    course_code: String,
    course_title: String,
    session_id: Uuid,
    lecture_title: String,
    instructions_md: String,
    opens_at: Option<DateTime<Utc>>,
    due_at: Option<DateTime<Utc>>,
    closes_at: Option<DateTime<Utc>>,
    settings: Value,
    results_released_at: Option<DateTime<Utc>>,
    identity_revealed_at: Option<DateTime<Utc>>,
}

pub async fn list_for_student(
    pool: &PgPool,
    student_id: Uuid,
    now: DateTime<Utc>,
) -> Res<Vec<StudentAssessment>> {
    let rows = sqlx::query_as::<_, ListRow>(
        "select a.id, a.title, a.kind, a.status, a.course_id, c.code as course_code, c.title as course_title,
                a.session_id, s.title as lecture_title, a.instructions_md, a.opens_at, a.due_at, a.closes_at, a.settings,
                a.results_released_at, a.identity_revealed_at
         from assessments a
         join courses c on c.id = a.course_id
         join lecture_sessions s on s.id = a.session_id
         join student_course_enrollments e on e.course_id = a.course_id and e.student_account_id = $1
         where a.status in ('published', 'closed', 'finalized')
         order by coalesce(a.due_at, a.closes_at, a.created_at) desc",
    )
    .bind(student_id)
    .fetch_all(pool)
    .await?;
    let attempts =
        sqlx::query_as::<_, (Uuid, i64, Option<Uuid>, Option<Uuid>, Option<DateTime<Utc>>)>(
            "select assessment_id, count(*),
                (array_agg(id order by attempt_no) filter (where status = 'in_progress'))[1],
                (array_agg(id order by attempt_no desc))[1],
                max(submitted_at)
         from assessment_attempts where student_account_id = $1 group by assessment_id",
        )
        .bind(student_id)
        .fetch_all(pool)
        .await?;
    let by_assessment: HashMap<Uuid, _> = attempts.into_iter().map(|r| (r.0, r)).collect();
    let accommodations = sqlx::query_as::<_, (Uuid, i32, i32, Option<DateTime<Utc>>)>(
        "select assessment_id, extra_time_pct, extra_attempts, due_override
         from assessment_accommodations where student_account_id = $1",
    )
    .bind(student_id)
    .fetch_all(pool)
    .await?;
    let acc_by: HashMap<Uuid, Accommodation> = accommodations
        .into_iter()
        .map(|(id, t, a, d)| {
            (
                id,
                Accommodation {
                    extra_time_pct: t,
                    extra_attempts: a,
                    due_override: d,
                },
            )
        })
        .collect();

    let mut out = Vec::new();
    for row in rows {
        let settings = AssessmentSettings::from_value(&row.settings);
        let (used, open_id, latest_id, last_submitted) = by_assessment
            .get(&row.id)
            .map_or((0, None, None, None), |r| (r.1 as i32, r.2, r.3, r.4));
        let acc = acc_by.get(&row.id).copied().unwrap_or_default();
        let window = super::delivery::Window {
            opens_at: row.opens_at,
            due_at: row.due_at,
            closes_at: row.closes_at,
        };
        let startable = can_start(&StartContext {
            now,
            status: &row.status,
            window,
            settings: &settings,
            accommodation: acc,
            attempts_used: used,
            last_submitted_at: last_submitted,
        });
        let released = results_released(
            &row.status,
            row.results_released_at,
            row.due_at,
            &settings,
            now,
        );
        let state = if open_id.is_some() {
            "in_progress"
        } else if startable.is_ok() {
            "open"
        } else if matches!(startable, Err(super::delivery::StartDenied::NotOpenYet(_))) {
            "upcoming"
        } else if used > 0 {
            "submitted"
        } else {
            "closed"
        };
        let _ = row.identity_revealed_at;
        out.push(StudentAssessment {
            id: row.id,
            title: row.title,
            kind: row.kind,
            status: row.status,
            course_id: row.course_id,
            course_code: row.course_code,
            course_title: row.course_title,
            session_id: row.session_id,
            lecture_title: row.lecture_title,
            instructions_md: row.instructions_md,
            opens_at: row.opens_at,
            due_at: row.due_at,
            closes_at: row.closes_at,
            time_limit_secs: settings.time_limit_secs,
            rules: StudentSettingsView::from(&settings),
            integrity: settings.integrity,
            max_attempts: settings.max_attempts + acc.extra_attempts,
            attempts_used: used,
            in_progress_attempt_id: open_id,
            latest_attempt_id: latest_id,
            state,
            can_start: startable.is_ok() || open_id.is_some(),
            results_available: released && used > 0,
        });
    }
    Ok(out)
}

/// Whether students may see results, by the release setting. An explicit
/// release by the lecturer always counts.
pub fn results_released(
    status: &str,
    released_at: Option<DateTime<Utc>>,
    due_at: Option<DateTime<Utc>>,
    settings: &AssessmentSettings,
    now: DateTime<Utc>,
) -> bool {
    if released_at.is_some_and(|t| t <= now) {
        return true;
    }
    match settings.release {
        ReleasePolicy::Manual => false,
        ReleasePolicy::AfterFinalize => status == "finalized",
        ReleasePolicy::AfterDue => {
            matches!(status, "closed" | "finalized") || due_at.is_some_and(|d| now >= d)
        }
        ReleasePolicy::Immediate => true,
    }
}

// ---- start ----------------------------------------------------------------------------------------

pub async fn start_attempt(
    pool: &PgPool,
    student_id: Uuid,
    assessment_id: Uuid,
    device_id: Option<String>,
    now: DateTime<Utc>,
) -> Res<AttemptView> {
    let assessment = store::find_assessment(pool, assessment_id)
        .await?
        .ok_or(ServiceError::NotFound("Assessment not found"))?;
    if !crate::materials::store::student_enrolled(pool, student_id, assessment.course_id).await? {
        return Err(ServiceError::NotFound("Assessment not found"));
    }
    let settings = assessment.settings();
    let version_id = assessment
        .published_version_id
        .ok_or(ServiceError::NotFound("Assessment not found"))?;

    // Resume an open attempt, or close it out if its time has run out.
    if let Some(open) = open_attempt(pool, assessment.id, student_id).await? {
        let expired = open
            .deadline_at
            .is_some_and(|d| now > d + chrono::Duration::seconds(settings.grace_secs));
        if !expired {
            return build_view(pool, &open, now).await;
        }
        finish(
            pool,
            &open,
            &assessment,
            "auto_submitted",
            open.deadline_at.unwrap_or(now),
            false,
            None,
        )
        .await?;
    }

    let accommodation = store::accommodation(pool, assessment.id, student_id).await?;
    let (used, last_submitted): (i64, Option<DateTime<Utc>>) = sqlx::query_as(
        "select count(*), max(submitted_at) from assessment_attempts
         where assessment_id = $1 and student_account_id = $2 and status <> 'void'",
    )
    .bind(assessment.id)
    .bind(student_id)
    .fetch_one(pool)
    .await?;
    can_start(&StartContext {
        now,
        status: &assessment.status,
        window: assessment.window(),
        settings: &settings,
        accommodation,
        attempts_used: used as i32,
        last_submitted_at: last_submitted,
    })
    .map_err(|denied| ServiceError::Conflict(denied.message()))?;

    let snapshot = store::load_snapshot(pool, version_id)
        .await?
        .ok_or_else(|| ServiceError::Internal("the published version is missing".to_owned()))?;
    let seed: i64 = rand::random();
    let plan = plan_attempt(seed, &snapshot.questions, &settings);
    let deadline = compute_deadline(now, &assessment.window(), &settings, &accommodation);
    let order = serde_json::to_value(&plan.question_order)
        .map_err(|e| ServiceError::Internal(e.to_string()))?;
    let option_orders: HashMap<String, &Vec<String>> = plan
        .option_orders
        .iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    let option_orders =
        serde_json::to_value(option_orders).map_err(|e| ServiceError::Internal(e.to_string()))?;

    for _ in 0..6 {
        let alias = format!("Candidate {}", 10_000 + rand::random::<u32>() % 90_000);
        let inserted = sqlx::query_as::<_, AttemptRow>(&format!(
            "insert into assessment_attempts
               (assessment_id, version_id, student_account_id, attempt_no, grading_alias, started_at,
                deadline_at, question_order, option_orders, seed, device_id)
             values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) returning {}",
            store::ATTEMPT_COLS
        ))
        .bind(assessment.id)
        .bind(version_id)
        .bind(student_id)
        .bind(used as i32 + 1)
        .bind(&alias)
        .bind(now)
        .bind(deadline)
        .bind(&order)
        .bind(&option_orders)
        .bind(seed)
        .bind(&device_id)
        .fetch_one(pool)
        .await;
        match inserted {
            Ok(row) => return build_view(pool, &row, now).await,
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                // A double click or a second device raced us: resume the winner.
                if let Some(open) = open_attempt(pool, assessment.id, student_id).await? {
                    return build_view(pool, &open, now).await;
                }
            }
            Err(error) => return Err(error.into()),
        }
    }
    Err(ServiceError::Internal(
        "could not allocate an attempt".to_owned(),
    ))
}

async fn open_attempt(
    pool: &PgPool,
    assessment_id: Uuid,
    student_id: Uuid,
) -> Res<Option<AttemptRow>> {
    Ok(sqlx::query_as::<_, AttemptRow>(&format!(
        "select {} from assessment_attempts
         where assessment_id = $1 and student_account_id = $2 and status = 'in_progress'",
        store::ATTEMPT_COLS
    ))
    .bind(assessment_id)
    .bind(student_id)
    .fetch_optional(pool)
    .await?)
}

// ---- sync (autosave, offline replay, submit) ----------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct AnswerOp {
    /// Unique per change on the client, so a replayed batch never applies twice.
    pub op_id: Uuid,
    pub question_id: Uuid,
    pub response: Value,
    /// Client-side counter; the highest revision wins per question.
    pub revision: i32,
    #[serde(default)]
    pub time_spent_ms: Option<i32>,
    #[serde(default)]
    pub flagged: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ClientEvent {
    pub kind: String,
    #[serde(default)]
    pub client_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub meta: Value,
}

#[derive(Debug, Deserialize)]
pub struct SubmitRequest {
    /// Monotonic milliseconds the device measured from the server-issued start.
    #[serde(default)]
    pub elapsed_ms: Option<i64>,
    #[serde(default)]
    pub claimed_at: Option<DateTime<Utc>>,
    /// The submit was made while the device was offline.
    #[serde(default)]
    pub offline: bool,
}

#[derive(Debug, Default, Deserialize)]
pub struct SyncRequest {
    #[serde(default)]
    pub device_id: Option<String>,
    #[serde(default)]
    pub ops: Vec<AnswerOp>,
    #[serde(default)]
    pub events: Vec<ClientEvent>,
    #[serde(default)]
    pub submit: Option<SubmitRequest>,
}

#[derive(Debug, Serialize)]
pub struct SyncResponse {
    pub applied: usize,
    pub duplicates: usize,
    pub rejected: usize,
    pub status: String,
    pub deadline_at: Option<DateTime<Utc>>,
    pub server_now: DateTime<Utc>,
    pub receipt: Option<String>,
    /// Accepted after the deadline on the strength of an offline claim.
    pub flagged: bool,
    pub auto_submitted: bool,
}

/// Normalises a client response for a question, or rejects it. Keeping only
/// the expected shape means a client cannot store arbitrary JSON.
fn clean_response(question: &SnapshotQuestion, response: &Value) -> Option<Value> {
    match question.question_type {
        QuestionType::McqSingle | QuestionType::McqMulti => {
            let valid: Vec<String> = super::model::declared_options(&question.public_payload)
                .into_iter()
                .map(|o| o.id)
                .collect();
            let mut selected: Vec<String> = super::model::string_list(response.get("selected"));
            selected.sort();
            selected.dedup();
            if selected.iter().any(|s| !valid.contains(s)) {
                return None;
            }
            if question.question_type == QuestionType::McqSingle && selected.len() > 1 {
                return None;
            }
            Some(json!({"selected": selected}))
        }
        QuestionType::TrueFalse => match response.get("value") {
            None | Some(Value::Null) => Some(json!({})),
            Some(Value::Bool(b)) => Some(json!({"value": b})),
            _ => None,
        },
        QuestionType::ShortAnswer | QuestionType::LongAnswer => {
            let text = response.get("text").and_then(Value::as_str).unwrap_or("");
            let cap = question
                .public_payload
                .get("max_chars")
                .and_then(Value::as_u64)
                .unwrap_or(20_000)
                .min(50_000) as usize;
            if text.chars().count() > cap.max(50) * 3 {
                return None;
            }
            Some(json!({"text": text}))
        }
    }
}

pub async fn sync(
    pool: &PgPool,
    student_id: Uuid,
    attempt_id: Uuid,
    request: SyncRequest,
    now: DateTime<Utc>,
) -> Res<SyncResponse> {
    let attempt = owned_attempt(pool, student_id, attempt_id).await?;
    let (assessment, snapshot) = load_context(pool, &attempt).await?;
    let settings = assessment.settings();
    let mut response = SyncResponse {
        applied: 0,
        duplicates: 0,
        rejected: 0,
        status: attempt.status.clone(),
        deadline_at: attempt.deadline_at,
        server_now: now,
        receipt: attempt.receipt.clone(),
        flagged: false,
        auto_submitted: false,
    };
    if !attempt.is_open() {
        // Already submitted: a replayed submit is idempotent; late ops are refused.
        response.rejected = request.ops.len();
        return Ok(response);
    }

    let claim = request.submit.as_ref().and_then(|s| {
        s.elapsed_ms.map(|elapsed_ms| SubmitClaim {
            elapsed_ms,
            claimed_at: s.claimed_at,
        })
    });
    let decision = judge_sync(
        now,
        attempt.started_at,
        attempt.deadline_at,
        settings.grace_secs,
        settings.allow_offline_continue,
        claim,
    );
    if decision == SyncDecision::Reject {
        // Too late: close the attempt with what the server already holds.
        let at = attempt.deadline_at.unwrap_or(now);
        finish(
            pool,
            &attempt,
            &assessment,
            "auto_submitted",
            at,
            false,
            None,
        )
        .await?;
        let done = store::find_attempt(pool, attempt.id)
            .await?
            .unwrap_or(attempt);
        response.rejected = request.ops.len();
        response.status = done.status;
        response.receipt = done.receipt;
        response.auto_submitted = true;
        return Ok(response);
    }
    if let SyncDecision::AcceptFlagged(reason) = &decision {
        response.flagged = true;
        record_event(
            pool,
            attempt.id,
            "late_sync",
            request.submit.as_ref().and_then(|s| s.claimed_at),
            json!({"reason": reason}),
            false,
        )
        .await?;
    }

    // A second device takes over: allowed, and logged.
    if let (Some(device), Some(existing)) = (&request.device_id, &attempt.device_id) {
        if device != existing {
            record_event(
                pool,
                attempt.id,
                "device_change",
                None,
                json!({"from": existing, "to": device}),
                true,
            )
            .await?;
            sqlx::query("update assessment_attempts set device_id = $2 where id = $1")
                .bind(attempt.id)
                .bind(device)
                .execute(pool)
                .await?;
        }
    } else if let (Some(device), None) = (&request.device_id, &attempt.device_id) {
        sqlx::query("update assessment_attempts set device_id = $2 where id = $1")
            .bind(attempt.id)
            .bind(device)
            .execute(pool)
            .await?;
    }

    for op in &request.ops {
        let Some(question) = snapshot.question(op.question_id) else {
            response.rejected += 1;
            continue;
        };
        let Some(clean) = clean_response(question, &op.response) else {
            response.rejected += 1;
            continue;
        };
        let mut tx = pool.begin().await?;
        let fresh = sqlx::query(
            "insert into attempt_ops (client_op_id, attempt_id) values ($1, $2) on conflict do nothing",
        )
        .bind(op.op_id)
        .bind(attempt.id)
        .execute(&mut *tx)
        .await?
        .rows_affected()
            > 0;
        if !fresh {
            response.duplicates += 1;
            continue;
        }
        sqlx::query(
            "insert into attempt_answers (attempt_id, question_id, response, revision, time_spent_ms, flagged_for_review, answered_at)
             values ($1, $2, $3, $4, coalesce($5, 0), coalesce($6, false), now())
             on conflict (attempt_id, question_id) do update
               set response = excluded.response, revision = excluded.revision,
                   time_spent_ms = greatest(attempt_answers.time_spent_ms, excluded.time_spent_ms),
                   flagged_for_review = coalesce($6, attempt_answers.flagged_for_review), answered_at = now()
             where attempt_answers.revision <= excluded.revision",
        )
        .bind(attempt.id)
        .bind(op.question_id)
        .bind(&clean)
        .bind(op.revision)
        .bind(op.time_spent_ms)
        .bind(op.flagged)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        response.applied += 1;
    }

    let violations = if settings.records_integrity() {
        let mut total = None;
        for event in &request.events {
            if !is_known_event(&event.kind) {
                continue;
            }
            let violation = is_violation(&event.kind);
            record_event(
                pool,
                attempt.id,
                &event.kind,
                event.client_at,
                event.meta.clone(),
                violation,
            )
            .await?;
            if event.kind == "offline_end" && settings.max_offline_gap_secs > 0 {
                let gap = event
                    .meta
                    .get("gap_secs")
                    .and_then(Value::as_i64)
                    .unwrap_or(0);
                if gap > settings.max_offline_gap_secs {
                    record_event(
                        pool,
                        attempt.id,
                        "offline_gap_exceeded",
                        event.client_at,
                        json!({"gap_secs": gap}),
                        false,
                    )
                    .await?;
                }
            }
            total = Some(());
        }
        total.map(|_| ())
    } else {
        None
    };
    let _ = violations;

    // Strict assessments can auto-submit after too many violations.
    let limit = settings.auto_submit_after_violations;
    let over_limit = limit > 0 && {
        let count: Option<i32> = sqlx::query_scalar(
            "select (integrity_summary->>'violations')::int from assessment_attempts where id = $1",
        )
        .bind(attempt.id)
        .fetch_one(pool)
        .await?;
        count.unwrap_or(0) >= limit
    };

    if let Some(submit) = &request.submit {
        let offline = submit.offline || matches!(decision, SyncDecision::AcceptFlagged(_));
        let at = match (&decision, submit.claimed_at) {
            (SyncDecision::AcceptFlagged(_), Some(claimed)) => {
                claimed.min(attempt.deadline_at.unwrap_or(claimed))
            }
            _ => now,
        };
        finish(
            pool,
            &attempt,
            &assessment,
            "submitted",
            at,
            offline,
            submit.claimed_at,
        )
        .await?;
    } else if over_limit {
        finish(
            pool,
            &attempt,
            &assessment,
            "auto_submitted",
            now,
            false,
            None,
        )
        .await?;
        response.auto_submitted = true;
    }
    let after = store::find_attempt(pool, attempt.id)
        .await?
        .unwrap_or(attempt);
    response.status = after.status;
    response.receipt = after.receipt;
    response.auto_submitted = response.auto_submitted || after.auto_submitted;
    Ok(response)
}

async fn record_event(
    pool: &PgPool,
    attempt_id: Uuid,
    kind: &str,
    client_at: Option<DateTime<Utc>>,
    meta: Value,
    violation: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "insert into assessment_integrity_events (attempt_id, kind, client_at, meta) values ($1, $2, $3, $4)",
    )
    .bind(attempt_id)
    .bind(kind)
    .bind(client_at)
    .bind(meta)
    .execute(pool)
    .await?;
    sqlx::query(
        "update assessment_attempts set integrity_summary =
           jsonb_set(jsonb_set(integrity_summary, '{violations}',
                to_jsonb(coalesce((integrity_summary->>'violations')::int, 0) + $2)),
                '{events}', to_jsonb(coalesce((integrity_summary->>'events')::int, 0) + 1))
         where id = $1",
    )
    .bind(attempt_id)
    .bind(i32::from(violation))
    .execute(pool)
    .await?;
    Ok(())
}

// ---- finishing an attempt ------------------------------------------------------------------------

/// Submits an attempt the server is closing (assessment closed, time out).
pub async fn force_submit(pool: &PgPool, attempt_id: Uuid) -> Res<()> {
    let attempt = store::find_attempt(pool, attempt_id)
        .await?
        .ok_or(ServiceError::NotFound("Attempt not found"))?;
    let assessment = store::find_assessment(pool, attempt.assessment_id)
        .await?
        .ok_or(ServiceError::NotFound("Assessment not found"))?;
    let now = Utc::now();
    let at = attempt.deadline_at.map_or(now, |d| d.min(now));
    finish(
        pool,
        &attempt,
        &assessment,
        "auto_submitted",
        at,
        false,
        None,
    )
    .await
}

async fn finish(
    pool: &PgPool,
    attempt: &AttemptRow,
    assessment: &AssessmentRow,
    status: &str,
    submitted_at: DateTime<Utc>,
    offline: bool,
    claimed_at: Option<DateTime<Utc>>,
) -> Res<()> {
    let snapshot = store::load_snapshot(pool, attempt.version_id)
        .await?
        .ok_or_else(|| ServiceError::Internal("the attempt's version is missing".to_owned()))?;
    let settings = assessment.settings();
    let accommodation =
        store::accommodation(pool, assessment.id, attempt.student_account_id).await?;
    let due = assessment.window().effective_due(&accommodation);
    let late_secs = due.map_or(0, |d| (submitted_at - d).num_seconds());
    let late = late_outcome(late_secs, &settings);

    let auto = status == "auto_submitted";
    let mut tx = pool.begin().await?;
    // Only one caller gets to finish an attempt. An automatic close is stored as
    // `submitted` plus a flag, so the status can later move on to `graded`
    // without forgetting how the attempt ended.
    let claimed = sqlx::query(
        "update assessment_attempts
         set status = 'submitted', auto_submitted = $7, submitted_at = $2, submitted_offline = $3,
             claimed_submit_at = $4, late = $5,
             integrity_summary = jsonb_set(integrity_summary, '{late_penalty_pct}', to_jsonb($6::float8))
         where id = $1 and status = 'in_progress'",
    )
    .bind(attempt.id)
    .bind(submitted_at)
    .bind(offline)
    .bind(claimed_at)
    .bind(late.late)
    .bind(late.penalty_pct)
    .bind(auto)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if claimed == 0 {
        return Ok(());
    }
    // Every question gets an answer row so grading and reports are uniform.
    for question in &snapshot.questions {
        sqlx::query(
            "insert into attempt_answers (attempt_id, question_id, response) values ($1, $2, '{}'::jsonb)
             on conflict (attempt_id, question_id) do nothing",
        )
        .bind(attempt.id)
        .bind(question.id)
        .execute(&mut *tx)
        .await?;
    }
    let answers: Vec<(Uuid, Uuid, Value)> = sqlx::query_as(
        "select id, question_id, response from attempt_answers where attempt_id = $1 order by question_id",
    )
    .bind(attempt.id)
    .fetch_all(&mut *tx)
    .await?;
    let mut digest = Sha256::new();
    digest.update(attempt.id.as_bytes());
    digest.update(submitted_at.timestamp_millis().to_be_bytes());
    for (answer_id, question_id, response) in &answers {
        digest.update(question_id.as_bytes());
        digest.update(response.to_string().as_bytes());
        let Some(question) = snapshot.question(*question_id) else {
            continue;
        };
        let scored = score_answer(question, response, &settings);
        if scored.needs_review {
            continue; // waits for the lecturer or AI
        }
        let grade_id: Uuid = sqlx::query_scalar(
            "insert into answer_grades (answer_id, source, status, score, max_score, feedback_md)
             values ($1, 'auto', 'confirmed', $2, $3, $4) returning id",
        )
        .bind(answer_id)
        .bind(scored.score)
        .bind(scored.max)
        .bind(&scored.feedback)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query("update attempt_answers set current_grade_id = $2 where id = $1")
            .bind(answer_id)
            .bind(grade_id)
            .execute(&mut *tx)
            .await?;
    }
    let hex = crate::utils::hex_lower(digest.finalize()).to_uppercase();
    let receipt = format!("{}-{}-{}", &hex[0..4], &hex[4..8], &hex[8..12]);
    sqlx::query("update assessment_attempts set receipt = $2 where id = $1")
        .bind(attempt.id)
        .bind(&receipt)
        .execute(&mut *tx)
        .await?;
    store::emit(
        &mut *tx,
        "attempt.submitted",
        json!({"assessment_id": assessment.id, "attempt_id": attempt.id, "late": late.late}),
    )
    .await?;
    tx.commit().await?;
    grading::recompute_attempt(pool, attempt.id).await?;
    Ok(())
}

pub async fn submit(
    pool: &PgPool,
    student_id: Uuid,
    attempt_id: Uuid,
    submit: Option<SubmitRequest>,
    now: DateTime<Utc>,
) -> Res<SyncResponse> {
    sync(
        pool,
        student_id,
        attempt_id,
        SyncRequest {
            submit: Some(submit.unwrap_or(SubmitRequest {
                elapsed_ms: None,
                claimed_at: None,
                offline: false,
            })),
            ..SyncRequest::default()
        },
        now,
    )
    .await
}

// ---- results ------------------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct StudentResult {
    pub attempt_id: Uuid,
    pub assessment_id: Uuid,
    pub title: String,
    pub attempt_no: i32,
    pub status: String,
    pub submitted_at: Option<DateTime<Utc>>,
    pub late: bool,
    pub penalty_pct: f64,
    /// Present once every answer is graded.
    pub score: Option<f64>,
    pub max_score: f64,
    pub percent: Option<f64>,
    pub complete: bool,
    pub visibility: Visibility,
    pub ai_assisted: bool,
    pub questions: Vec<ResultQuestion>,
}

pub async fn result_view(
    pool: &PgPool,
    student_id: Uuid,
    attempt_id: Uuid,
    now: DateTime<Utc>,
) -> Res<StudentResult> {
    let attempt = owned_attempt(pool, student_id, attempt_id).await?;
    if !attempt.is_submitted() {
        return Err(ServiceError::Conflict(
            "Submit the attempt to see a result".to_owned(),
        ));
    }
    let (assessment, snapshot) = load_context(pool, &attempt).await?;
    let settings = assessment.settings();
    if !results_released(
        &assessment.status,
        assessment.results_released_at,
        assessment.due_at,
        &settings,
        now,
    ) {
        return Err(ServiceError::Forbidden(
            "Results have not been released yet",
        ));
    }
    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            Value,
            Option<f64>,
            Option<String>,
            Option<String>,
            Option<String>,
        ),
    >(
        "select aa.question_id, aa.response, g.score, g.status, g.source, g.feedback_md
         from attempt_answers aa left join answer_grades g on g.id = aa.current_grade_id
         where aa.attempt_id = $1",
    )
    .bind(attempt.id)
    .fetch_all(pool)
    .await?;
    let by_q: HashMap<Uuid, _> = rows.into_iter().map(|r| (r.0, r)).collect();
    let order: Vec<Uuid> =
        serde_json::from_value(attempt.question_order.clone()).unwrap_or_default();
    let option_orders: HashMap<String, Vec<String>> =
        serde_json::from_value(attempt.option_orders.clone()).unwrap_or_default();
    let visibility = settings.visibility;
    let mut ai_assisted = false;
    let mut questions = Vec::new();
    for (i, id) in order.iter().enumerate() {
        let Some(question) = snapshot.question(*id) else {
            continue;
        };
        let row = by_q.get(id);
        let confirmed =
            row.is_some_and(|r| matches!(r.3.as_deref(), Some("confirmed" | "overridden")));
        let score = row.and_then(|r| r.2).filter(|_| confirmed);
        if row.is_some_and(|r| r.4.as_deref() == Some("ai") && confirmed) {
            ai_assisted = true;
        }
        let opts = option_orders
            .get(&id.to_string())
            .cloned()
            .unwrap_or_default();
        let options = super::model::student_options(question, &opts);
        questions.push(ResultQuestion {
            id: *id,
            position: i,
            question_type: question.question_type,
            prompt_md: question.prompt_md.clone(),
            points: question.points,
            options,
            response: row.map_or(json!({}), |r| r.1.clone()),
            score: if visibility >= Visibility::Correctness {
                score
            } else {
                None
            },
            correct: if visibility >= Visibility::Correctness {
                score.map(|s| s >= question.points - 0.001)
            } else {
                None
            },
            correct_answer: (visibility >= Visibility::Answers)
                .then(|| revealable_answer(question))
                .flatten(),
            feedback_md: (visibility >= Visibility::Feedback)
                .then(|| row.and_then(|r| r.5.clone()).filter(|f| !f.is_empty()))
                .flatten(),
            explanation_md: (visibility >= Visibility::Feedback
                && !question.explanation_md.is_empty())
            .then(|| question.explanation_md.clone()),
            source_ranges: question.source_ranges.clone(),
            ai_assisted: row.is_some_and(|r| r.4.as_deref() == Some("ai")),
        });
    }
    if visibility < Visibility::Correctness {
        questions.clear();
    }
    let complete = attempt.status == "graded";
    let penalty = attempt
        .integrity_summary
        .get("late_penalty_pct")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let max = snapshot.max_score();
    let score = attempt.final_score.filter(|_| complete);
    Ok(StudentResult {
        attempt_id: attempt.id,
        assessment_id: assessment.id,
        title: assessment.title,
        attempt_no: attempt.attempt_no,
        status: attempt.status,
        submitted_at: attempt.submitted_at,
        late: attempt.late,
        penalty_pct: penalty,
        score,
        max_score: max,
        percent: score
            .filter(|_| max > 0.0)
            .map(|s| ((s / max) * 1000.0).round() / 10.0),
        complete,
        visibility,
        ai_assisted,
        questions,
    })
}
