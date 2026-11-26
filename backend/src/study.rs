//! Shared building blocks for the study pipeline: transcript loading, AI
//! prompts, tolerant parsing of model output, and deterministic fallbacks used
//! when no model is reachable (so chapters, summaries and concept maps never
//! silently disappear).

use std::collections::{HashMap, HashSet};

use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Transcript {
    /// One caption per line, prefixed with `[mm:ss]` when offsets are known.
    pub text: String,
    pub duration_sec: i32,
}

#[derive(Debug, Clone)]
pub struct Chapter {
    pub title: String,
    pub summary: String,
    pub start_sec: i32,
    pub end_sec: i32,
}

#[derive(Debug, Clone)]
pub struct Card {
    pub prompt: String,
    pub answer: String,
    pub topic_tag: Option<String>,
    pub difficulty: String,
}

/// `[mm:ss]` at the start of a line, as text (`"01:15"`).
pub fn leading_stamp(line: &str) -> Option<String> {
    let line = line.trim_start();
    let inner = line.strip_prefix('[')?.split(']').next()?;
    inner.split_once(':')?;
    Some(inner.to_owned())
}

/// Seconds of the last `[mm:ss]` marker in a block of text.
pub fn last_offset(text: &str) -> Option<i32> {
    text.lines().rev().find_map(|line| {
        let stamp = leading_stamp(line)?;
        let (m, s) = stamp.split_once(':')?;
        Some(m.trim().parse::<i32>().ok()? * 60 + s.trim().parse::<i32>().ok()?)
    })
}

pub fn format_offset(seconds: i64) -> String {
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

#[derive(sqlx::FromRow)]
struct CaptionRow {
    text: String,
    offset_sec: i64,
}

/// Loads a session's visible captions with `[mm:ss]` offsets from the start of
/// the lecture.
pub async fn load_transcript(pool: &PgPool, session_id: Uuid) -> Result<Transcript, sqlx::Error> {
    let rows = sqlx::query_as::<_, CaptionRow>(
        "select c.text,
                greatest(0, extract(epoch from (c.created_at - coalesce(s.started_at, c.created_at)))::bigint) as offset_sec
         from caption_chunks c
         join lecture_sessions s on s.id = c.session_id
         where c.session_id = $1 and not c.is_hidden
         order by c.sequence_number",
    )
    .bind(session_id)
    .fetch_all(pool)
    .await?;
    let duration_sec = rows.last().map(|row| row.offset_sec).unwrap_or(0) as i32;
    let text = rows
        .iter()
        .map(|row| format!("[{}] {}", format_offset(row.offset_sec), row.text.trim()))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(Transcript { text, duration_sec })
}

/// Transcript-resource `content` is JSON (`{"text": "..."}` from the
/// transcribe job, or a bare string / array when uploaded by hand).
pub fn text_from_resource_json(content: &str) -> String {
    match serde_json::from_str::<Value>(content) {
        Ok(Value::String(text)) => text,
        Ok(Value::Object(map)) => map
            .get("text")
            .and_then(|value| value.as_str())
            .map(str::to_owned)
            .unwrap_or_default(),
        Ok(Value::Array(items)) => items
            .iter()
            .filter_map(|item| match item {
                Value::String(text) => Some(text.clone()),
                other => other
                    .get("text")
                    .and_then(|value| value.as_str())
                    .map(str::to_owned),
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => content.to_owned(),
    }
}

/// Keeps the head and tail of very long transcripts.
pub fn clamp_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }
    let half = max_chars / 2;
    let head: String = text.chars().take(half).collect();
    let tail: String = text
        .chars()
        .rev()
        .take(half)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{head}\n[... middle of the lecture omitted for length ...]\n{tail}")
}

pub fn system_prompt(job_type: &str, params: &Value) -> String {
    let base = "You are KLASYNC, an accessibility-first learning assistant for university students. The user message is a lecture transcript (lines may start with [mm:ss]).";
    let topic = params
        .get("topic")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    match job_type {
        "flashcards" => format!(
            "{base} Create 8 to 14 high quality study flashcards from the lecture{}. \
             Return ONLY a valid JSON array of objects with keys: \"prompt\" (a clear question), \
             \"answer\" (a concise, correct answer), \"topic_tag\" (short topic label), \
             \"difficulty\" (\"easy\", \"medium\" or \"hard\"). No markdown fences, no commentary.",
            topic
                .map(|t| format!(", focusing specifically on: {t}"))
                .unwrap_or_default()
        ),
        "chapters" => format!(
            "{base} Split the lecture into 4 to 8 chronological chapters. Return ONLY a valid JSON \
             array of objects with keys: \"chapter_index\" (integer from 1), \"title\" (short), \
             \"summary\" (1-2 sentences), \"start_timestamp_sec\" (integer seconds), \
             \"end_timestamp_sec\" (integer seconds). Use the [mm:ss] markers to compute seconds. \
             No markdown fences, no commentary."
        ),
        "visualization" => format!(
            "{base} Build a concept map of the lecture{}. Return ONLY a valid JSON object: \
             {{\"title\": string, \"nodes\": [{{\"id\": string, \"label\": string (max 5 words), \
             \"group\": string (topic cluster), \"importance\": integer 1-5}}], \
             \"edges\": [{{\"from\": node id, \"to\": node id, \"label\": string (relationship, max 4 words)}}]}}. \
             Use 8 to 24 nodes, connect every node, make the central idea importance 5. \
             No markdown fences, no commentary.",
            topic
                .map(|t| format!(" centred on: {t}"))
                .unwrap_or_default()
        ),
        "summarize" => format!(
            "{base} Write a clear study summary in markdown: a short overview paragraph followed by bullet points of the key ideas, definitions and formulas."
        ),
        "explain" => format!("{base} Explain the main ideas in plain, simple language for a student who missed the lecture."),
        "lecture_qa_index" | "question_answer" => format!("{base} Answer using only the lecture content. Be concise."),
        _ => base.to_owned(),
    }
}

fn str_field<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(|v| v.as_str()))
        .map(str::trim)
        .filter(|text| !text.is_empty())
}

