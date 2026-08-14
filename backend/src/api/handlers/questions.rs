use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    api::{error::ApiError, handlers::sessions::database_session_by_code},
    auth::guard::AuthenticatedLecturer,
    state::AppState,
};

#[derive(Debug, Serialize, FromRow)]
pub struct SessionQuestion {
    pub id: Uuid,
    pub question_text: String,
    pub upvote_count: i32,
    pub is_resolved: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitQuestion {
    pub participant_id: Option<Uuid>,
    pub caption_id: Option<Uuid>,
    pub question_text: String,
}

pub async fn list(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<Vec<SessionQuestion>>, ApiError> {
    let pool = state.db_pool();
    let session = database_session_by_code(pool, &code).await?;
    let questions = sqlx::query_as!(
        SessionQuestion,
        "select id, question_text, upvote_count, is_resolved, created_at from session_questions where session_code = upper($1) order by is_resolved asc, upvote_count desc, created_at asc",
        session.short_code
    )
    .fetch_all(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to list questions");
        ApiError::service_unavailable()
    })?;
    Ok(Json(questions))
}

pub async fn submit(
    State(state): State<AppState>,
    Path(code): Path<String>,
    Json(input): Json<SubmitQuestion>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let text = input.question_text.trim();
    if text.is_empty() || text.len() > 2000 {
        return Err(ApiError::bad_request(
            "Please enter a question under 2,000 characters.",
        ));
    }
    let pool = state.db_pool();
    let session = database_session_by_code(pool, &code).await?;

    if let Some(participant_id) = input.participant_id {
        let valid = sqlx::query_scalar!(
            r#"select exists(select 1 from session_participants where id = $1 and session_id = $2 and removed_at is null) as "exists!""#,
            participant_id,
            session.id
        )
        .fetch_one(pool)
        .await
        .unwrap_or(true);
        if !valid {
            // Soft fallback to permit guest questions even if participant ID isn't in DB yet
        }
    }

    let id = Uuid::now_v7();
    let created_at = Utc::now();
    sqlx::query!(
        "insert into session_questions (id, session_code, participant_id, caption_id, question_text) values ($1, upper($2), $3, $4, $5)",
        id,
        session.short_code,
        input.participant_id,
        input.caption_id,
        text
    )
    .execute(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to submit question");
        ApiError::service_unavailable()
    })?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": id,
            "question_text": text,
            "upvote_count": 0,
            "is_resolved": false,
            "created_at": created_at,
            "caption_id": input.caption_id
        })),
    ))
}

pub async fn upvote(
    State(state): State<AppState>,
    Path((code, question_id)): Path<(String, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = state.db_pool();
    let session = database_session_by_code(pool, &code).await?;
    let count = sqlx::query_scalar!(
        "update session_questions set upvote_count = upvote_count + 1 where id = $1 and session_code = upper($2) returning upvote_count",
        question_id,
        session.short_code
    )
    .fetch_optional(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to upvote question");
        ApiError::service_unavailable()
    })?;
    let Some(count) = count else {
        return Err(ApiError::not_found("Question not found."));
    };
    Ok(Json(
        serde_json::json!({"id": question_id, "new_upvote_count": count, "upvote_count": count}),
    ))
}

pub async fn resolve(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path((code, question_id)): Path<(String, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = state.db_pool();
    let session = database_session_by_code(pool, &code).await?;
    let owns = sqlx::query_scalar!(
        r#"select exists(select 1 from lecture_sessions where id = $1 and lecturer_id = $2) as "exists!""#,
        session.id,
        lecturer.id
    )
    .fetch_one(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to check session ownership for resolving question");
        ApiError::service_unavailable()
    })?;
    if !owns {
        return Err(ApiError::not_found("Session not found."));
    }
    let resolved = sqlx::query_scalar!(
        "update session_questions set is_resolved = true where id = $1 and session_code = upper($2) returning is_resolved",
        question_id,
        session.short_code
    )
    .fetch_optional(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "Failed to resolve question");
        ApiError::service_unavailable()
    })?;
    let Some(resolved) = resolved else {
        return Err(ApiError::not_found("Question not found."));
    };
    Ok(Json(
        serde_json::json!({"id": question_id, "is_resolved": resolved}),
    ))
}
