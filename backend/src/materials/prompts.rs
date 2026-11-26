//! Pure logic for the generation pipeline: prompts, tolerant parsing of model
//! output, the grounding check, repair of near-valid documents, transcript
//! slicing and merging of lecturer-locked blocks. No I/O, so all of it is unit
//! tested.

use serde_json::Value;

use super::{
    blocks::{
        check_latex, parse_stamp, Block, BlockKind, CalloutVariant, MaterialDoc, Origin,
        SourceRange,
    },
    registry::{Outline, SectionPlan},
    store::MaterialSettings,
};
use crate::study;

/// Bump when prompts or parsing change in a way that should invalidate caches
/// and idempotency keys.
pub const PROMPT_VERSION: &str = "materials-v1";

pub const MIN_SECTIONS: usize = 2;
pub const MAX_SECTIONS: usize = 10;

// ---- prompts ------------------------------------------------------------------

fn style_rules(settings: &MaterialSettings) -> String {
    let depth = match settings.depth.as_str() {
        "concise" => "Be concise: short explanations, only the essential examples.",
        "detailed" => {
            "Be thorough: full explanations, intermediate steps in derivations, several examples."
        }
        _ => "Use a standard level of detail.",
    };
    format!(
        "Audience: {} students. Tone: {}. Write in {}. {depth}",
        settings.audience_level, settings.tone, settings.language
    )
}

pub fn outline_system(settings: &MaterialSettings) -> String {
    format!(
        "You plan the structure of a professional lecture handout for a university lecturer, \
from the transcript of a lecture they just gave. {style}\n\
Return ONLY JSON: {{\"title\": string, \"objectives\": [3-5 learning objectives, each starting with a verb], \
\"sections\": [{{\"title\": string, \"objective\": one sentence on what this section teaches, \
\"start\": \"mm:ss\", \"end\": \"mm:ss\"}}]}}.\n\
Rules: {min}-{max} sections in the order taught; each section's start/end are transcript timestamps \
covering where that topic was discussed; sections must not overlap; use the lecturer's own terminology; \
do not invent topics that were not covered.",
        style = style_rules(settings),
        min = MIN_SECTIONS,
        max = MAX_SECTIONS
    )
}

pub fn section_system(settings: &MaterialSettings) -> String {
    format!(
        "You write one section of a professional lecture handout from a lecture transcript. {style}\n\
Return ONLY JSON: {{\"blocks\": [ ... ]}}. Each block is an object with a \"type\" and these fields:\n\
- paragraph: {{\"text\"}}\n\
- definition: {{\"term\", \"text\"}}\n\
- key_result: {{\"title\"?, \"text\"}}\n\
- worked_example: {{\"title\", \"problem\", \"steps\": [string], \"answer\"?}}\n\
- equation: {{\"latex\" (bare LaTeX, no $ signs), \"caption\"?}}\n\
- table: {{\"caption\"?, \"header\": [string], \"rows\": [[string]]}}\n\
- callout: {{\"variant\": \"note\"|\"tip\"|\"warning\"|\"misconception\", \"text\"}}\n\
Text may use inline Markdown (**bold**, *italic*, `code`). Wrap inline maths in single dollar signs, e.g. $q = k A \\Delta T$. \
Inside JSON strings every LaTeX backslash must be written as a double backslash (\\\\frac, \\\\text, \\\\times).\n\
EVERY block must also carry \"sources\": [{{\"start\": \"mm:ss\", \"end\": \"mm:ss\"}}] copied from the transcript \
timestamps it was written from. If you add something the lecturer did not say (extra context, an additional \
example), set \"origin\": \"supplemented\" and keep it clearly secondary. Never state facts you are unsure of.\n\
Write exposition, not a transcript: explain the ideas in full sentences, define terms when first used, \
and include a worked example or a table where the lecture gave data or a calculation. 4-12 blocks. \
Do not repeat the section title as a block, and do not write a summary (a later step does).",
        style = style_rules(settings)
    )
}

pub fn summary_system(settings: &MaterialSettings) -> String {
    format!(
        "You summarise a lecture handout. {} Return ONLY JSON: {{\"points\": [5-8 short sentences, \
each capturing one idea the student must keep]}}. Use only what the handout says.",
        style_rules(settings)
    )
}