fn int_field(value: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter().find_map(|key| {
        let v = value.get(*key)?;
        v.as_i64()
            .or_else(|| v.as_f64().map(|f| f as i64))
            .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
    })
}

fn list_of<'a>(value: &'a Value, wrapper_keys: &[&str]) -> Option<&'a Vec<Value>> {
    value.as_array().or_else(|| {
        wrapper_keys
            .iter()
            .find_map(|key| value.get(*key).and_then(|v| v.as_array()))
    })
}

pub fn parse_chapters(value: &Value) -> Option<Vec<Chapter>> {
    let items = list_of(value, &["chapters", "items"])?;
    let mut chapters: Vec<Chapter> = items
        .iter()
        .filter_map(|item| {
            let title = str_field(item, &["title", "name"])?.to_owned();
            let summary = str_field(item, &["summary", "description"])
                .unwrap_or("")
                .to_owned();
            let start =
                int_field(item, &["start_timestamp_sec", "start_sec", "start"]).unwrap_or(0);
            let end = int_field(item, &["end_timestamp_sec", "end_sec", "end"]).unwrap_or(start);
            Some(Chapter {
                title,
                summary,
                start_sec: start.max(0) as i32,
                end_sec: end.max(start).max(0) as i32,
            })
        })
        .collect();
    chapters.sort_by_key(|chapter| chapter.start_sec);
    // Models often leave end == 0; chain each chapter to the next one.
    for index in 0..chapters.len() {
        let next_start = chapters.get(index + 1).map(|next| next.start_sec);
        if let Some(next_start) = next_start {
            if chapters[index].end_sec <= chapters[index].start_sec {
                chapters[index].end_sec = next_start;
            }
        }
    }
    Some(chapters)
}

