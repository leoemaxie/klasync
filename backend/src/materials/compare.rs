//! Cross-offering comparison: "what changed in what was taught" between, say,
//! ME 501 in 2024 and ME 501 in 2026.
//!
//! 1. **Align** the sections of both offerings. Section vectors (pgvector
//!    embeddings, computed when a material is published) are used when both
//!    sides have them; otherwise lexical similarity. Alignment is a monotone
//!    dynamic program (order is preserved) followed by a pass that finds
//!    sections that merely *moved*.
//! 2. **Classify** each pair: unchanged, modified, moved, added or removed.
//! 3. **Diff** modified pairs at block level (definitions, examples and
//!    equations added or dropped), not character level.
//! 4. **Report**: a coverage matrix, a change list, drift (topics that
//!    disappeared) and an optional AI-written narrative.
//!
//! The analyzer reads published versions only and never modifies materials.

use std::collections::HashSet;

use async_trait::async_trait;
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    blocks::{Block, MaterialDoc},
    registry::{Analyzer, AnalyzerInput, MaterialEvent, PluginError},
    store,
};
use crate::{
    ai::{EmbedItem, EmbedKind, Task, TextRequest, Thinking},
    ai_worker::PendingJob,
    embeddings,
    state::AppState,
};

/// Below this a pair is not the same topic.
pub const MATCH_THRESHOLD_LEXICAL: f64 = 0.22;
pub const MATCH_THRESHOLD_VECTOR: f64 = 0.72;
/// Above this body similarity a matched section counts as unchanged.
pub const UNCHANGED_THRESHOLD: f64 = 0.9;

// ---- similarity ---------------------------------------------------------------------

const STOP: &[&str] = &[
    "the", "and", "for", "that", "with", "this", "from", "are", "was", "were", "has", "have",
    "will", "can", "not", "but", "its", "you", "your", "they", "their", "which", "when", "what",
    "how", "also", "into", "than", "then", "there", "these", "those", "such", "each", "over",
];

pub fn tokens(text: &str) -> HashSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .map(str::to_lowercase)
        .filter(|w| w.chars().count() >= 3 && !STOP.contains(&w.as_str()))
        .collect()
}

/// Jaccard overlap of the two texts' content words, 0..=1.
pub fn token_jaccard(a: &str, b: &str) -> f64 {
    let (a, b) = (tokens(a), tokens(b));
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let intersection = a.intersection(&b).count() as f64;
    let union = a.union(&b).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

pub fn cosine(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let (mut dot, mut na, mut nb) = (0.0f64, 0.0f64, 0.0f64);
    for (x, y) in a.iter().zip(b) {
        dot += (*x as f64) * (*y as f64);
        na += (*x as f64).powi(2);
        nb += (*y as f64).powi(2);
    }
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        (dot / (na.sqrt() * nb.sqrt())).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone)]
pub struct SectionData {
    pub lecture_title: String,
    pub version_id: Uuid,
    pub block_id: String,
    pub heading: String,
    pub body: String,
    pub blocks: Vec<Block>,
    pub vector: Option<Vec<f32>>,
}

fn section_similarity(a: &SectionData, b: &SectionData) -> (f64, bool) {
    if let (Some(va), Some(vb)) = (&a.vector, &b.vector) {
        return (cosine(va, vb), true);
    }
    let heading = token_jaccard(&a.heading, &b.heading);
    let body = token_jaccard(&a.body, &b.body);
    (0.35 * heading + 0.65 * body, false)
}

// ---- alignment ----------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Change {
    Unchanged,
    Modified,
    Moved,
    Added,
    Removed,
}

#[derive(Debug, Clone, Serialize)]
pub struct SectionMatch {
    pub a: Option<usize>,
    pub b: Option<usize>,
    pub similarity: f64,
    pub change: Change,
}

