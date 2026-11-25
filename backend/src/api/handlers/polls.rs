//! Interactive lecture content: polls, quizzes and comprehension checks.
//!
//! Lecturers create them by hand or have the AI write one from what was just
//! said; students answer from the live screen; results update for everyone.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    ai::{extract_json, Task, TextRequest, Thinking},
    api::{
        error::{ApiError, LogApiError},
        handlers::sessions::database_session_by_code,
    },
    auth::guard::{AuthenticatedLecturer, OptionalLecturer},
    models::LectureSession,
    state::AppState,
    study,
};

#[derive(Debug, FromRow)]
struct PollRow {
    id: Uuid,
    kind: String,
    question: String,
    options: serde_json::Value,
    correct_index: Option<i32>,
    explanation: Option<String>,
    status: String,
    source: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct PollView {
    pub id: Uuid,
    pub kind: String,
    pub question: String,
    pub options: Vec<String>,
    pub status: String,
    pub source: String,
    pub created_at: DateTime<Utc>,
    pub counts: Vec<i64>,
    pub total: i64,
    /// Hidden from students until the poll is closed (or they have answered a quiz).
    pub correct_index: Option<i32>,
    pub explanation: Option<String>,
    pub my_choice: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub participant_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePoll {
    pub kind: Option<String>,
    pub question: String,
    pub options: Vec<String>,
    pub correct_index: Option<i32>,
    pub explanation: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RespondInput {
    pub participant_id: Uuid,
    pub choice_index: i32,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct GenerateInput {
    pub kind: Option<String>,
}

fn options_of(row: &PollRow) -> Vec<String> {
    row.options
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

async fn view(
    pool: &sqlx::PgPool,
    row: PollRow,
    participant_id: Option<Uuid>,
    reveal: bool,
) -> Result<PollView, ApiError> {
    let options = options_of(&row);
    let tallies = sqlx::query_as::<_, (i32, i64)>(
        "select choice_index, count(*) from session_poll_responses where poll_id = $1 group by choice_index",
    )
    .bind(row.id)
    .fetch_all(pool)
    .await
    .log_internal_error("Failed to tally poll responses")?;
    let mut counts = vec![0i64; options.len()];
    for (index, count) in tallies {
        if let Some(slot) = counts.get_mut(index as usize) {
            *slot = count;
        }
    }
    let my_choice = match participant_id {
        Some(participant_id) => sqlx::query_scalar::<_, i32>(
            "select choice_index from session_poll_responses where poll_id = $1 and participant_id = $2",
        )
        .bind(row.id)
        .bind(participant_id)
        .fetch_optional(pool)
        .await
        .log_internal_error("Failed to load poll response")?,
        None => None,
    };
    let show_answer =
        reveal || row.status == "closed" || (my_choice.is_some() && row.kind == "quiz");
    Ok(PollView {
        total: counts.iter().sum(),
        counts,
        id: row.id,
        kind: row.kind,
        question: row.question,
        options,
        status: row.status,
        source: row.source,
        created_at: row.created_at,
        correct_index: if show_answer { row.correct_index } else { None },
        explanation: if show_answer { row.explanation } else { None },
        my_choice,
    })
}

async fn owned_session(
    pool: &sqlx::PgPool,
    code: &str,
    lecturer: &AuthenticatedLecturer,
) -> Result<LectureSession, ApiError> {
    let session = database_session_by_code(pool, code).await?;
    let owns = sqlx::query_scalar::<_, bool>(
        "select exists(select 1 from lecture_sessions where id = $1 and lecturer_id = $2)",
    )
    .bind(session.id)
    .bind(lecturer.id)
    .fetch_one(pool)
    .await
    .log_internal_error("Failed to verify session ownership for polls")?;
    if !owns {
        return Err(ApiError::not_found("Session not found"));
    }
    Ok(session)
}

const POLL_COLUMNS: &str =
    "id, kind, question, options, correct_index, explanation, status, source, created_at";

pub async fn list(
    State(state): State<AppState>,
    OptionalLecturer(lecturer): OptionalLecturer,
    Path(code): Path<String>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<PollView>>, ApiError> {
    let pool = state.db_pool();
    let session = database_session_by_code(pool, &code).await?;
    let is_owner = match lecturer {
        Some(l) => sqlx::query_scalar::<_, bool>(
            "select exists(select 1 from lecture_sessions where id = $1 and lecturer_id = $2)",
        )
        .bind(session.id)
        .bind(l.id)
        .fetch_one(pool)
        .await
        .unwrap_or(false),
        None => false,
    };
    let rows = sqlx::query_as::<_, PollRow>(&format!(
        "select {POLL_COLUMNS} from session_polls where session_id = $1 order by created_at desc limit 20"
    ))
    .bind(session.id)
    .fetch_all(pool)
    .await
    .log_internal_error("Failed to list polls")?;
    let mut views = Vec::with_capacity(rows.len());
    for row in rows {
        views.push(view(pool, row, query.participant_id, is_owner).await?);
    }
    Ok(Json(views))
}

fn validate(input: &CreatePoll) -> Result<(String, Vec<String>), ApiError> {
    let kind = input.kind.as_deref().unwrap_or("poll");
    if !["poll", "quiz", "comprehension"].contains(&kind) {
        return Err(ApiError::bad_request("Unknown poll type"));
    }
    let question = input.question.trim();
    if question.is_empty() || question.len() > 500 {
        return Err(ApiError::bad_request(
            "Enter a question under 500 characters",
        ));
    }
    let options: Vec<String> = input
        .options
        .iter()
        .map(|o| o.trim().to_owned())
        .filter(|o| !o.is_empty())
        .collect();
    if !(2..=6).contains(&options.len()) {
        return Err(ApiError::bad_request(
            "Provide between 2 and 6 answer options",
        ));
    }
    if let Some(correct) = input.correct_index {
        if correct < 0 || correct as usize >= options.len() {
            return Err(ApiError::bad_request("Correct answer is out of range"));
        }
    }
    if kind == "quiz" && input.correct_index.is_none() {
        return Err(ApiError::bad_request("Quizzes need a correct answer"));
    }
    Ok((kind.to_owned(), options))
}

async fn insert_poll(
    pool: &sqlx::PgPool,
    session_id: Uuid,
    input: &CreatePoll,
    source: &str,
) -> Result<PollRow, ApiError> {
    let (kind, options) = validate(input)?;
    // One live question at a time keeps the student screen uncluttered.
    sqlx::query(
        "update session_polls set status = 'closed', closed_at = now() where session_id = $1 and status = 'open'",
    )
    .bind(session_id)
    .execute(pool)
    .await
    .log_internal_error("Failed to close earlier polls")?;
    sqlx::query_as::<_, PollRow>(&format!(
        "insert into session_polls (id, session_id, kind, question, options, correct_index, explanation, source)
         values ($1, $2, $3, $4, $5, $6, $7, $8) returning {POLL_COLUMNS}"
    ))
    .bind(Uuid::now_v7())
    .bind(session_id)
    .bind(kind)
    .bind(input.question.trim())
    .bind(serde_json::json!(options))
    .bind(input.correct_index)
    .bind(input.explanation.as_deref().map(str::trim).filter(|e| !e.is_empty()))
    .bind(source)
    .fetch_one(pool)
    .await
    .log_internal_error("Failed to create poll")
}

pub async fn create(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(code): Path<String>,
    Json(input): Json<CreatePoll>,
) -> Result<(StatusCode, Json<PollView>), ApiError> {
    let pool = state.db_pool();
    let session = owned_session(pool, &code, &lecturer).await?;
    let row = insert_poll(pool, session.id, &input, "lecturer").await?;
    Ok((
        StatusCode::CREATED,
        Json(view(pool, row, None, true).await?),
    ))
}

/// Writes one multiple-choice question from the most recent lecture content.
pub async fn generate(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(code): Path<String>,
    body: axum::body::Bytes,
) -> Result<(StatusCode, Json<PollView>), ApiError> {
    let pool = state.db_pool();
    let session = owned_session(pool, &code, &lecturer).await?;
    let kind = serde_json::from_slice::<GenerateInput>(&body)
        .ok()
        .and_then(|i| i.kind)
        .filter(|k| ["quiz", "comprehension", "poll"].contains(&k.as_str()))
        .unwrap_or_else(|| "quiz".to_owned());

    let transcript = study::load_transcript(pool, session.id)
        .await
        .log_internal_error("Failed to load captions for quiz generation")?;
    if transcript.text.trim().len() < 120 {
        return Err(ApiError::conflict(
            "Not enough lecture content yet. Speak for a little longer, then try again.",
        ));
    }
    // Recent content only: the quiz should test what was just taught.
    let recent: String = {
        let all = &transcript.text;
        let count = all.chars().count();
        all.chars().skip(count.saturating_sub(3_500)).collect()
    };
    let system = "You are KLASYNC, a teaching assistant writing a quick in-class check. From the lecture excerpt write ONE multiple-choice question with 4 options testing the most important idea just covered. Return ONLY a valid JSON object: {\"question\": string, \"options\": [4 strings], \"correct_index\": integer 0-3, \"explanation\": \"one sentence why\"}. No markdown fences.";
    let request = TextRequest::new(Task::Quiz, system, recent)
        .json()
        .max_tokens(900)
        .thinking(Thinking::Low)
        .temperature(0.4)
        .for_session(session.id);
    // A question with a wrong "correct answer" in front of a class is worse than
    // none, so validate the shape and escalate to a stronger model if it is off.
    let (input, _) = state
        .ai
        .generate_validated(&request, |output| {
            let parsed = extract_json(&output.text).ok_or("not valid JSON")?;
            let candidate = CreatePoll {
                kind: Some(kind.clone()),
                question: parsed
                    .get("question")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_owned(),
                options: parsed
                    .get("options")
                    .and_then(|v| v.as_array())
                    .map(|a| a.iter().filter_map(|o| o.as_str().map(str::to_owned)).collect())
                    .unwrap_or_default(),
                correct_index: parsed
                    .get("correct_index")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32),
                explanation: parsed
                    .get("explanation")
                    .and_then(|v| v.as_str())
                    .map(str::to_owned),
            };
            validate(&candidate)
                .map(|_| candidate)
                .map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| {
            tracing::warn!(%error, "Quiz generation failed");
            ApiError::conflict("The AI model could not write a question right now. Try again in a moment or add one manually.")
        })?;
    let row = insert_poll(pool, session.id, &input, "ai")
        .await
        .map_err(|_| {
            ApiError::conflict("The AI model returned an unusable question. Please try again.")
        })?;
    Ok((
        StatusCode::CREATED,
        Json(view(pool, row, None, true).await?),
    ))
}

pub async fn respond(
    State(state): State<AppState>,
    Path((code, poll_id)): Path<(String, Uuid)>,
    Json(input): Json<RespondInput>,
) -> Result<Json<PollView>, ApiError> {
    let pool = state.db_pool();
    let session = database_session_by_code(pool, &code).await?;
    let valid_participant = sqlx::query_scalar::<_, bool>(
        "select exists(select 1 from session_participants
                       where id = $1 and session_id = $2 and removed_at is null and muted_at is null)",
    )
    .bind(input.participant_id)
    .bind(session.id)
    .fetch_one(pool)
    .await
    .log_internal_error("Failed to validate poll participant")?;
    if !valid_participant {
        return Err(ApiError::forbidden("Join the session before answering"));
    }
    let row = sqlx::query_as::<_, PollRow>(&format!(
        "select {POLL_COLUMNS} from session_polls where id = $1 and session_id = $2"
    ))
    .bind(poll_id)
    .bind(session.id)
    .fetch_optional(pool)
    .await
    .log_internal_error("Failed to load poll")?
    .ok_or_else(|| ApiError::not_found("Poll not found"))?;
    if row.status != "open" {
        return Err(ApiError::conflict("This question is closed"));
    }
    if input.choice_index < 0 || input.choice_index as usize >= options_of(&row).len() {
        return Err(ApiError::bad_request("Choose one of the listed answers"));
    }
    let inserted = sqlx::query(
        "insert into session_poll_responses (poll_id, participant_id, choice_index)
         values ($1, $2, $3) on conflict (poll_id, participant_id) do nothing",
    )
    .bind(poll_id)
    .bind(input.participant_id)
    .bind(input.choice_index)
    .execute(pool)
    .await
    .log_internal_error("Failed to record poll response")?;
    if inserted.rows_affected() == 0 {
        return Err(ApiError::conflict(
            "You have already answered this question",
        ));
    }
    Ok(Json(
        view(pool, row, Some(input.participant_id), false).await?,
    ))
}

pub async fn close(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path((code, poll_id)): Path<(String, Uuid)>,
) -> Result<Json<PollView>, ApiError> {
    let pool = state.db_pool();
    let session = owned_session(pool, &code, &lecturer).await?;
    let row = sqlx::query_as::<_, PollRow>(&format!(
        "update session_polls set status = 'closed', closed_at = coalesce(closed_at, now())
         where id = $1 and session_id = $2 returning {POLL_COLUMNS}"
    ))
    .bind(poll_id)
    .bind(session.id)
    .fetch_optional(pool)
    .await
    .log_internal_error("Failed to close poll")?
    .ok_or_else(|| ApiError::not_found("Poll not found"))?;
    Ok(Json(view(pool, row, None, true).await?))
}