pub fn questions_system(settings: &MaterialSettings) -> String {
    format!(
        "You write review questions for a lecture handout. {} Return ONLY JSON: \
{{\"questions\": [{{\"question\": string, \"answer\": string (a model answer, 1-3 sentences), \
\"difficulty\": \"easy\"|\"medium\"|\"hard\"}}]}}. 5-8 questions mixing recall and application; \
every question must be answerable from the handout.",
        style_rules(settings)
    )
}

// ---- JSON helpers ---------------------------------------------------------------

/// LaTeX commands that begin with a letter JSON also uses for an escape
/// (`\t`, `\f`, `\b`, `\r`, `\n`). Models often write them with a single
/// backslash inside a JSON string, which then parses as a tab, form feed and
/// so on (`\text` becomes TAB + "ext"). Longest names first.
const LATEX_COMMANDS: &[&str] = &[
    "textbf",
    "textit",
    "text",
    "times",
    "theta",
    "tau",
    "tilde",
    "triangle",
    "top",
    "tan",
    "to",
    "frac",
    "forall",
    "flat",
    "beta",
    "begin",
    "bar",
    "boldsymbol",
    "bigl",
    "bigr",
    "rightarrow",
    "right",
    "rangle",
    "rho",
    "nabla",
    "neq",
    "neg",
    "notin",
    "nu",
    "ne",
    "not",
];

/// Doubles a lone backslash that starts one of [`LATEX_COMMANDS`], so the JSON
/// parser yields the LaTeX the model meant. Already-doubled backslashes and
/// real escapes (`\n`, `\"`, `\u00e9`) are left alone.
pub fn fix_json_latex_escapes(raw: &str) -> String {
    let chars: Vec<char> = raw.chars().collect();
    let mut out = String::with_capacity(raw.len() + 16);
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '\\' {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        // Count the run of backslashes; an odd run ends in a real escape start.
        let start = i;
        while i < chars.len() && chars[i] == '\\' {
            i += 1;
        }
        let run = i - start;
        for _ in 0..run {
            out.push('\\');
        }
        if run % 2 == 1 {
            let rest: String = chars[i..].iter().take(12).collect();
            let word_end = |name: &str| {
                rest.starts_with(name)
                    && !rest[name.len()..]
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_alphabetic())
            };
            let lead = rest.chars().next();
            if matches!(lead, Some('t' | 'f' | 'b' | 'r' | 'n'))
                && LATEX_COMMANDS.iter().any(|name| word_end(name))
            {
                // Turn the lone backslash into an escaped backslash.
                out.push('\\');
            }
        }
    }
    out
}

fn str_of<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn str_list(value: &Value, keys: &[&str]) -> Vec<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_array))
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(|s| s.trim().to_owned()))
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn stamp_of(value: &Value) -> Option<i32> {
    match value {
        Value::String(s) => parse_stamp(s),
        Value::Number(n) => n.as_i64().map(|n| n as i32),
        _ => None,
    }
}

// ---- outline ---------------------------------------------------------------------

/// Reads the model's outline. Sections are clamped into the lecture, sorted,
/// de-overlapped and merged down to [`MAX_SECTIONS`].
pub fn parse_outline(value: &Value, duration_sec: i32, fallback_title: &str) -> Option<Outline> {
    let object = value.as_object().map(|_| value)?;
    let raw = object.get("sections")?.as_array()?;
    let duration = duration_sec.max(60);
    let mut sections: Vec<SectionPlan> = Vec::new();
    for item in raw {
        let Some(title) = str_of(item, &["title", "name"]) else {
            continue;
        };
        let start = item
            .get("start")
            .and_then(stamp_of)
            .unwrap_or(0)
            .clamp(0, duration);
        let end = item
            .get("end")
            .and_then(stamp_of)
            .unwrap_or(duration)
            .clamp(start, duration);
        sections.push(SectionPlan {
            index: 0,
            title: title.chars().take(120).collect(),
            objective: str_of(item, &["objective", "summary"])
                .unwrap_or("")
                .to_owned(),
            start_sec: start,
            end_sec: end,
        });
    }
    if sections.is_empty() {
        return None;
    }
    sections.sort_by_key(|s| (s.start_sec, s.end_sec));
    // De-overlap: a section starts no earlier than the previous one ends.
    for i in 1..sections.len() {
        let prev_end = sections[i - 1].end_sec;
        if sections[i].start_sec < prev_end {
            sections[i].start_sec = prev_end.min(sections[i].end_sec);
        }
    }
    while sections.len() > MAX_SECTIONS {
        merge_shortest_pair(&mut sections);
    }
    for (i, section) in sections.iter_mut().enumerate() {
        section.index = i;
    }
    Some(Outline {
        title: str_of(object, &["title"])
            .unwrap_or(fallback_title)
            .to_owned(),
        objectives: str_list(object, &["objectives", "learning_objectives"])
            .into_iter()
            .take(6)
            .collect(),
        sections,
    })
}

