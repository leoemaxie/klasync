//! Rolling lecture summaries.
//!
//! While a lecture is live, new captions are condensed every minute or so into
//! a short summary + key points that students see beside the live captions
//! (and keep in their archive). The summariser uses the configured AI model and
//! falls back to an extractive summary so students always get something, even
//! when the free model tier is rate-limited.

use std::time::Duration;

use serde_json::json;
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    activity,
    ai::{extract_json, Task, TextRequest, Thinking},
    state::AppState,
    study,
};

const TICK: Duration = Duration::from_secs(20);
/// Summarise once this much new speech has accumulated...
const MIN_NEW_CHARS: usize = 350;
/// ...but never more often than this.
const MIN_SECONDS_BETWEEN: i64 = 45;
/// And flush whatever is left once the lecture has ended.
const FINAL_FLUSH_WINDOW_MINUTES: i32 = 45;

#[derive(Debug, FromRow)]
struct Candidate {
    id: Uuid,
    status: String,
}

#[derive(Debug, FromRow)]
struct LastSummary {
    sequence: i32,
    covers_through_caption: i64,
    summary: String,
    age_seconds: i64,
}

#[derive(Debug, FromRow)]
struct NewCaption {
    sequence_number: i64,
    text: String,
    offset_sec: i64,
}

pub async fn run_loop(state: AppState) {
    loop {
        activity::sleep(TICK).await;
        // Idle service (no lecture can be live): leave the database alone.
        if !activity::db_active() {
            continue;
        }
        let pool = state.db_pool();
        let candidates = sqlx::query_as::<_, Candidate>(
            "select id, status::text as status from lecture_sessions
             where deleted_at is null
               and (status = 'live'
                    or (status = 'ended' and ended_at > now() - make_interval(mins => $1)))",
        )
        .bind(FINAL_FLUSH_WINDOW_MINUTES)
        .fetch_all(pool)
        .await;
        let Ok(candidates) = candidates else { continue };
        for candidate in candidates {
            if let Err(error) = summarise_session(&state, &candidate).await {
                tracing::warn!(session_id = %candidate.id, %error, "Rolling summary failed");
            }
        }
    }
}

async fn summarise_session(state: &AppState, candidate: &Candidate) -> Result<(), String> {
    let pool = state.db_pool();
    let session_id = candidate.id;

    let last = sqlx::query_as::<_, LastSummary>(
        "select sequence, covers_through_caption, summary,
                extract(epoch from (now() - created_at))::bigint as age_seconds
         from session_rolling_summaries where session_id = $1 order by sequence desc limit 1",
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;
    let covered = last.as_ref().map(|l| l.covers_through_caption).unwrap_or(0);

    let fresh = sqlx::query_as::<_, NewCaption>(
        "select c.sequence_number, c.text,
                greatest(0, extract(epoch from (c.created_at - coalesce(s.started_at, c.created_at)))::bigint) as offset_sec
         from caption_chunks c join lecture_sessions s on s.id = c.session_id
         where c.session_id = $1 and c.sequence_number > $2 and not c.is_hidden
         order by c.sequence_number",
    )
    .bind(session_id)
    .bind(covered)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    if fresh.is_empty() {
        return Ok(());
    }

    let chars: usize = fresh.iter().map(|c| c.text.len()).sum();
    let live = candidate.status == "live";
    let age = last.as_ref().map(|l| l.age_seconds).unwrap_or(i64::MAX);
    let due = if live {
        chars >= MIN_NEW_CHARS && age >= MIN_SECONDS_BETWEEN
    } else {
        // Lecture over: summarise the tail once, however short.
        chars >= 80
    };
    if !due {
        return Ok(());
    }

    // Only one instance should do the work for a session at a time.
    let _lock = match &state.redis {
        Some(redis) => match redis
            .try_lock("rolling-summary", &session_id.to_string(), 120)
            .await
        {
            Ok(Some(lock)) => Some(lock),
            Ok(None) => return Ok(()),
            Err(_) => None,
        },
        None => None,
    };

    let transcript = fresh
        .iter()
        .map(|c| format!("[{}] {}", study::format_offset(c.offset_sec), c.text.trim()))
        .collect::<Vec<_>>()
        .join("\n");
    let transcript = study::clamp_text(&transcript, 12_000);
    let previous = last.as_ref().map(|l| l.summary.as_str()).unwrap_or("");
    let user = if previous.is_empty() {
        transcript.clone()
    } else {
        format!("Summary so far:\n{previous}\n\nNew lecture content:\n{transcript}")
    };
    let system = "You are KLASYNC, a live lecture note-taker for students. Summarise ONLY the new lecture content \
        (use the summary-so-far only for context). Return ONLY a valid JSON object: \
        {\"summary\": \"2-3 sentence summary of what was just covered\", \
        \"key_points\": [\"3 to 5 short bullet points: definitions, formulas, examples, instructions\"]}. \
        No markdown fences, no commentary.";

    let request = TextRequest::new(Task::RollingSummary, system, user)
        .json()
        .max_tokens(900)
        .thinking(Thinking::Minimal)
        .for_session(session_id);
    // The cheap model gets the first try; a stronger one only if its JSON is unusable.
    let parsed = state
        .ai
        .generate_validated(&request, |output| {
            let raw = output.text.as_str();
            match extract_json(raw) {
                Some(value) => {
                    let summary = value
                        .get("summary")
                        .and_then(|v| v.as_str())
                        .map(|s| s.trim().to_owned())
                        .filter(|s| !s.is_empty())
                        .ok_or_else(|| "the answer had no summary".to_owned())?;
                    let points: Vec<String> = value
                        .get("key_points")
                        .and_then(|p| p.as_array())
                        .map(|items| {
                            items
                                .iter()
                                .filter_map(|i| i.as_str().map(|s| s.trim().to_owned()))
                                .filter(|s| !s.is_empty())
                                .take(6)
                                .collect()
                        })
                        .unwrap_or_default();
                    Ok((summary, points))
                }
                // Plain-prose answers are still usable.
                None if !raw.trim().is_empty() => {
                    Ok((raw.trim().chars().take(900).collect::<String>(), Vec::new()))
                }
                None => Err("empty answer".to_owned()),
            }
        })
        .await;

    let (summary, key_points, source) = match parsed {
        Ok(((summary, points), _output)) => (summary, points, "ai"),
        Err(error) => {
            tracing::debug!(%error, "Rolling summary AI call failed; using extractive fallback");
            let (summary, points) = study::heuristic_summary(&transcript);
            (summary, points, "heuristic")
        }
    };
    if summary.trim().is_empty() {
        return Ok(());
    }

    let through = fresh.last().map(|c| c.sequence_number).unwrap_or(covered);
    sqlx::query(
        "insert into session_rolling_summaries (id, session_id, sequence, summary, key_points, covers_through_caption, source)
         values ($1, $2, $3, $4, $5, $6, $7)
         on conflict (session_id, sequence) do nothing",
    )
    .bind(Uuid::now_v7())
    .bind(session_id)
    .bind(last.as_ref().map(|l| l.sequence).unwrap_or(0) + 1)
    .bind(summary)
    .bind(json!(key_points))
    .bind(through)
    .bind(source)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    if let Some(lock) = _lock {
        let _ = lock.release().await;
    }
    Ok(())
}