/// Aligns two ordered lists of sections.
pub fn align(a: &[SectionData], b: &[SectionData]) -> Vec<SectionMatch> {
    let (n, m) = (a.len(), b.len());
    let mut sim = vec![vec![(0.0f64, false); m]; n];
    for i in 0..n {
        for j in 0..m {
            sim[i][j] = section_similarity(&a[i], &b[j]);
        }
    }
    let ok = |value: (f64, bool)| {
        let threshold = if value.1 { MATCH_THRESHOLD_VECTOR } else { MATCH_THRESHOLD_LEXICAL };
        value.0 >= threshold
    };

    // Monotone alignment: maximise total similarity of order-preserving pairs.
    let mut dp = vec![vec![0.0f64; m + 1]; n + 1];
    for i in 1..=n {
        for j in 1..=m {
            let mut best = dp[i - 1][j].max(dp[i][j - 1]);
            if ok(sim[i - 1][j - 1]) {
                best = best.max(dp[i - 1][j - 1] + sim[i - 1][j - 1].0);
            }
            dp[i][j] = best;
        }
    }
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    let (mut i, mut j) = (n, m);
    while i > 0 && j > 0 {
        if ok(sim[i - 1][j - 1]) && (dp[i][j] - (dp[i - 1][j - 1] + sim[i - 1][j - 1].0)).abs() < 1e-9 {
            pairs.push((i - 1, j - 1));
            i -= 1;
            j -= 1;
        } else if (dp[i][j] - dp[i - 1][j]).abs() < 1e-9 {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    pairs.reverse();

    let mut used_a: HashSet<usize> = pairs.iter().map(|p| p.0).collect();
    let mut used_b: HashSet<usize> = pairs.iter().map(|p| p.1).collect();
    let mut matches: Vec<SectionMatch> = pairs
        .iter()
        .map(|&(i, j)| {
            let body = token_jaccard(&a[i].body, &b[j].body);
            SectionMatch {
                a: Some(i),
                b: Some(j),
                similarity: sim[i][j].0,
                change: if body >= UNCHANGED_THRESHOLD { Change::Unchanged } else { Change::Modified },
            }
        })
        .collect();

    // Moved: unmatched on both sides but clearly the same topic.
    for i in 0..n {
        if used_a.contains(&i) {
            continue;
        }
        let best = (0..m)
            .filter(|j| !used_b.contains(j) && ok(sim[i][*j]))
            .max_by(|x, y| sim[i][*x].0.partial_cmp(&sim[i][*y].0).unwrap_or(std::cmp::Ordering::Equal));
        if let Some(j) = best {
            used_a.insert(i);
            used_b.insert(j);
            matches.push(SectionMatch {
                a: Some(i),
                b: Some(j),
                similarity: sim[i][j].0,
                change: Change::Moved,
            });
        }
    }
    for i in 0..n {
        if !used_a.contains(&i) {
            matches.push(SectionMatch { a: Some(i), b: None, similarity: 0.0, change: Change::Removed });
        }
    }
    for j in 0..m {
        if !used_b.contains(&j) {
            matches.push(SectionMatch { a: None, b: Some(j), similarity: 0.0, change: Change::Added });
        }
    }
    matches.sort_by_key(|x| (x.a.unwrap_or(usize::MAX), x.b.unwrap_or(usize::MAX)));
    matches
}

// ---- block diff ---------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct BlockDiff {
    pub unchanged: usize,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<ChangedBlock>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ChangedBlock {
    pub block_type: &'static str,
    pub before: String,
    pub after: String,
}

fn label(block: &Block) -> String {
    let text = block.plain_text();
    let short: String = text.chars().take(120).collect();
    format!("{}: {short}", block.kind.type_name())
}

/// Block-level diff: matches blocks of the same type by content similarity.
pub fn diff_blocks(a: &[Block], b: &[Block]) -> BlockDiff {
    let mut diff = BlockDiff::default();
    let mut taken: HashSet<usize> = HashSet::new();
    for block in a {
        if matches!(block.kind, super::blocks::BlockKind::Heading { .. }) {
            continue;
        }
        let text = block.plain_text();
        let best = b
            .iter()
            .enumerate()
            .filter(|(j, other)| {
                !taken.contains(j)
                    && other.kind.type_name() == block.kind.type_name()
                    && !matches!(other.kind, super::blocks::BlockKind::Heading { .. })
            })
            .map(|(j, other)| (j, token_jaccard(&text, &other.plain_text())))
            .max_by(|x, y| x.1.partial_cmp(&y.1).unwrap_or(std::cmp::Ordering::Equal));
        match best {
            Some((j, score)) if score >= 0.9 => {
                taken.insert(j);
                diff.unchanged += 1;
            }
            Some((j, score)) if score >= 0.4 => {
                taken.insert(j);
                diff.changed.push(ChangedBlock {
                    block_type: block.kind.type_name(),
                    before: text.chars().take(160).collect(),
                    after: b[j].plain_text().chars().take(160).collect(),
                });
            }
            _ => diff.removed.push(label(block)),
        }
    }
    for (j, block) in b.iter().enumerate() {
        if !taken.contains(&j) && !matches!(block.kind, super::blocks::BlockKind::Heading { .. }) {
            diff.added.push(label(block));
        }
    }
    diff
}

// ---- report -------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default, Serialize, PartialEq, Eq)]
pub struct Counts {
    pub unchanged: usize,
    pub modified: usize,
    pub moved: usize,
    pub added: usize,
    pub removed: usize,
}

pub fn count(matches: &[SectionMatch]) -> Counts {
    let mut counts = Counts::default();
    for m in matches {
        match m.change {
            Change::Unchanged => counts.unchanged += 1,
            Change::Modified => counts.modified += 1,
            Change::Moved => counts.moved += 1,
            Change::Added => counts.added += 1,
            Change::Removed => counts.removed += 1,
        }
    }
    counts
}

/// The comparison report (without the AI narrative).
pub fn build_report(a: &[SectionData], b: &[SectionData]) -> Value {
    let matches = align(a, b);
    let counts = count(&matches);
    let topic = |m: &SectionMatch| -> String {
        match (m.a, m.b) {
            (Some(i), _) => a[i].heading.clone(),
            (None, Some(j)) => b[j].heading.clone(),
            _ => String::new(),
        }
    };
    let coverage: Vec<Value> = matches
        .iter()
        .map(|m| {
            json!({
                "topic": topic(m),
                "topic_b": m.b.map(|j| b[j].heading.clone()),
                "lecture_a": m.a.map(|i| a[i].lecture_title.clone()),
                "lecture_b": m.b.map(|j| b[j].lecture_title.clone()),
                "in_a": m.a.is_some(),
                "in_b": m.b.is_some(),
                "change": m.change,
                "similarity": (m.similarity * 100.0).round() / 100.0,
            })
        })
        .collect();
    let changes: Vec<Value> = matches
        .iter()
        .filter(|m| matches!(m.change, Change::Modified | Change::Moved))
        .filter_map(|m| {
            let (i, j) = (m.a?, m.b?);
            Some(json!({
                "topic": a[i].heading,
                "topic_b": b[j].heading,
                "change": m.change,
                "similarity": (m.similarity * 100.0).round() / 100.0,
                "diff": diff_blocks(&a[i].blocks, &b[j].blocks),
            }))
        })
        .collect();
    let drift: Vec<String> = matches
        .iter()
        .filter(|m| m.change == Change::Removed)
        .map(topic)
        .collect();
    let new_topics: Vec<String> = matches
        .iter()
        .filter(|m| m.change == Change::Added)
        .map(topic)
        .collect();
    json!({
        "method": if a.iter().chain(b).all(|s| s.vector.is_some()) { "embeddings" } else { "lexical" },
        "sections_a": a.len(),
        "sections_b": b.len(),
        "summary": counts,
        "coverage": coverage,
        "changes": changes,
        "drift": drift,
        "new_topics": new_topics,
    })
}

// ---- data loading -------------------------------------------------------------------

fn parse_vector(text: &str) -> Option<Vec<f32>> {
    let inner = text.trim().trim_start_matches('[').trim_end_matches(']');
    if inner.is_empty() {
        return None;
    }
    inner.split(',').map(|v| v.trim().parse::<f32>().ok()).collect()
}

/// All sections of a course's published materials, in lecture order.
pub async fn load_course_sections(pool: &PgPool, course_id: Uuid) -> Result<Vec<SectionData>, String> {
    let rows = sqlx::query_as::<_, (Uuid, String, Value)>(
        "select v.id, s.title, v.doc
         from lecture_materials m
         join lecture_sessions s on s.id = m.session_id
         join material_versions v on v.id = m.published_version_id
         where m.course_id = $1 and m.status = 'published'
         order by s.started_at nulls last, s.title",
    )
    .bind(course_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    let version_ids: Vec<Uuid> = rows.iter().map(|r| r.0).collect();
    let vectors = sqlx::query_as::<_, (Uuid, String, String)>(
        "select version_id, block_id, embedding::text from material_section_embeddings
         where version_id = any($1)",
    )
    .bind(&version_ids)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for (version_id, lecture_title, doc) in rows {
        let doc = store::parse_doc(&doc)?;
        for section in doc.sections() {
            if section.heading.is_empty() {
                continue;
            }
            let vector = vectors
                .iter()
                .find(|(v, block, _)| *v == version_id && block == section.block_id)
                .and_then(|(_, _, text)| parse_vector(text));
            out.push(SectionData {
                lecture_title: lecture_title.clone(),
                version_id,
                block_id: section.block_id.to_owned(),
                heading: section.heading.clone(),
                body: section.body_text(),
                blocks: section.blocks.iter().map(|b| (*b).clone()).collect(),
                vector,
            });
        }
    }
    Ok(out)
}

// ---- embedding job ------------------------------------------------------------------

/// Embeds every section of a published version (job `material_embed`).
pub async fn run_embed_job(
    state: &AppState,
    pool: &PgPool,
    job: &PendingJob,
    version_id: Option<Uuid>,
) -> Result<Option<Uuid>, String> {
    let version_id = version_id.ok_or_else(|| "no version to embed".to_owned())?;
    let Some((model, dimensions)) = state.ai.embedding_model() else {
        return Ok(None);
    };
    if dimensions != embeddings::DIMENSIONS {
        return Ok(None);
    }
    let version = store::find_version(pool, version_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "the version no longer exists".to_owned())?;
    let doc: MaterialDoc = store::parse_doc(&version.doc)?;
    let sections: Vec<_> = doc.sections().into_iter().filter(|s| !s.heading.is_empty()).collect();
    if sections.is_empty() {
        return Ok(None);
    }
    let items: Vec<EmbedItem> = sections
        .iter()
        .map(|s| EmbedItem {
            text: format!("{}\n{}", s.heading, s.body_text()).chars().take(6_000).collect(),
            title: Some(doc.title.clone()),
        })
        .collect();
    let embedded = state
        .ai
        .embed(&items, EmbedKind::Document, Some(job.session_id))
        .await
        .map_err(|e| e.to_string())?;
    if embedded.vectors.len() != sections.len() {
        return Err("the embedding service returned the wrong number of vectors".to_owned());
    }
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    sqlx::query("delete from material_section_embeddings where version_id = $1 and model = $2")
        .bind(version_id)
        .bind(&embedded.model)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    for (section, vector) in sections.iter().zip(&embedded.vectors) {
        sqlx::query(
            "insert into material_section_embeddings (version_id, block_id, heading, body, embedding, model)
             values ($1, $2, $3, $4, $5::vector, $6)",
        )
        .bind(version_id)
        .bind(section.block_id)
        .bind(&section.heading)
        .bind(section.body_text().chars().take(6_000).collect::<String>())
        .bind(embeddings::vector_literal(vector))
        .bind(&embedded.model)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    }
    tx.commit().await.map_err(|e| e.to_string())?;
    let _ = model;
    Ok(None)
}

// ---- analyzer plug-in ---------------------------------------------------------------

pub struct ComparisonAnalyzer;

#[async_trait]
impl Analyzer for ComparisonAnalyzer {
    fn id(&self) -> &'static str {
        "comparison"
    }
    fn label(&self) -> &'static str {
        "Offering comparison"
    }

    /// params: `{"course_a": uuid, "course_b": uuid, "narrative": bool}`.
    async fn analyze(&self, state: &AppState, input: AnalyzerInput) -> Result<Value, PluginError> {
        let pool = state.db_pool();
        let id_of = |key: &str| -> Result<Uuid, PluginError> {
            input
                .params
                .get(key)
                .and_then(Value::as_str)
                .and_then(|s| Uuid::parse_str(s).ok())
                .ok_or_else(|| PluginError::failed(format!("{key} must be a course id")))
        };
        let (course_a, course_b) = (id_of("course_a")?, id_of("course_b")?);
        if course_a == course_b {
            return Err(PluginError::failed("choose two different offerings"));
        }
        let mut infos = Vec::new();
        for id in [course_a, course_b] {
            let info = store::course_info(pool, id)
                .await
                .map_err(|e| PluginError::failed(e.to_string()))?
                .ok_or_else(|| PluginError::failed("course not found"))?;
            if info.lecturer_id != input.requested_by {
                return Err(PluginError::failed("you can only compare your own course offerings"));
            }
            infos.push(info);
        }
        if infos[0].code.trim().to_lowercase() != infos[1].code.trim().to_lowercase() {
            return Err(PluginError::failed("these are different courses (the course codes differ)"));
        }
        let a = load_course_sections(pool, course_a).await.map_err(PluginError::Failed)?;
        let b = load_course_sections(pool, course_b).await.map_err(PluginError::Failed)?;
        if a.is_empty() || b.is_empty() {
            return Err(PluginError::failed("both offerings need published materials to compare"));
        }
        let mut report = build_report(&a, &b);
        report["course_a"] = serde_json::to_value(&infos[0]).unwrap_or_default();
        report["course_b"] = serde_json::to_value(&infos[1]).unwrap_or_default();
        if input.params.get("narrative").and_then(Value::as_bool).unwrap_or(false) {
            report["narrative"] = match narrative(state, &report).await {
                Some(text) => Value::String(text),
                None => Value::Null,
            };
        }
        let id: Uuid = sqlx::query_scalar(
            "insert into material_comparisons (course_a, course_b, report, created_by)
             values ($1, $2, $3, $4) returning id",
        )
        .bind(course_a)
        .bind(course_b)
        .bind(&report)
        .bind(input.requested_by)
        .fetch_one(pool)
        .await
        .map_err(|e| PluginError::failed(e.to_string()))?;
        report["id"] = json!(id);
        Ok(report)
    }

    /// Publishing a material queues its section embeddings.
    async fn on_event(&self, state: &AppState, event: &MaterialEvent) -> Result<(), PluginError> {
        if event.event_type != "material.published" || !state.ai.supports_embeddings() {
            return Ok(());
        }
        let get = |key: &str| event.payload.get(key).and_then(Value::as_str).and_then(|s| Uuid::parse_str(s).ok());
        let (Some(session), Some(lecturer), Some(version), Some(material)) =
            (get("session_id"), get("lecturer_id"), get("version_id"), get("material_id"))
        else {
            return Ok(());
        };
        store::enqueue_job(
            state.db_pool(),
            session,
            lecturer,
            "material_embed",
            json!({"material_id": material, "version_id": version, "nonce": version}),
        )
        .await
        .map_err(|e| PluginError::failed(e.to_string()))?;
        Ok(())
    }
}

/// A few sentences on the main shifts, from the report's facts only.
async fn narrative(state: &AppState, report: &Value) -> Option<String> {
    let facts = json!({
        "summary": report["summary"],
        "drift": report["drift"],
        "new_topics": report["new_topics"],
        "modified": report["changes"].as_array().map(|c| c.iter().take(8).map(|x| json!({
            "topic": x["topic"], "added": x["diff"]["added"], "removed": x["diff"]["removed"]
        })).collect::<Vec<_>>()),
    });
    let request = TextRequest::new(
        Task::Material,
        "You summarise how a university course's lecture materials changed between two offerings. \
Write 3-5 plain sentences for the lecturer: what topics were dropped, what was added, and what changed \
in emphasis. Use only the facts given; do not speculate.",
        facts.to_string(),
    )
    .thinking(Thinking::Minimal)
    .max_tokens(600);
    state.ai.generate(&request).await.ok().map(|o| o.text.trim().to_owned()).filter(|t| !t.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::blocks::BlockKind;

    fn section(heading: &str, body: &str) -> SectionData {
        let blocks = vec![
            Block::new("h", BlockKind::Heading { level: 1, text: heading.into() }),
            Block::new("p", BlockKind::Paragraph { text: body.into() }),
        ];
        SectionData {
            lecture_title: "L".into(),
            version_id: Uuid::nil(),
            block_id: "h".into(),
            heading: heading.into(),
            body: body.into(),
            blocks,
            vector: None,
        }
    }

    const COND: &str = "Conduction transfers heat through solids following Fourier law with thermal conductivity";
    const CONV: &str = "Convection transfers heat through moving fluids using Newton cooling law and heat transfer coefficient";
    const RAD: &str = "Radiation transfers heat as electromagnetic waves following Stefan Boltzmann law emissivity";
    const FIN: &str = "Fins extend surface area to increase heat dissipation fin efficiency effectiveness";

    #[test]
    fn jaccard_and_cosine_behave() {
        assert!(token_jaccard(COND, COND) > 0.99);
        assert!(token_jaccard(COND, CONV) < token_jaccard(COND, COND));
        assert!(token_jaccard("alpha beta gamma", "delta epsilon zeta") < 0.01);
        assert!((cosine(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-9);
        assert!(cosine(&[1.0, 0.0], &[0.0, 1.0]) < 1e-9);
        assert_eq!(cosine(&[1.0], &[1.0, 2.0]), 0.0);
    }

    #[test]
    fn alignment_classifies_every_change() {
        let a = vec![
            section("Conduction", COND),
            section("Convection", CONV),
            section("Radiation", RAD),
        ];
        let b = vec![
            section("Conduction", COND),
            section("Convection", &format!("{CONV} forced and natural convection correlations Nusselt number")),
            section("Fins", FIN),
        ];
        let matches = align(&a, &b);
        let find = |i: Option<usize>, j: Option<usize>| matches.iter().find(|m| m.a == i && m.b == j);
        assert_eq!(find(Some(0), Some(0)).unwrap().change, Change::Unchanged);
        assert_eq!(find(Some(1), Some(1)).unwrap().change, Change::Modified);
        assert_eq!(find(Some(2), None).unwrap().change, Change::Removed);
        assert_eq!(find(None, Some(2)).unwrap().change, Change::Added);
        let counts = count(&matches);
        assert_eq!(counts, Counts { unchanged: 1, modified: 1, moved: 0, added: 1, removed: 1 });
    }

    #[test]
    fn reordered_sections_are_moved_not_lost() {
        let a = vec![section("Conduction", COND), section("Radiation", RAD)];
        let b = vec![section("Radiation", RAD), section("Conduction", COND)];
        let matches = align(&a, &b);
        let counts = count(&matches);
        assert_eq!(counts.added + counts.removed, 0);
        assert_eq!(counts.unchanged + counts.moved, 2);
        assert!(counts.moved >= 1);
    }

    #[test]
    fn vectors_take_precedence_when_both_sides_have_them() {
        let mut a = vec![section("Alpha", "aaa bbb ccc")];
        let mut b = vec![section("Totally different words", "xxx yyy zzz")];
        a[0].vector = Some(vec![1.0, 0.0, 0.0]);
        b[0].vector = Some(vec![0.99, 0.05, 0.0]);
        let matches = align(&a, &b);
        assert_eq!(matches.len(), 1);
        assert!(matches[0].a.is_some() && matches[0].b.is_some());
        b[0].vector = Some(vec![0.0, 1.0, 0.0]);
        let matches = align(&a, &b);
        assert_eq!(count(&matches), Counts { unchanged: 0, modified: 0, moved: 0, added: 1, removed: 1 });
    }

    #[test]
    fn block_diff_separates_added_removed_and_changed() {
        let a = vec![
            Block::new("1", BlockKind::Definition { term: "Flux".into(), text: "heat per unit area per second".into() }),
            Block::new("2", BlockKind::Paragraph { text: "Heat flows from hot to cold along the gradient always".into() }),
            Block::new("3", BlockKind::Equation { latex: "q=-k dT/dx".into(), caption: None, number: None }),
        ];
        let b = vec![
            Block::new("1", BlockKind::Definition { term: "Flux".into(), text: "heat per unit area per second".into() }),
            Block::new("2", BlockKind::Paragraph { text: "Heat flows from hot to cold along the temperature gradient in solids".into() }),
            Block::new("9", BlockKind::WorkedExample { title: "Wall".into(), problem: "p".into(), steps: vec!["s".into()], answer: None }),
        ];
        let diff = diff_blocks(&a, &b);
        assert_eq!(diff.unchanged, 1);
        assert_eq!(diff.changed.len(), 1);
        assert_eq!(diff.changed[0].block_type, "paragraph");
        assert_eq!(diff.removed.len(), 1);
        assert!(diff.removed[0].starts_with("equation"));
        assert_eq!(diff.added.len(), 1);
        assert!(diff.added[0].starts_with("worked_example"));
    }

    #[test]
    fn report_has_coverage_changes_and_drift() {
        let a = vec![section("Conduction", COND), section("Radiation", RAD)];
        let b = vec![section("Conduction", COND), section("Fins", FIN)];
        let report = build_report(&a, &b);
        assert_eq!(report["method"], "lexical");
        assert_eq!(report["summary"]["removed"], 1);
        assert_eq!(report["drift"][0], "Radiation");
        assert_eq!(report["new_topics"][0], "Fins");
        assert_eq!(report["coverage"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn vectors_parse_from_pgvector_text() {
        assert_eq!(parse_vector("[0.5,1,-2]"), Some(vec![0.5, 1.0, -2.0]));
        assert_eq!(parse_vector("[]"), None);
        assert_eq!(parse_vector("[a,b]"), None);
    }
}