fn merge_shortest_pair(sections: &mut Vec<SectionPlan>) {
    let mut best = 0;
    let mut best_len = i32::MAX;
    for i in 0..sections.len() - 1 {
        let len = sections[i + 1].end_sec - sections[i].start_sec;
        if len < best_len {
            best = i;
            best_len = len;
        }
    }
    let next = sections.remove(best + 1);
    let section = &mut sections[best];
    section.title = format!("{} and {}", section.title, next.title);
    section.end_sec = next.end_sec;
    if !next.objective.is_empty() {
        section.objective = format!("{} {}", section.objective, next.objective)
            .trim()
            .to_owned();
    }
}

/// Deterministic outline from the chapters the study pipeline already made, so
/// a lecture still gets a structure when the model is unreachable.
pub fn outline_from_chapters(
    chapters: &[study::Chapter],
    lecture_title: &str,
    duration_sec: i32,
) -> Outline {
    let mut sections: Vec<SectionPlan> = chapters
        .iter()
        .map(|c| SectionPlan {
            index: 0,
            title: c.title.clone(),
            objective: c.summary.clone(),
            start_sec: c.start_sec,
            end_sec: c.end_sec.max(c.start_sec),
        })
        .collect();
    if sections.is_empty() {
        sections.push(SectionPlan {
            index: 0,
            title: lecture_title.to_owned(),
            objective: String::new(),
            start_sec: 0,
            end_sec: duration_sec.max(60),
        });
    }
    while sections.len() > MAX_SECTIONS {
        merge_shortest_pair(&mut sections);
    }
    for (i, section) in sections.iter_mut().enumerate() {
        section.index = i;
    }
    Outline {
        title: lecture_title.to_owned(),
        objectives: Vec::new(),
        sections,
    }
}

// ---- transcript slicing -------------------------------------------------------------

/// The part of a timestamped transcript (`[mm:ss] text` lines) inside
/// `[start, end]` plus `pad` seconds either side. Lines without a timestamp stay
/// with the previous line. A transcript with no timestamps is clamped whole.
pub fn slice_transcript(text: &str, start: i32, end: i32, pad: i32, max_chars: usize) -> String {
    let has_stamps = text.lines().any(|l| study::leading_stamp(l).is_some());
    if !has_stamps {
        return study::clamp_text(text, max_chars);
    }
    let (lo, hi) = (start - pad, end + pad);
    let mut keep = false;
    let mut out = String::new();
    for line in text.lines() {
        if let Some(stamp) = study::leading_stamp(line).and_then(|s| parse_stamp(&s)) {
            keep = stamp >= lo && stamp <= hi;
        }
        if keep {
            out.push_str(line);
            out.push('\n');
        }
    }
    if out.trim().is_empty() {
        return study::clamp_text(text, max_chars);
    }
    study::clamp_text(out.trim_end(), max_chars)
}

// ---- blocks --------------------------------------------------------------------------

