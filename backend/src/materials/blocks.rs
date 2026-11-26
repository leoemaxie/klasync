//! The block document: the single source of truth for a lecture material.
//!
//! Everything else (generators, renderers, delivery channels, analyzers) works
//! on this typed tree, never on a Markdown blob. Text fields hold *inline
//! Markdown*: `**bold**`, `*italic*`, `` `code` `` and `$math$` (left as text).
//! Each block records where it came from (`source_ranges`, in transcript
//! seconds) and whether the AI added it beyond the lecture (`origin`).

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Bump when the shape changes incompatibly; stored in every document.
pub const SCHEMA_VERSION: u32 = 1;

const MAX_TEXT_CHARS: usize = 20_000;
const MAX_BLOCKS: usize = 2_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MaterialDoc {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(default)]
    pub meta: DocMeta,
    #[serde(default)]
    pub blocks: Vec<Block>,
}

fn default_schema_version() -> u32 {
    SCHEMA_VERSION
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DocMeta {
    /// Learning objectives shown at the top of the material.
    #[serde(default)]
    pub objectives: Vec<String>,
    #[serde(default)]
    pub language: Option<String>,
    /// True while the content is an unreviewed AI draft.
    #[serde(default)]
    pub ai_drafted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Block {
    pub id: String,
    #[serde(flatten)]
    pub kind: BlockKind,
    #[serde(default)]
    pub origin: Origin,
    /// Transcript time spans this block was written from.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_ranges: Vec<SourceRange>,
    /// Lecturer-protected: regeneration never overwrites it.
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub low_confidence: bool,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// Written from what was said in the lecture.
    #[default]
    Lecture,
    /// Added by the AI beyond the lecture; shown to the lecturer to accept or drop.
    Supplemented,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceRange {
    pub start_sec: i32,
    pub end_sec: i32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CalloutVariant {
    Note,
    Tip,
    Warning,
    Misconception,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReviewQuestion {
    pub question: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReadingItem {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BlockKind {
    Heading {
        level: u8,
        text: String,
    },
    Paragraph {
        text: String,
    },
    Definition {
        term: String,
        text: String,
    },
    KeyResult {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        text: String,
    },
    WorkedExample {
        title: String,
        problem: String,
        #[serde(default)]
        steps: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        answer: Option<String>,
    },
    Equation {
        latex: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        caption: Option<String>,
        /// Continuous equation number, assigned by the assembler.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        number: Option<u32>,
    },
    Figure {
        caption: String,
        /// Accessibility text; generated from the concept map for diagrams.
        alt: String,
        /// A `session_visualizations` row (concept map) this figure draws.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        visualization_id: Option<Uuid>,
        /// Plain-text rendering of the figure for formats without graphics.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        text_fallback: Option<String>,
    },
    Table {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        caption: Option<String>,
        header: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    Callout {
        variant: CalloutVariant,
        text: String,
    },
    ReviewQuestions {
        questions: Vec<ReviewQuestion>,
    },
    FurtherReading {
        items: Vec<ReadingItem>,
    },
    Summary {
        points: Vec<String>,
    },
}

impl BlockKind {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Heading { .. } => "heading",
            Self::Paragraph { .. } => "paragraph",
            Self::Definition { .. } => "definition",
            Self::KeyResult { .. } => "key_result",
            Self::WorkedExample { .. } => "worked_example",
            Self::Equation { .. } => "equation",
            Self::Figure { .. } => "figure",
            Self::Table { .. } => "table",
            Self::Callout { .. } => "callout",
            Self::ReviewQuestions { .. } => "review_questions",
            Self::FurtherReading { .. } => "further_reading",
            Self::Summary { .. } => "summary",
        }
    }
}

impl Block {
    pub fn new(id: impl Into<String>, kind: BlockKind) -> Self {
        Self {
            id: id.into(),
            kind,
            origin: Origin::Lecture,
            source_ranges: Vec::new(),
            locked: false,
            low_confidence: false,
        }
    }

    pub fn supplemented(mut self) -> Self {
        self.origin = Origin::Supplemented;
        self
    }

    pub fn with_sources(mut self, ranges: Vec<SourceRange>) -> Self {
        self.source_ranges = ranges;
        self
    }

    /// All human-readable text in the block, for search, embeddings and diffs.
    pub fn plain_text(&self) -> String {
        match &self.kind {
            BlockKind::Heading { text, .. }
            | BlockKind::Paragraph { text }
            | BlockKind::Callout { text, .. } => strip_inline(text),
            BlockKind::Definition { term, text } => {
                format!("{}: {}", strip_inline(term), strip_inline(text))
            }
            BlockKind::KeyResult { title, text } => {
                let body = strip_inline(text);
                match title {
                    Some(title) => format!("{}: {body}", strip_inline(title)),
                    None => body,
                }
            }
            BlockKind::WorkedExample {
                title,
                problem,
                steps,
                answer,
            } => {
                let mut out = vec![strip_inline(title), strip_inline(problem)];
                out.extend(steps.iter().map(|s| strip_inline(s)));
                if let Some(answer) = answer {
                    out.push(strip_inline(answer));
                }
                out.join(" ")
            }
            BlockKind::Equation { latex, caption, .. } => match caption {
                Some(caption) => format!("{latex} {}", strip_inline(caption)),
                None => latex.clone(),
            },
            BlockKind::Figure { caption, alt, .. } => {
                format!("{} {}", strip_inline(caption), strip_inline(alt))
            }
            BlockKind::Table {
                caption,
                header,
                rows,
            } => {
                let mut out: Vec<String> = Vec::new();
                if let Some(caption) = caption {
                    out.push(strip_inline(caption));
                }
                out.extend(header.iter().map(|c| strip_inline(c)));
                for row in rows {
                    out.extend(row.iter().map(|c| strip_inline(c)));
                }
                out.join(" ")
            }
            BlockKind::ReviewQuestions { questions } => questions
                .iter()
                .map(|q| strip_inline(&q.question))
                .collect::<Vec<_>>()
                .join(" "),
            BlockKind::FurtherReading { items } => items
                .iter()
                .map(|i| strip_inline(&i.title))
                .collect::<Vec<_>>()
                .join(" "),
            BlockKind::Summary { points } => points
                .iter()
                .map(|p| strip_inline(p))
                .collect::<Vec<_>>()
                .join(" "),
        }
    }
}

impl MaterialDoc {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            title: title.into(),
            subtitle: None,
            meta: DocMeta::default(),
            blocks: Vec::new(),
        }
    }

    pub fn plain_text(&self) -> String {
        self.blocks
            .iter()
            .map(Block::plain_text)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Splits the document into sections at level-1/2 headings. Blocks before
    /// the first heading form an untitled opening section.
    pub fn sections(&self) -> Vec<Section<'_>> {
        let mut sections: Vec<Section<'_>> = Vec::new();
        for block in &self.blocks {
            let starts_section =
                matches!(&block.kind, BlockKind::Heading { level, .. } if *level <= 2);
            if starts_section || sections.is_empty() {
                let heading = match &block.kind {
                    BlockKind::Heading { text, .. } if starts_section => strip_inline(text),
                    _ => String::new(),
                };
                sections.push(Section {
                    block_id: block.id.as_str(),
                    heading,
                    blocks: Vec::new(),
                });
            }
            if let Some(section) = sections.last_mut() {
                section.blocks.push(block);
            }
        }
        sections
    }

    /// Re-numbers equations continuously from `start` and returns the next number.
    pub fn number_equations(&mut self, start: u32) -> u32 {
        let mut next = start;
        for block in &mut self.blocks {
            if let BlockKind::Equation { number, .. } = &mut block.kind {
                *number = Some(next);
                next += 1;
            }
        }
        next
    }

    /// Gives every block with an empty or duplicate id a fresh stable one.
    pub fn normalise_ids(&mut self, prefix: &str) {
        let mut seen: HashSet<String> = HashSet::new();
        for (index, block) in self.blocks.iter_mut().enumerate() {
            if block.id.trim().is_empty() || seen.contains(&block.id) {
                let mut candidate = format!("{prefix}{}", index + 1);
                let mut bump = 0;
                while seen.contains(&candidate) {
                    bump += 1;
                    candidate = format!("{prefix}{}-{bump}", index + 1);
                }
                block.id = candidate;
            }
            seen.insert(block.id.clone());
        }
    }
}

#[derive(Debug, Clone)]
pub struct Section<'a> {
    pub block_id: &'a str,
    pub heading: String,
    pub blocks: Vec<&'a Block>,
}

impl Section<'_> {
    pub fn body_text(&self) -> String {
        self.blocks
            .iter()
            .skip(usize::from(!self.heading.is_empty()))
            .map(|b| b.plain_text())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

// ---- validation --------------------------------------------------------------

/// Checks a document against the schema rules. Returns every problem found so
/// a generator or an editor can show them all at once.
pub fn validate(doc: &MaterialDoc) -> Result<(), Vec<String>> {
    let mut problems = Vec::new();
    if doc.schema_version != SCHEMA_VERSION {
        problems.push(format!(
            "unsupported schema version {} (expected {SCHEMA_VERSION})",
            doc.schema_version
        ));
    }
    if doc.title.trim().is_empty() {
        problems.push("the document needs a title".to_owned());
    }
    if doc.blocks.is_empty() {
        problems.push("the document has no content".to_owned());
    }
    if doc.blocks.len() > MAX_BLOCKS {
        problems.push(format!("too many blocks (limit {MAX_BLOCKS})"));
    }
    let mut ids = HashSet::new();
    for block in &doc.blocks {
        if block.id.trim().is_empty() {
            problems.push("a block has no id".to_owned());
        } else if !ids.insert(block.id.as_str()) {
            problems.push(format!("duplicate block id {}", block.id));
        }
        for range in &block.source_ranges {
            if range.start_sec < 0 || range.end_sec < range.start_sec {
                problems.push(format!("block {} has an invalid source range", block.id));
            }
        }
        validate_block(block, &mut problems);
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems)
    }
}

fn check_text(block: &Block, label: &str, text: &str, problems: &mut Vec<String>) {
    if text.trim().is_empty() {
        problems.push(format!("block {} has an empty {label}", block.id));
    }
    if text.chars().count() > MAX_TEXT_CHARS {
        problems.push(format!("block {} has too much text in {label}", block.id));
    }
    if !fences_closed(text) {
        problems.push(format!("block {} has an unclosed code fence", block.id));
    }
}

fn validate_block(block: &Block, problems: &mut Vec<String>) {
    match &block.kind {
        BlockKind::Heading { level, text } => {
            if !(1..=4).contains(level) {
                problems.push(format!("block {} has heading level {level}", block.id));
            }
            check_text(block, "heading", text, problems);
        }
        BlockKind::Paragraph { text } | BlockKind::Callout { text, .. } => {
            check_text(block, "text", text, problems)
        }
        BlockKind::Definition { term, text } => {
            check_text(block, "term", term, problems);
            check_text(block, "definition", text, problems);
        }
        BlockKind::KeyResult { text, .. } => check_text(block, "text", text, problems),
        BlockKind::WorkedExample {
            title,
            problem,
            steps,
            ..
        } => {
            check_text(block, "title", title, problems);
            check_text(block, "problem", problem, problems);
            if steps.is_empty() {
                problems.push(format!("worked example {} has no steps", block.id));
            }
        }
        BlockKind::Equation { latex, .. } => {
            if latex.trim().is_empty() {
                problems.push(format!("equation {} is empty", block.id));
            } else if let Err(reason) = check_latex(latex) {
                problems.push(format!("equation {} does not parse: {reason}", block.id));
            }
        }
        BlockKind::Figure { caption, alt, .. } => {
            check_text(block, "caption", caption, problems);
            check_text(block, "alt text", alt, problems);
        }
        BlockKind::Table { header, rows, .. } => {
            if header.is_empty() {
                problems.push(format!("table {} has no header", block.id));
            }
            if rows.iter().any(|row| row.len() != header.len()) {
                problems.push(format!("table {} has rows of the wrong width", block.id));
            }
        }
        BlockKind::ReviewQuestions { questions } => {
            if questions.is_empty() || questions.iter().any(|q| q.question.trim().is_empty()) {
                problems.push(format!("review questions {} are empty", block.id));
            }
        }
        BlockKind::FurtherReading { items } => {
            if items.is_empty() || items.iter().any(|i| i.title.trim().is_empty()) {
                problems.push(format!("further reading {} is empty", block.id));
            }
        }
        BlockKind::Summary { points } => {
            if points.is_empty() || points.iter().any(|p| p.trim().is_empty()) {
                problems.push(format!("summary {} is empty", block.id));
            }
        }
    }
}

/// Code fences (```) must open and close.
pub fn fences_closed(text: &str) -> bool {
    text.matches("```").count() & 1 == 0
}

/// A cheap structural LaTeX check: balanced braces, `\left`/`\right` and
/// `\begin`/`\end` pairs. It cannot prove the formula compiles, but it catches
/// the truncated or mangled output models produce.
pub fn check_latex(latex: &str) -> Result<(), String> {
    let mut depth: i32 = 0;
    let mut chars = latex.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                // Skip the escaped character so `\{` and `\}` are not counted.
                chars.next();
            }
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth < 0 {
                    return Err("unmatched closing brace".to_owned());
                }
            }
            _ => {}
        }
    }
    if depth != 0 {
        return Err("unclosed brace".to_owned());
    }
    if latex.matches("\\left").count() != latex.matches("\\right").count() {
        return Err("\\left and \\right do not pair up".to_owned());
    }
    if latex.matches("\\begin{").count() != latex.matches("\\end{").count() {
        return Err("\\begin and \\end do not pair up".to_owned());
    }
    if latex.contains('$') {
        return Err("remove the surrounding $ signs".to_owned());
    }
    Ok(())
}

