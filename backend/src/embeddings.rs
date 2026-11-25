//! Semantic search over lectures (pgvector).
//!
//! A lecture transcript is split into overlapping passages of a few sentences,
//! each embedded and stored with its start time. Questions are embedded the same
//! way and matched by cosine distance, which powers "ask this lecture" (answers
//! grounded in retrieved passages instead of the whole transcript) and search
//! across every lecture a student can access.

use serde::Serialize;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::{
    ai::{EmbedItem, EmbedKind},
    ai_worker::lecture_text,
    state::AppState,
    study,
};

/// Must match `vector(1536)` in migration 0013.
pub const DIMENSIONS: usize = 1536;
const TARGET_CHARS: usize = 900;

#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    pub index: i32,
    pub start_sec: i32,
    pub end_sec: i32,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Hit {
    pub session_id: Uuid,
    pub session_title: String,
    pub course_code: String,
    pub start_sec: i32,
    pub end_sec: i32,
    pub text: String,
    pub score: f64,
}

fn parse_line(line: &str, previous: i32) -> (i32, String) {
    match study::leading_stamp(line) {
        Some(stamp) => {
            let seconds = stamp
                .split_once(':')
                .and_then(|(m, s)| {
                    Some(m.trim().parse::<i32>().ok()? * 60 + s.trim().parse::<i32>().ok()?)
                })
                .unwrap_or(previous);
            let text = line.trim_start();
            let text = text
                .find(']')
                .map(|end| text[end + 1..].trim())
                .unwrap_or(text);
            (seconds, text.to_owned())
        }
        None => (previous, line.trim().to_owned()),
    }
}

/// Groups transcript lines into passages of roughly `TARGET_CHARS`, carrying
/// the last line into the next passage so an idea split across a boundary is
/// still retrievable.
pub fn chunk_transcript(text: &str) -> Vec<Chunk> {
    let mut lines: Vec<(i32, String)> = Vec::new();
    let mut previous = 0;
    for raw in text.lines().filter(|line| !line.trim().is_empty()) {
        let (offset, body) = parse_line(raw, previous);
        previous = offset;
        if !body.is_empty() {
            lines.push((offset, body));
        }
    }
    let mut chunks: Vec<Chunk> = Vec::new();
    let mut current: Vec<(i32, String)> = Vec::new();
    let mut size = 0usize;
    let flush = |current: &Vec<(i32, String)>, chunks: &mut Vec<Chunk>| {
        if current.is_empty() {
            return;
        }
        chunks.push(Chunk {
            index: chunks.len() as i32,
            start_sec: current.first().map(|l| l.0).unwrap_or(0),
            end_sec: current.last().map(|l| l.0).unwrap_or(0),
            text: current
                .iter()
                .map(|l| l.1.as_str())
                .collect::<Vec<_>>()
                .join(" "),
        });
    };
    for line in lines {
        size += line.1.len() + 1;
        current.push(line);
        if size >= TARGET_CHARS {
            flush(&current, &mut chunks);
            let overlap = current.pop();
            current.clear();
            size = 0;
            if let Some(overlap) = overlap {
                size += overlap.1.len() + 1;
                current.push(overlap);
            }
        }
    }
    // A tail that is only the carried-over line adds nothing.
    if current.len() > 1 || (current.len() == 1 && chunks.is_empty()) {
        flush(&current, &mut chunks);
    }
    chunks
}

/// pgvector's text input format.
pub fn vector_literal(vector: &[f32]) -> String {
    let mut out = String::with_capacity(vector.len() * 10 + 2);
    out.push('[');
    for (i, value) in vector.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&value.to_string());
    }
    out.push(']');
    out
}

fn ensure_ready(state: &AppState) -> Result<String, String> {
    let (model, dimensions) = state
        .ai
        .embedding_model()
        .ok_or_else(|| "Semantic search is not configured on this server.".to_owned())?;
    if dimensions != DIMENSIONS {
        return Err(format!(
            "EMBEDDING_DIMENSIONS must be {DIMENSIONS} to match the database schema (got {dimensions})."
        ));
    }
    Ok(model)
}