pub fn parse_flashcards(value: &Value) -> Vec<Card> {
    let Some(items) = list_of(value, &["flashcards", "cards", "items"]) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let prompt = str_field(item, &["prompt", "question", "front", "q"])?.to_owned();
            let answer = str_field(item, &["answer", "back", "a"])?.to_owned();
            let difficulty = match str_field(item, &["difficulty"])
                .map(|d| d.to_ascii_lowercase())
                .as_deref()
            {
                Some("easy") => "easy",
                Some("hard") => "hard",
                _ => "medium",
            };
            Some(Card {
                prompt,
                answer,
                topic_tag: str_field(item, &["topic_tag", "topic", "tag"]).map(str::to_owned),
                difficulty: difficulty.to_owned(),
            })
        })
        .collect()
}

/// Validates and cleans a model-produced concept map.
pub fn normalise_concept_map(value: &Value) -> Option<Value> {
    let object = value.as_object()?;
    let nodes_raw = object.get("nodes")?.as_array()?;
    let mut seen = HashSet::new();
    let mut nodes = Vec::new();
    for node in nodes_raw.iter().take(40) {
        let label = str_field(node, &["label", "name", "title"])?;
        let id = node
            .get("id")
            .map(|v| match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .unwrap_or_else(|| label.to_owned());
        if !seen.insert(id.clone()) {
            continue;
        }
        nodes.push(json!({
            "id": id,
            "label": label.chars().take(60).collect::<String>(),
            "group": str_field(node, &["group", "cluster"]).unwrap_or("Concepts"),
            "importance": int_field(node, &["importance", "weight"]).unwrap_or(2).clamp(1, 5),
        }));
    }
    if nodes.is_empty() {
        return None;
    }
    let ids: HashSet<String> = nodes
        .iter()
        .filter_map(|n| n.get("id").and_then(|v| v.as_str()).map(str::to_owned))
        .collect();
    let edges: Vec<Value> = object
        .get("edges")
        .and_then(|e| e.as_array())
        .map(|edges| {
            edges
                .iter()
                .filter_map(|edge| {
                    let from = edge.get("from").or_else(|| edge.get("source")).map(|v| {
                        v.as_str()
                            .map(str::to_owned)
                            .unwrap_or_else(|| v.to_string())
                    })?;
                    let to = edge.get("to").or_else(|| edge.get("target")).map(|v| {
                        v.as_str()
                            .map(str::to_owned)
                            .unwrap_or_else(|| v.to_string())
                    })?;
                    if from == to || !ids.contains(&from) || !ids.contains(&to) {
                        return None;
                    }
                    Some(json!({
                        "from": from,
                        "to": to,
                        "label": str_field(edge, &["label", "relation"]).unwrap_or(""),
                    }))
                })
                .collect()
        })
        .unwrap_or_default();
    Some(json!({
        "title": object.get("title").and_then(|t| t.as_str()).unwrap_or("Lecture concept map"),
        "nodes": nodes,
        "edges": edges,
    }))
}

const STOPWORDS: &[&str] = &[
    "about", "above", "after", "again", "also", "among", "because", "been", "before", "being",
    "between", "both", "could", "does", "doing", "down", "during", "each", "from", "have",
    "having", "here", "into", "just", "like", "make", "many", "more", "most", "much", "must",
    "only", "other", "over", "same", "should", "some", "such", "than", "that", "their", "them",
    "then", "there", "these", "they", "thing", "things", "this", "those", "through", "very",
    "want", "were", "what", "when", "where", "which", "while", "will", "with", "would", "your",
    "okay", "going", "gonna", "know", "think", "right", "yeah", "lecture", "today", "class",
    "students", "student", "let", "well", "really", "actually", "can", "now", "and", "the",
];

fn strip_offset(line: &str) -> &str {
    let line = line.trim();
    if line.starts_with('[') {
        if let Some(end) = line.find(']') {
            return line[end + 1..].trim();
        }
    }
    line
}

fn tokens(line: &str) -> Vec<String> {
    line.split(|c: char| !c.is_alphanumeric() && c != '\'' && c != '-')
        .map(|word| word.trim_matches(|c| c == '\'' || c == '-').to_lowercase())
        .filter(|word| word.chars().count() >= 4 && !STOPWORDS.contains(&word.as_str()))
        .collect()
}

/// Frequency / co-occurrence concept map used when no AI model is available.
pub fn heuristic_concept_map(text: &str) -> Value {
    let lines: Vec<&str> = text
        .lines()
        .map(strip_offset)
        .filter(|line| !line.is_empty())
        .collect();
    let mut frequency: HashMap<String, usize> = HashMap::new();
    let line_tokens: Vec<Vec<String>> = lines.iter().map(|line| tokens(line)).collect();
    for token_list in &line_tokens {
        for token in token_list {
            *frequency.entry(token.clone()).or_insert(0) += 1;
        }
    }
    let mut ranked: Vec<(String, usize)> = frequency.into_iter().filter(|(_, n)| *n >= 2).collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    ranked.truncate(14);
    if ranked.len() < 3 {
        return json!({"title": "Lecture concept map", "nodes": [], "edges": []});
    }
    let top: HashSet<&str> = ranked.iter().map(|(word, _)| word.as_str()).collect();
    let max = ranked[0].1 as f64;
    let nodes: Vec<Value> = ranked
        .iter()
        .map(|(word, count)| {
            let importance = (1.0 + 4.0 * (*count as f64 / max)).round() as i64;
            let mut label = word.clone();
            if let Some(first) = label.get(0..1) {
                label = format!("{}{}", first.to_uppercase(), &label[1..]);
            }
            json!({"id": word, "label": label, "group": "Key terms", "importance": importance.clamp(1, 5)})
        })
        .collect();
    let mut pair_counts: HashMap<(String, String), usize> = HashMap::new();
    for token_list in &line_tokens {
        let mut present: Vec<&String> = token_list
            .iter()
            .filter(|token| top.contains(token.as_str()))
            .collect();
        present.sort();
        present.dedup();
        for i in 0..present.len() {
            for j in i + 1..present.len() {
                *pair_counts
                    .entry((present[i].clone(), present[j].clone()))
                    .or_insert(0) += 1;
            }
        }
    }
    let mut pairs: Vec<((String, String), usize)> = pair_counts.into_iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut edges: Vec<Value> = pairs
        .iter()
        .take(24)
        .map(|((a, b), _)| json!({"from": a, "to": b, "label": "related"}))
        .collect();
    // Make sure nothing floats free: attach orphans to the central concept.
    let linked: HashSet<String> = pairs
        .iter()
        .take(24)
        .flat_map(|((a, b), _)| [a.clone(), b.clone()])
        .collect();
    let centre = ranked[0].0.clone();
    for (word, _) in &ranked {
        if !linked.contains(word) && *word != centre {
            edges.push(json!({"from": centre, "to": word, "label": "related"}));
        }
    }
    json!({"title": "Lecture concept map", "nodes": nodes, "edges": edges})
}

fn offset_of(line: &str) -> Option<i32> {
    let line = line.trim();
    let inner = line.strip_prefix('[')?.split(']').next()?;
    let (minutes, seconds) = inner.split_once(':')?;
    Some(minutes.trim().parse::<i32>().ok()? * 60 + seconds.trim().parse::<i32>().ok()?)
}

fn first_sentence(text: &str, max_chars: usize) -> String {
    let end = text
        .find(['.', '?', '!'])
        .map(|i| i + 1)
        .unwrap_or(text.len());
    let sentence: String = text[..end].chars().take(max_chars).collect();
    sentence.trim().to_owned()
}

/// Splits the lecture into equal slices when the model cannot be reached.
pub fn heuristic_chapters(text: &str, duration_sec: i32) -> Vec<Chapter> {
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() {
        return Vec::new();
    }
    let slices = lines.len().div_ceil(8).clamp(1, 6);
    let per = lines.len().div_ceil(slices);
    let mut chapters = Vec::new();
    for (index, chunk) in lines.chunks(per).enumerate() {
        let joined = chunk
            .iter()
            .map(|line| strip_offset(line))
            .collect::<Vec<_>>()
            .join(" ");
        let start = chunk.first().and_then(|l| offset_of(l)).unwrap_or_else(|| {
            if duration_sec > 0 {
                (duration_sec as usize * index / slices) as i32
            } else {
                0
            }
        });
        let end = chunk.last().and_then(|l| offset_of(l)).unwrap_or(start);
        let title = first_sentence(&joined, 60);
        chapters.push(Chapter {
            title: if title.is_empty() {
                format!("Part {}", index + 1)
            } else {
                title
            },
            summary: first_sentence(&joined, 240),
            start_sec: start,
            end_sec: end.max(start),
        });
    }
    chapters
}

/// Extractive summary (top-scoring sentences) used when a model is unavailable.
pub fn heuristic_summary(text: &str) -> (String, Vec<String>) {
    let cleaned: Vec<String> = text
        .lines()
        .map(strip_offset)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    let joined = cleaned.join(" ");
    let sentences: Vec<String> = joined
        .split_inclusive(['.', '?', '!'])
        .map(|s| s.trim().to_owned())
        .filter(|s| s.split_whitespace().count() >= 4)
        .collect();
    if sentences.is_empty() {
        return (joined.chars().take(400).collect(), Vec::new());
    }
    let mut frequency: HashMap<String, usize> = HashMap::new();
    for sentence in &sentences {
        for token in tokens(sentence) {
            *frequency.entry(token).or_insert(0) += 1;
        }
    }
    let mut scored: Vec<(usize, f64)> = sentences
        .iter()
        .enumerate()
        .map(|(index, sentence)| {
            let toks = tokens(sentence);
            let score: usize = toks
                .iter()
                .map(|t| frequency.get(t).copied().unwrap_or(0))
                .sum();
            (index, score as f64 / (toks.len().max(1) as f64).sqrt())
        })
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut picked: Vec<usize> = scored.iter().take(4).map(|(i, _)| *i).collect();
    picked.sort();
    let points: Vec<String> = picked
        .iter()
        .map(|i| sentences[*i].chars().take(220).collect())
        .collect();
    (points.join(" "), points)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chapters_chain_end_times() {
        let raw = json!([
            {"chapter_index":1,"title":"A","summary":"s","start_timestamp_sec":0,"end_timestamp_sec":0},
            {"chapter_index":2,"title":"B","summary":"s","start_timestamp_sec":120}
        ]);
        let chapters = parse_chapters(&raw).unwrap();
        assert_eq!(chapters[0].end_sec, 120);
    }

    #[test]
    fn flashcards_accept_alternate_keys() {
        let raw = json!({"flashcards":[{"question":"Q?","answer":"A","difficulty":"HARD"}]});
        let cards = parse_flashcards(&raw);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].difficulty, "hard");
    }

    #[test]
    fn concept_map_drops_dangling_edges() {
        let raw = json!({"nodes":[{"id":"a","label":"A"},{"id":"b","label":"B"}],
                         "edges":[{"from":"a","to":"b"},{"from":"a","to":"zzz"}]});
        let map = normalise_concept_map(&raw).unwrap();
        assert_eq!(map["edges"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn heuristic_map_builds_from_repeated_terms() {
        let text = "[00:01] Photosynthesis converts light energy into chemical energy.\n\
                    [00:10] Chlorophyll absorbs light energy during photosynthesis.\n\
                    [00:20] Chemical energy is stored as glucose by photosynthesis.";
        let map = heuristic_concept_map(text);
        assert!(map["nodes"].as_array().unwrap().len() >= 3);
    }

    #[test]
    fn transcript_json_variants() {
        assert_eq!(text_from_resource_json("{\"text\":\"hi\"}"), "hi");
        assert_eq!(text_from_resource_json("\"yo\""), "yo");
        assert_eq!(text_from_resource_json("[\"a\",\"b\"]"), "a\nb");
    }

    #[test]
    fn heuristic_chapters_use_offsets() {
        let text = (0..16)
            .map(|i| format!("[{i:02}:00] Sentence number {i} about topics."))
            .collect::<Vec<_>>()
            .join("\n");
        let chapters = heuristic_chapters(&text, 900);
        assert!(chapters.len() >= 2);
        assert_eq!(chapters[0].start_sec, 0);
    }
}