// ---- inline markdown ---------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
}

/// Parses `**bold**`, `*italic*` and `` `code` `` into spans. Anything
/// unterminated is kept as literal text, so a stray `*` never loses content.
pub fn parse_inline(input: &str) -> Vec<Span> {
    let chars: Vec<char> = input.chars().collect();
    let mut spans: Vec<Span> = Vec::new();
    let mut buffer = String::new();
    let (mut bold, mut italic) = (false, false);
    let mut i = 0;

    let flush = |buffer: &mut String, spans: &mut Vec<Span>, bold: bool, italic: bool| {
        if !buffer.is_empty() {
            spans.push(Span {
                text: std::mem::take(buffer),
                bold,
                italic,
                code: false,
            });
        }
    };

    while i < chars.len() {
        let c = chars[i];
        if c == '`' {
            if let Some(end) = chars[i + 1..].iter().position(|&x| x == '`') {
                flush(&mut buffer, &mut spans, bold, italic);
                spans.push(Span {
                    text: chars[i + 1..i + 1 + end].iter().collect(),
                    bold,
                    italic,
                    code: true,
                });
                i += end + 2;
                continue;
            }
        } else if c == '*' && chars.get(i + 1) == Some(&'*') {
            let closing = bold;
            let can_open = chars[i + 2..].windows(2).any(|w| w == ['*', '*']);
            if closing || can_open {
                flush(&mut buffer, &mut spans, bold, italic);
                bold = !bold;
                i += 2;
                continue;
            }
        } else if c == '*' {
            let can_open = chars[i + 1..].contains(&'*');
            if italic || can_open {
                flush(&mut buffer, &mut spans, bold, italic);
                italic = !italic;
                i += 1;
                continue;
            }
        }
        buffer.push(c);
        i += 1;
    }
    flush(&mut buffer, &mut spans, bold, italic);
    spans
}

