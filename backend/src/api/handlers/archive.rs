//! Student archive reads: transcript, recording playback, rolling summaries.

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::Response,
    Json,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    api::{
        error::{ApiError, LogApiError},
        handlers::ai_study::require_access,
    },
    auth::guard::AuthenticatedStudent,
    state::AppState,
    study,
};

#[derive(Debug, Serialize)]
pub struct TranscriptLine {
    pub offset_sec: Option<i64>,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct TranscriptResponse {
    pub session_id: Uuid,
    pub source: &'static str,
    pub duration_sec: i64,
    pub lines: Vec<TranscriptLine>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct RecordingInfo {
    pub resource_id: Uuid,
    pub content_type: Option<String>,
    pub byte_size: Option<i64>,
    pub duration_sec: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct RollingSummary {
    pub id: Uuid,
    pub sequence: i32,
    pub summary: String,
    pub key_points: serde_json::Value,
    pub source: String,
    pub created_at: DateTime<Utc>,
}

pub async fn transcript(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Path(session_id): Path<Uuid>,
) -> Result<Json<TranscriptResponse>, ApiError> {
    let pool = state.db_pool();
    require_access(pool, &student, session_id).await?;

    let loaded = study::load_transcript(pool, session_id)
        .await
        .log_internal_error("Failed to load session transcript")?;
    if !loaded.text.is_empty() {
        let lines = loaded
            .text
            .lines()
            .map(|line| {
                // `[mm:ss] text`
                let (offset, text) = match line.strip_prefix('[').and_then(|l| l.split_once("] ")) {
                    Some((stamp, rest)) => {
                        let seconds = stamp.split_once(':').and_then(|(m, s)| {
                            Some(m.parse::<i64>().ok()? * 60 + s.parse::<i64>().ok()?)
                        });
                        (seconds, rest.to_owned())
                    }
                    None => (None, line.to_owned()),
                };
                TranscriptLine {
                    offset_sec: offset,
                    text,
                }
            })
            .collect();
        return Ok(Json(TranscriptResponse {
            session_id,
            source: "captions",
            duration_sec: loaded.duration_sec as i64,
            lines,
        }));
    }

    let resource = sqlx::query_scalar::<_, String>(
        "select content::text from lecture_resources
         where session_id = $1 and resource_type = 'transcript' and content is not null
         order by created_at desc limit 1",
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await
    .log_internal_error("Failed to load transcript resource")?;
    let lines: Vec<TranscriptLine> = resource
        .map(|content| study::text_from_resource_json(&content))
        .unwrap_or_default()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| TranscriptLine {
            offset_sec: None,
            text: line.trim().to_owned(),
        })
        .collect();
    Ok(Json(TranscriptResponse {
        session_id,
        source: if lines.is_empty() {
            "none"
        } else {
            "transcript"
        },
        duration_sec: 0,
        lines,
    }))
}

pub async fn summaries(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Path(session_id): Path<Uuid>,
) -> Result<Json<Vec<RollingSummary>>, ApiError> {
    let pool = state.db_pool();
    require_access(pool, &student, session_id).await?;
    let rows = sqlx::query_as::<_, RollingSummary>(
        "select id, sequence, summary, key_points, source, created_at
         from session_rolling_summaries where session_id = $1 order by sequence",
    )
    .bind(session_id)
    .fetch_all(pool)
    .await
    .log_internal_error("Failed to load rolling summaries")?;
    Ok(Json(rows))
}

/// All recording segments for a session in playback order. A lecturer who
/// pauses and resumes the microphone produces several recordings.
async fn recordings(pool: &sqlx::PgPool, session_id: Uuid) -> Result<Vec<RecordingInfo>, ApiError> {
    sqlx::query_as::<_, RecordingInfo>(
        "select id as resource_id, content_type, byte_size, duration_sec, created_at
         from lecture_resources
         where session_id = $1 and resource_type in ('audio', 'recording') and storage_key is not null
           and (expires_at is null or expires_at > now())
         order by created_at asc",
    )
    .bind(session_id)
    .fetch_all(pool)
    .await
    .log_internal_error("Failed to look up lecture recordings")
}

pub async fn recording_info(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Path(session_id): Path<Uuid>,
) -> Result<Json<Vec<RecordingInfo>>, ApiError> {
    let pool = state.db_pool();
    require_access(pool, &student, session_id).await?;
    Ok(Json(recordings(pool, session_id).await?))
}

/// Streams a lecture recording inline with HTTP Range support so browsers can
/// seek. (The previous player pointed at an `/audio/chunk/1` URL that did not
/// exist, which is why playback always reported "not found".)
pub async fn recording_stream(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Path((session_id, resource_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let pool = state.db_pool();
    require_access(pool, &student, session_id).await?;
    let (content_type, key) = sqlx::query_as::<_, (Option<String>, String)>(
        "select content_type, storage_key from lecture_resources
         where id = $1 and session_id = $2 and resource_type in ('audio', 'recording')
           and storage_key is not null and (expires_at is null or expires_at > now())",
    )
    .bind(resource_id)
    .bind(session_id)
    .fetch_optional(pool)
    .await
    .log_internal_error("Failed to look up lecture recording")?
    .ok_or_else(|| ApiError::not_found("No recording was captured for this lecture"))?;
    let bytes = state.storage.get(&key).await.map_err(|error| {
        tracing::error!(%error, %key, "Failed to read lecture recording from storage");
        ApiError::not_found("The lecture recording could not be retrieved")
    })?;
    let total = bytes.len();
    let content_type = content_type
        .filter(|value| value.starts_with("audio/") || value.starts_with("video/"))
        .unwrap_or_else(|| "audio/webm".to_owned());

    let range = headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| parse_range(value, total));

    let builder = Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CACHE_CONTROL, "private, max-age=300");
    let response = match range {
        Some((start, end)) => builder
            .status(StatusCode::PARTIAL_CONTENT)
            .header(
                header::CONTENT_RANGE,
                format!("bytes {start}-{end}/{total}"),
            )
            .header(header::CONTENT_LENGTH, end - start + 1)
            .body(Body::from(bytes[start..=end].to_vec())),
        None => builder
            .status(StatusCode::OK)
            .header(header::CONTENT_LENGTH, total)
            .body(Body::from(bytes)),
    };
    response.log_internal_error("Failed to build recording response")
}

/// Parses a single `bytes=start-end` range.
fn parse_range(value: &str, total: usize) -> Option<(usize, usize)> {
    if total == 0 {
        return None;
    }
    let spec = value.strip_prefix("bytes=")?;
    let (start, end) = spec.split_once('-')?;
    if start.is_empty() {
        let suffix: usize = end.parse().ok()?;
        return Some((total.saturating_sub(suffix.min(total)), total - 1));
    }
    let start: usize = start.parse().ok()?;
    let end: usize = if end.is_empty() {
        total - 1
    } else {
        end.parse::<usize>().ok()?.min(total - 1)
    };
    if start > end || start >= total {
        return None;
    }
    Some((start, end))
}

#[cfg(test)]
mod tests {
    use super::parse_range;

    #[test]
    fn parses_ranges() {
        assert_eq!(parse_range("bytes=0-9", 100), Some((0, 9)));
        assert_eq!(parse_range("bytes=90-", 100), Some((90, 99)));
        assert_eq!(parse_range("bytes=-10", 100), Some((90, 99)));
        assert_eq!(parse_range("bytes=0-999", 100), Some((0, 99)));
        assert_eq!(parse_range("bytes=200-300", 100), None);
    }
}