/// (Re)builds the search index for one lecture. Returns the passage count.
pub async fn embed_session(
    state: &AppState,
    pool: &PgPool,
    session_id: Uuid,
) -> Result<usize, String> {
    let model = ensure_ready(state)?;
    let transcript = lecture_text(state, pool, session_id, None).await?;
    let chunks = chunk_transcript(&transcript.text);
    if chunks.is_empty() {
        return Err("There is no transcript text to index.".to_owned());
    }
    let title = sqlx::query_scalar::<_, String>("select title from lecture_sessions where id = $1")
        .bind(session_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| error.to_string())?;
    let items: Vec<EmbedItem> = chunks
        .iter()
        .map(|chunk| EmbedItem {
            text: chunk.text.clone(),
            title: title.clone(),
        })
        .collect();
    let embedded = state
        .ai
        .embed(&items, EmbedKind::Document, Some(session_id))
        .await
        .map_err(|error| error.to_string())?;
    if embedded.vectors.len() != chunks.len() {
        return Err("the embedding service returned the wrong number of vectors".to_owned());
    }

    let mut tx = pool.begin().await.map_err(|error| error.to_string())?;
    sqlx::query("delete from lecture_chunks where session_id = $1 and model = $2")
        .bind(session_id)
        .bind(&embedded.model)
        .execute(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
    for (chunk, vector) in chunks.iter().zip(&embedded.vectors) {
        sqlx::query(
            "insert into lecture_chunks (session_id, chunk_index, start_sec, end_sec, text, embedding, model)
             values ($1, $2, $3, $4, $5, $6::vector, $7)",
        )
        .bind(session_id)
        .bind(chunk.index)
        .bind(chunk.start_sec)
        .bind(chunk.end_sec)
        .bind(&chunk.text)
        .bind(vector_literal(vector))
        .bind(&embedded.model)
        .execute(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
    }
    tx.commit().await.map_err(|error| error.to_string())?;
    let _ = model;
    Ok(chunks.len())
}

async fn embed_query(
    state: &AppState,
    query: &str,
    session_id: Option<Uuid>,
) -> Result<String, String> {
    let embedded = state
        .ai
        .embed(
            &[EmbedItem {
                text: query.to_owned(),
                title: None,
            }],
            EmbedKind::Query,
            session_id,
        )
        .await
        .map_err(|error| error.to_string())?;
    embedded
        .vectors
        .first()
        .map(|vector| vector_literal(vector))
        .ok_or_else(|| "the embedding service returned nothing".to_owned())
}

/// Closest passages within one lecture. Sorting by `distance + 0` keeps the
/// planner from using the HNSW index, whose approximate results can miss rows
/// once a selective filter is applied; a per-lecture exact scan is tiny.
pub async fn search_session(
    state: &AppState,
    pool: &PgPool,
    session_id: Uuid,
    query: &str,
    limit: i64,
) -> Result<Vec<Hit>, String> {
    let model = ensure_ready(state)?;
    let literal = embed_query(state, query, Some(session_id)).await?;
    sqlx::query_as::<_, Hit>(
        "select c.session_id, s.title as session_title, co.code as course_code,
                c.start_sec, c.end_sec, c.text,
                (1 - (c.embedding <=> $1::vector))::float8 as score
         from lecture_chunks c
         join lecture_sessions s on s.id = c.session_id
         join courses co on co.id = s.course_id
         where c.session_id = $2 and c.model = $3
         order by (c.embedding <=> $1::vector) + 0
         limit $4",
    )
    .bind(literal)
    .bind(session_id)
    .bind(model)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())
}

pub async fn has_index(pool: &PgPool, session_id: Uuid, model: &str) -> bool {
    sqlx::query_scalar::<_, bool>(
        "select exists(select 1 from lecture_chunks where session_id = $1 and model = $2)",
    )
    .bind(session_id)
    .bind(model)
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

/// Closest passages across every lecture the student may open.
pub async fn search_for_student(
    state: &AppState,
    pool: &PgPool,
    student_id: Uuid,
    query: &str,
    limit: i64,
) -> Result<Vec<Hit>, String> {
    let model = ensure_ready(state)?;
    let literal = embed_query(state, query, None).await?;
    sqlx::query_as::<_, Hit>(
        "with accessible as (
           select s.id from lecture_sessions s
           where s.deleted_at is null
             and (
               exists(select 1 from session_participants p
                      where p.session_id = s.id
                        and (p.student_account_id = $2
                             or exists(select 1 from student_session_claims cl
                                       where cl.participant_id = p.id and cl.student_account_id = $2)))
               or (s.status = 'ended' and exists(
                     select 1 from student_course_enrollments e
                     where e.course_id = s.course_id and e.student_account_id = $2
                       and e.enrollment_type in ('claimed', 'roster_matched')))
             )
         )
         select c.session_id, s.title as session_title, co.code as course_code,
                c.start_sec, c.end_sec, c.text,
                (1 - (c.embedding <=> $1::vector))::float8 as score
         from lecture_chunks c
         join accessible a on a.id = c.session_id
         join lecture_sessions s on s.id = c.session_id
         join courses co on co.id = s.course_id
         where c.model = $3
         order by (c.embedding <=> $1::vector) + 0
         limit $4",
    )
    .bind(literal)
    .bind(student_id)
    .bind(model)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_carry_offsets_and_overlap() {
        let text = (0..30)
            .map(|i| {
                format!(
                    "[{:02}:{:02}] This is sentence number {i} about photosynthesis in plants.",
                    i / 2,
                    (i % 2) * 30
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let chunks = chunk_transcript(&text);
        assert!(chunks.len() >= 2);
        assert_eq!(chunks[0].start_sec, 0);
        assert!(chunks[1].start_sec <= chunks[0].end_sec, "passages overlap");
        assert!(chunks.iter().all(|c| !c.text.contains('[')));
        assert!(chunks.iter().enumerate().all(|(i, c)| c.index == i as i32));
    }

    #[test]
    fn unstamped_text_still_chunks() {
        let chunks = chunk_transcript("One line only.");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].start_sec, 0);
    }

    #[test]
    fn vector_literal_matches_pgvector_format() {
        assert_eq!(vector_literal(&[0.5, -1.0, 0.25]), "[0.5,-1,0.25]");
    }
}