/// The text with inline markup removed.
pub fn strip_inline(input: &str) -> String {
    parse_inline(input)
        .into_iter()
        .map(|span| span.text)
        .collect()
}

// ---- helpers shared by generators --------------------------------------------

/// `mm:ss` or `h:mm:ss` into seconds.
pub fn parse_stamp(value: &str) -> Option<i32> {
    let parts: Vec<&str> = value.trim().trim_matches(['[', ']']).split(':').collect();
    let nums: Option<Vec<i32>> = parts.iter().map(|p| p.trim().parse::<i32>().ok()).collect();
    match nums?.as_slice() {
        [m, s] => Some(m * 60 + s),
        [h, m, s] => Some(h * 3600 + m * 60 + s),
        [s] => Some(*s),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn para(id: &str, text: &str) -> Block {
        Block::new(id, BlockKind::Paragraph { text: text.into() })
    }

    #[test]
    fn round_trips_through_json() {
        let mut doc = MaterialDoc::new("Heat transfer");
        doc.blocks.push(Block::new(
            "b1",
            BlockKind::Heading {
                level: 1,
                text: "Conduction".into(),
            },
        ));
        doc.blocks.push(
            Block::new(
                "b2",
                BlockKind::Equation {
                    latex: "q = -k \\frac{dT}{dx}".into(),
                    caption: None,
                    number: None,
                },
            )
            .with_sources(vec![SourceRange {
                start_sec: 10,
                end_sec: 90,
            }]),
        );
        let json = serde_json::to_value(&doc).unwrap();
        assert_eq!(json["blocks"][1]["type"], "equation");
        assert_eq!(json["blocks"][1]["source_ranges"][0]["start_sec"], 10);
        let back: MaterialDoc = serde_json::from_value(json).unwrap();
        assert_eq!(back, doc);
    }

    #[test]
    fn validation_reports_every_problem() {
        let mut doc = MaterialDoc::new(" ");
        doc.blocks.push(para("a", "ok"));
        doc.blocks.push(para("a", "dup"));
        doc.blocks.push(Block::new(
            "t",
            BlockKind::Table {
                caption: None,
                header: vec!["x".into(), "y".into()],
                rows: vec![vec!["1".into()]],
            },
        ));
        doc.blocks.push(Block::new(
            "e",
            BlockKind::Equation {
                latex: "\\frac{a}{b".into(),
                caption: None,
                number: None,
            },
        ));
        let problems = validate(&doc).unwrap_err();
        assert!(problems.iter().any(|p| p.contains("title")));
        assert!(problems.iter().any(|p| p.contains("duplicate block id a")));
        assert!(problems.iter().any(|p| p.contains("wrong width")));
        assert!(problems.iter().any(|p| p.contains("does not parse")));
    }

    #[test]
    fn latex_checks_catch_mangled_output() {
        assert!(check_latex("\\frac{a}{b}").is_ok());
        assert!(check_latex("\\{a\\}").is_ok());
        assert!(check_latex("\\left( x \\right)").is_ok());
        assert!(check_latex("\\left( x").is_err());
        assert!(check_latex("\\begin{aligned} a \\end{aligned}").is_ok());
        assert!(check_latex("\\begin{aligned} a").is_err());
        assert!(check_latex("$x$").is_err());
    }

    #[test]
    fn inline_markup_parses_and_never_loses_text() {
        let spans = parse_inline("a **bold** and *it* and `code` 2*3");
        let text: String = spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(text, "a bold and it and code 2*3");
        assert!(spans.iter().any(|s| s.bold && s.text == "bold"));
        assert!(spans.iter().any(|s| s.italic && s.text == "it"));
        assert!(spans.iter().any(|s| s.code && s.text == "code"));
        assert_eq!(strip_inline("**x"), "**x");
    }

    #[test]
    fn sections_split_at_headings_and_ids_normalise() {
        let mut doc = MaterialDoc::new("T");
        doc.blocks.push(para("", "intro"));
        doc.blocks.push(Block::new(
            "h1",
            BlockKind::Heading {
                level: 2,
                text: "First".into(),
            },
        ));
        doc.blocks.push(para("", "body"));
        doc.normalise_ids("b");
        let ids: Vec<_> = doc.blocks.iter().map(|b| b.id.clone()).collect();
        assert_eq!(ids, vec!["b1", "h1", "b3"]);
        let sections = doc.sections();
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[1].heading, "First");
        assert_eq!(sections[1].body_text(), "body");
    }

    #[test]
    fn equations_number_continuously() {
        let mut doc = MaterialDoc::new("T");
        for i in 0..2 {
            doc.blocks.push(Block::new(
                format!("e{i}"),
                BlockKind::Equation {
                    latex: "a=b".into(),
                    caption: None,
                    number: None,
                },
            ));
        }
        assert_eq!(doc.number_equations(5), 7);
        assert!(matches!(
            doc.blocks[1].kind,
            BlockKind::Equation {
                number: Some(6),
                ..
            }
        ));
    }

    #[test]
    fn stamps_parse() {
        assert_eq!(parse_stamp("01:15"), Some(75));
        assert_eq!(parse_stamp("[1:02:03]"), Some(3723));
        assert_eq!(parse_stamp("abc"), None);
    }
}