fn ranges_of(item: &Value) -> Vec<SourceRange> {
    item.get("sources")
        .or_else(|| item.get("source_ranges"))
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|r| {
                    let start = r
                        .get("start")
                        .or_else(|| r.get("start_sec"))
                        .and_then(stamp_of)?;
                    let end = r
                        .get("end")
                        .or_else(|| r.get("end_sec"))
                        .and_then(stamp_of)?;
                    Some(SourceRange {
                        start_sec: start,
                        end_sec: end.max(start),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Reads `{"blocks": [...]}` (or a bare array). Items that do not fit the
/// schema are dropped and counted; the caller decides whether too many were.
pub fn parse_blocks(value: &Value) -> (Vec<Block>, usize) {
    let items = value
        .get("blocks")
        .and_then(Value::as_array)
        .or_else(|| value.as_array());
    let Some(items) = items else {
        return (Vec::new(), 0);
    };
    let mut blocks = Vec::new();
    let mut dropped = 0;
    for item in items {
        let mut body = item.clone();
        let sources = ranges_of(item);
        let origin = match item.get("origin").and_then(Value::as_str) {
            Some("supplemented") | Some("supplement") | Some("added") => Origin::Supplemented,
            _ => Origin::Lecture,
        };
        if let Some(map) = body.as_object_mut() {
            map.remove("sources");
            map.remove("source_ranges");
            map.remove("origin");
            map.remove("id");
            map.remove("locked");
            // Models often name the heading text `title`.
            if map.get("type").and_then(Value::as_str) == Some("heading")
                && !map.contains_key("text")
            {
                if let Some(title) = map.get("title").cloned() {
                    map.insert("text".into(), title);
                }
            }
        }
        match serde_json::from_value::<BlockKind>(body) {
            Ok(kind) => {
                let mut block = Block::new("", kind);
                block.origin = origin;
                block.source_ranges = sources;
                blocks.push(block);
            }
            Err(_) => dropped += 1,
        }
    }
    (blocks, dropped)
}

/// The grounding check: every block cites transcript spans inside the lecture,
/// or it is marked supplemented and flagged low-confidence. Source ranges are
/// clamped to the lecture. Returns how many blocks were downgraded.
pub fn ground_blocks(blocks: &mut [Block], duration_sec: i32) -> usize {
    let duration = duration_sec.max(1);
    let mut downgraded = 0;
    for block in blocks {
        block.source_ranges = block
            .source_ranges
            .iter()
            .filter(|r| r.start_sec <= duration)
            .map(|r| SourceRange {
                start_sec: r.start_sec.clamp(0, duration),
                end_sec: r.end_sec.clamp(r.start_sec.max(0), duration),
            })
            .collect();
        if block.origin == Origin::Lecture && block.source_ranges.is_empty() {
            block.origin = Origin::Supplemented;
            block.low_confidence = true;
            downgraded += 1;
        }
    }
    downgraded
}

/// Repairs what is cheaply repairable so a near-valid document is not thrown
/// away: pads table rows, demotes unparseable equations to flagged text and
/// removes empty blocks. Returns the number of repairs.
pub fn repair_blocks(blocks: &mut Vec<Block>) -> usize {
    let mut repairs = 0;
    let before = blocks.len();
    blocks.retain(|block| !is_empty_block(&block.kind));
    repairs += before - blocks.len();
    for block in blocks.iter_mut() {
        match &mut block.kind {
            BlockKind::Equation { latex, .. } => {
                *latex = latex
                    .trim()
                    .trim_start_matches("\\[")
                    .trim_end_matches("\\]")
                    .trim_matches('$')
                    .trim()
                    .to_owned();
                if check_latex(latex).is_err() {
                    let text = format!("`{}`", latex.replace('`', "'"));
                    block.kind = BlockKind::Paragraph { text };
                    block.low_confidence = true;
                    repairs += 1;
                }
            }
            BlockKind::Table { header, rows, .. } => {
                let width = header.len();
                for row in rows.iter_mut() {
                    if row.len() != width {
                        row.resize(width, String::new());
                        repairs += 1;
                    }
                }
            }
            BlockKind::Heading { level, .. } => {
                *level = (*level).clamp(1, 4);
            }
            _ => {}
        }
    }
    repairs
}

fn is_empty_block(kind: &BlockKind) -> bool {
    let blank = |s: &str| s.trim().is_empty();
    match kind {
        BlockKind::Paragraph { text }
        | BlockKind::Callout { text, .. }
        | BlockKind::KeyResult { text, .. } => blank(text),
        BlockKind::Definition { term, text } => blank(term) || blank(text),
        BlockKind::WorkedExample { problem, steps, .. } => blank(problem) || steps.is_empty(),
        BlockKind::Equation { latex, .. } => blank(latex),
        BlockKind::Table { header, .. } => header.is_empty(),
        BlockKind::Summary { points } => points.is_empty(),
        BlockKind::ReviewQuestions { questions } => questions.is_empty(),
        BlockKind::FurtherReading { items } => items.is_empty(),
        BlockKind::Heading { text, .. } => blank(text),
        BlockKind::Figure { caption, .. } => blank(caption),
    }
}

/// Gives each block of section `index` (0-based) an id `s<N>-<n>`.
pub fn assign_section_ids(blocks: &mut [Block], index: usize) {
    for (n, block) in blocks.iter_mut().enumerate() {
        block.id = format!("s{}-{}", index + 1, n + 1);
    }
}

pub fn parse_summary(value: &Value) -> Vec<String> {
    str_list(value, &["points", "summary", "key_points"])
}

pub fn parse_questions(value: &Value) -> Vec<super::blocks::ReviewQuestion> {
    value
        .get("questions")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|q| {
                    Some(super::blocks::ReviewQuestion {
                        question: str_of(q, &["question", "prompt"])?.to_owned(),
                        answer: str_of(q, &["answer"]).map(str::to_owned),
                        difficulty: str_of(q, &["difficulty"]).map(str::to_lowercase),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

// ---- assembling -------------------------------------------------------------------------

/// Splices per-section results into one document, in outline order.
pub fn assemble(
    outline: &Outline,
    sections: &[(usize, Vec<Block>)],
    language: &str,
) -> MaterialDoc {
    let mut doc = MaterialDoc::new(outline.title.clone());
    doc.meta.objectives = outline.objectives.clone();
    doc.meta.language = Some(language.to_owned());
    doc.meta.ai_drafted = true;
    for plan in &outline.sections {
        let Some((_, blocks)) = sections.iter().find(|(i, _)| *i == plan.index) else {
            continue;
        };
        let mut heading = Block::new(
            format!("s{}-h", plan.index + 1),
            BlockKind::Heading {
                level: 1,
                text: plan.title.clone(),
            },
        );
        heading.source_ranges = vec![SourceRange {
            start_sec: plan.start_sec,
            end_sec: plan.end_sec,
        }];
        doc.blocks.push(heading);
        doc.blocks.extend(blocks.iter().cloned());
    }
    doc.number_equations(1);
    doc
}

/// Keeps lecturer-locked blocks from a previous draft when a lecture is
/// regenerated: a locked block replaces the new block with the same id, or is
/// re-inserted after the block that preceded it (by id), else at the end.
pub fn merge_locked(new: &mut MaterialDoc, old: &MaterialDoc) -> usize {
    let mut kept = 0;
    for (position, block) in old.blocks.iter().enumerate() {
        if !block.locked {
            continue;
        }
        if let Some(slot) = new.blocks.iter_mut().find(|b| b.id == block.id) {
            *slot = block.clone();
            kept += 1;
            continue;
        }
        let anchor = old.blocks[..position]
            .iter()
            .rev()
            .find_map(|prev| new.blocks.iter().position(|b| b.id == prev.id));
        match anchor {
            Some(index) => new.blocks.insert(index + 1, block.clone()),
            None => new.blocks.push(block.clone()),
        }
        kept += 1;
    }
    new.number_equations(1);
    kept
}

/// Replaces the unlocked blocks of one section (identified by its heading
/// block) with `fresh`, keeping the heading and any locked blocks first.
pub fn replace_section(doc: &mut MaterialDoc, heading_id: &str, fresh: Vec<Block>) -> bool {
    let Some(start) = doc.blocks.iter().position(|b| b.id == heading_id) else {
        return false;
    };
    let end = doc.blocks[start + 1..]
        .iter()
        .position(|b| matches!(&b.kind, BlockKind::Heading { level, .. } if *level <= 2))
        .map(|offset| start + 1 + offset)
        .unwrap_or(doc.blocks.len());
    let locked: Vec<Block> = doc.blocks[start + 1..end]
        .iter()
        .filter(|b| b.locked)
        .cloned()
        .collect();
    let existing: std::collections::HashSet<String> =
        doc.blocks.iter().map(|b| b.id.clone()).collect();
    let mut fresh = fresh;
    // Fresh ids must not collide with blocks that stay in the document.
    for (n, block) in fresh.iter_mut().enumerate() {
        let mut candidate = format!("{heading_id}-r{}", n + 1);
        let mut bump = 0;
        while existing.contains(&candidate)
            && !doc.blocks[start + 1..end].iter().any(|b| b.id == candidate)
        {
            bump += 1;
            candidate = format!("{heading_id}-r{}-{bump}", n + 1);
        }
        block.id = candidate;
    }
    let mut replacement = locked;
    replacement.extend(fresh);
    doc.blocks.splice(start + 1..end, replacement);
    doc.number_equations(1);
    true
}

/// Formats chapters for the outline prompt.
pub fn chapters_prompt(chapters: &[study::Chapter]) -> String {
    chapters
        .iter()
        .map(|c| {
            format!(
                "[{}-{}] {} - {}",
                study::format_offset(c.start_sec as i64),
                study::format_offset(c.end_sec as i64),
                c.title,
                c.summary
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A short callout used when the model is unreachable and a section is built
/// from the transcript alone, so the lecturer sees *something* to edit.
pub fn fallback_section_blocks(plan: &SectionPlan, excerpt: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    if !plan.objective.trim().is_empty() {
        blocks.push(
            Block::new(
                "",
                BlockKind::Paragraph {
                    text: plan.objective.clone(),
                },
            )
            .with_sources(vec![SourceRange {
                start_sec: plan.start_sec,
                end_sec: plan.end_sec,
            }]),
        );
    }
    let cleaned: Vec<String> = excerpt
        .lines()
        .map(|l| {
            let l = l.trim();
            match l.strip_prefix('[').and_then(|r| r.split_once(']')) {
                Some((_, rest)) => rest.trim().to_owned(),
                None => l.to_owned(),
            }
        })
        .filter(|l| !l.is_empty())
        .collect();
    let text = cleaned.join(" ");
    if !text.is_empty() {
        let shown: String = text.chars().take(1_200).collect();
        blocks.push(
            Block::new("", BlockKind::Paragraph { text: shown }).with_sources(vec![SourceRange {
                start_sec: plan.start_sec,
                end_sec: plan.end_sec,
            }]),
        );
    }
    blocks.push(
        Block::new(
            "",
            BlockKind::Callout {
                variant: CalloutVariant::Note,
                text: "This section was assembled from the lecture transcript without AI rewriting. Edit it or regenerate it.".to_owned(),
            },
        )
        .supplemented(),
    );
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn plan(i: usize, start: i32, end: i32) -> SectionPlan {
        SectionPlan {
            index: i,
            title: format!("S{i}"),
            objective: String::new(),
            start_sec: start,
            end_sec: end,
        }
    }

    #[test]
    fn latex_backslashes_survive_json_parsing() {
        // The model wrote single backslashes inside JSON strings.
        let raw = r#"{"a": "\text{m}^2 \times 10 \frac{1}{2} \nu and \\text{ok} \n next \u00e9 tab\there"}"#;
        let broken: Value = serde_json::from_str(raw).unwrap();
        assert!(
            broken["a"].as_str().unwrap().contains('\t'),
            "premise: plain parsing mangles it"
        );
        let fixed: Value = serde_json::from_str(&fix_json_latex_escapes(raw)).unwrap();
        let text = fixed["a"].as_str().unwrap();
        assert!(
            text.contains("\\text{m}^2 \\times 10 \\frac{1}{2} \\nu"),
            "{text}"
        );
        assert!(
            text.contains("\\text{ok}"),
            "already-doubled backslashes are untouched: {text}"
        );
        assert!(text.contains("\n next"), "real newlines stay newlines");
        assert!(text.contains('é'), "unicode escapes stay");
        assert!(text.contains("tab\there"), "a real tab escape stays a tab");
    }

    #[test]
    fn outline_is_clamped_sorted_and_deoverlapped() {
        let value = json!({
            "title": "Lecture 3",
            "objectives": ["Explain X", "Apply Y"],
            "sections": [
                {"title": "Second", "start": "10:00", "end": "20:00"},
                {"title": "First", "start": "00:00", "end": "11:00"},
                {"title": "Past the end", "start": "19:00", "end": "99:00"},
                {"objective": "no title is skipped"}
            ]
        });
        let outline = parse_outline(&value, 1500, "fallback").unwrap();
        assert_eq!(outline.title, "Lecture 3");
        assert_eq!(outline.sections.len(), 3);
        assert_eq!(outline.sections[0].title, "First");
        assert!(outline.sections[1].start_sec >= outline.sections[0].end_sec);
        assert!(outline.sections.iter().all(|s| s.end_sec <= 1500));
        assert_eq!(outline.sections[2].index, 2);
        assert!(parse_outline(&json!({"sections": []}), 600, "t").is_none());
    }

    #[test]
    fn long_outlines_merge_down() {
        let sections: Vec<Value> = (0..14)
            .map(|i| json!({"title": format!("T{i}"), "start": i * 60, "end": i * 60 + 55}))
            .collect();
        let outline = parse_outline(&json!({ "sections": sections }), 900, "x").unwrap();
        assert_eq!(outline.sections.len(), MAX_SECTIONS);
    }

    #[test]
    fn slices_by_timestamp_and_keeps_continuations() {
        let text =
            "[00:00] intro\n[01:00] heat flows\nand continues\n[05:00] later topic\n[09:00] end";
        let slice = slice_transcript(text, 55, 120, 0, 10_000);
        assert!(slice.contains("heat flows"));
        assert!(slice.contains("and continues"));
        assert!(!slice.contains("later topic"));
        let plain = slice_transcript("no stamps here", 0, 10, 0, 10_000);
        assert_eq!(plain, "no stamps here");
    }

    #[test]
    fn blocks_parse_with_sources_and_origin() {
        let value = json!({"blocks": [
            {"type": "paragraph", "text": "ok", "sources": [{"start": "00:10", "end": "00:40"}]},
            {"type": "callout", "variant": "tip", "text": "extra", "origin": "supplemented"},
            {"type": "nonsense"},
            {"type": "heading", "title": "Via title", "level": 2, "sources": []}
        ]});
        let (blocks, dropped) = parse_blocks(&value);
        assert_eq!(blocks.len(), 3);
        assert_eq!(dropped, 1);
        assert_eq!(blocks[0].source_ranges[0].end_sec, 40);
        assert_eq!(blocks[1].origin, Origin::Supplemented);
        assert!(matches!(&blocks[2].kind, BlockKind::Heading { text, .. } if text == "Via title"));
    }

    #[test]
    fn grounding_marks_uncited_blocks_supplemented() {
        let mut blocks = vec![
            Block::new(
                "a",
                BlockKind::Paragraph {
                    text: "cited".into(),
                },
            )
            .with_sources(vec![SourceRange {
                start_sec: 10,
                end_sec: 5000,
            }]),
            Block::new(
                "b",
                BlockKind::Paragraph {
                    text: "uncited".into(),
                },
            ),
            Block::new(
                "c",
                BlockKind::Paragraph {
                    text: "late".into(),
                },
            )
            .with_sources(vec![SourceRange {
                start_sec: 9000,
                end_sec: 9100,
            }]),
        ];
        let downgraded = ground_blocks(&mut blocks, 600);
        assert_eq!(downgraded, 2);
        assert_eq!(blocks[0].origin, Origin::Lecture);
        assert_eq!(blocks[0].source_ranges[0].end_sec, 600);
        assert_eq!(blocks[1].origin, Origin::Supplemented);
        assert!(blocks[1].low_confidence);
        assert_eq!(blocks[2].origin, Origin::Supplemented);
    }

    #[test]
    fn repair_fixes_what_it_can() {
        let mut blocks = vec![
            Block::new(
                "e1",
                BlockKind::Equation {
                    latex: "$x = \\frac{1}{2}$".into(),
                    caption: None,
                    number: None,
                },
            ),
            Block::new(
                "e2",
                BlockKind::Equation {
                    latex: "\\frac{1".into(),
                    caption: None,
                    number: None,
                },
            ),
            Block::new(
                "t",
                BlockKind::Table {
                    caption: None,
                    header: vec!["a".into(), "b".into()],
                    rows: vec![vec!["1".into()]],
                },
            ),
            Block::new("p", BlockKind::Paragraph { text: "  ".into() }),
        ];
        let repairs = repair_blocks(&mut blocks);
        assert_eq!(repairs, 3);
        assert_eq!(blocks.len(), 3);
        assert!(
            matches!(&blocks[0].kind, BlockKind::Equation { latex, .. } if latex == "x = \\frac{1}{2}")
        );
        assert!(matches!(blocks[1].kind, BlockKind::Paragraph { .. }));
        assert!(blocks[1].low_confidence);
        assert!(matches!(&blocks[2].kind, BlockKind::Table { rows, .. } if rows[0].len() == 2));
        let mut doc = MaterialDoc::new("t");
        doc.blocks = blocks;
        assert!(super::super::blocks::validate(&doc).is_ok());
    }

    #[test]
    fn assemble_orders_sections_and_numbers_equations() {
        let outline = Outline {
            title: "T".into(),
            objectives: vec!["o".into()],
            sections: vec![plan(0, 0, 100), plan(1, 100, 200)],
        };
        let eq = |id: &str| {
            Block::new(
                id,
                BlockKind::Equation {
                    latex: "a=b".into(),
                    caption: None,
                    number: None,
                },
            )
        };
        let sections = vec![(1, vec![eq("s2-1")]), (0, vec![eq("s1-1")])];
        let doc = assemble(&outline, &sections, "English");
        let ids: Vec<_> = doc.blocks.iter().map(|b| b.id.as_str()).collect();
        assert_eq!(ids, vec!["s1-h", "s1-1", "s2-h", "s2-1"]);
        assert!(matches!(
            doc.blocks[3].kind,
            BlockKind::Equation {
                number: Some(2),
                ..
            }
        ));
        assert!(doc.meta.ai_drafted);
    }

    #[test]
    fn locked_blocks_survive_regeneration() {
        let mut old = MaterialDoc::new("T");
        old.blocks = vec![
            Block::new(
                "s1-h",
                BlockKind::Heading {
                    level: 1,
                    text: "A".into(),
                },
            ),
            Block::new(
                "s1-1",
                BlockKind::Paragraph {
                    text: "edited by lecturer".into(),
                },
            ),
            Block::new(
                "keep",
                BlockKind::Paragraph {
                    text: "lecturer addition".into(),
                },
            ),
        ];
        old.blocks[1].locked = true;
        old.blocks[2].locked = true;
        let mut new = MaterialDoc::new("T");
        new.blocks = vec![
            Block::new(
                "s1-h",
                BlockKind::Heading {
                    level: 1,
                    text: "A".into(),
                },
            ),
            Block::new(
                "s1-1",
                BlockKind::Paragraph {
                    text: "ai rewrite".into(),
                },
            ),
            Block::new(
                "s1-2",
                BlockKind::Paragraph {
                    text: "ai extra".into(),
                },
            ),
        ];
        assert_eq!(merge_locked(&mut new, &old), 2);
        let texts: Vec<_> = new.blocks.iter().map(Block::plain_text).collect();
        assert_eq!(texts[1], "edited by lecturer");
        assert_eq!(texts[2], "lecturer addition");
        assert!(texts.contains(&"ai extra".to_owned()));
    }

    #[test]
    fn replace_section_keeps_heading_and_locked() {
        let mut doc = MaterialDoc::new("T");
        let mut locked = Block::new(
            "l",
            BlockKind::Paragraph {
                text: "mine".into(),
            },
        );
        locked.locked = true;
        doc.blocks = vec![
            Block::new(
                "h1",
                BlockKind::Heading {
                    level: 1,
                    text: "One".into(),
                },
            ),
            Block::new("a", BlockKind::Paragraph { text: "old".into() }),
            locked,
            Block::new(
                "h2",
                BlockKind::Heading {
                    level: 1,
                    text: "Two".into(),
                },
            ),
            Block::new(
                "b",
                BlockKind::Paragraph {
                    text: "other section".into(),
                },
            ),
        ];
        let fresh = vec![Block::new("", BlockKind::Paragraph { text: "new".into() })];
        assert!(replace_section(&mut doc, "h1", fresh));
        let texts: Vec<_> = doc.blocks.iter().map(Block::plain_text).collect();
        assert_eq!(texts, vec!["One", "mine", "new", "Two", "other section"]);
        assert!(!replace_section(&mut doc, "missing", vec![]));
    }

    #[test]
    fn fallback_outline_comes_from_chapters() {
        let chapters = vec![
            study::Chapter {
                title: "A".into(),
                summary: "sa".into(),
                start_sec: 0,
                end_sec: 100,
            },
            study::Chapter {
                title: "B".into(),
                summary: "sb".into(),
                start_sec: 100,
                end_sec: 200,
            },
        ];
        let outline = outline_from_chapters(&chapters, "Lec", 200);
        assert_eq!(outline.sections.len(), 2);
        assert_eq!(outline.sections[1].index, 1);
        let blocks =
            fallback_section_blocks(&outline.sections[0], "[00:05] hello there\n[00:09] general");
        assert!(blocks
            .iter()
            .any(|b| b.plain_text().contains("hello there general")));
        assert!(blocks.last().unwrap().origin == Origin::Supplemented);
    }
}
