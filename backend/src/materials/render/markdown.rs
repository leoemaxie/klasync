//! Markdown renderer: plain-text export that matches the notes export students
//! already know. Equations use `$$ … $$`; figures fall back to their text.

use async_trait::async_trait;

use crate::materials::{
    blocks::{Block, BlockKind, CalloutVariant, MaterialDoc, Origin},
    registry::{RenderContext, RenderError, RenderInput, RenderedFile, Renderer},
};

pub struct MarkdownRenderer;

#[async_trait]
impl Renderer for MarkdownRenderer {
    fn id(&self) -> &'static str {
        "md"
    }
    fn label(&self) -> &'static str {
        "Markdown"
    }
    fn content_type(&self) -> &'static str {
        "text/markdown; charset=utf-8"
    }
    fn extension(&self) -> &'static str {
        "md"
    }
    async fn render(&self, input: &RenderInput<'_>) -> Result<RenderedFile, RenderError> {
        Ok(RenderedFile {
            bytes: render_markdown(input.doc, input.context).into_bytes(),
            content_type: "text/markdown; charset=utf-8",
            extension: "md",
        })
    }
}

fn cell(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}

fn block_md(block: &Block, out: &mut String) {
    let tag = if block.origin == Origin::Supplemented {
        " _(supplemented)_"
    } else {
        ""
    };
    match &block.kind {
        BlockKind::Heading { level, text } => {
            let hashes = "#".repeat((*level as usize + 1).min(6));
            out.push_str(&format!("{hashes} {text}\n\n"));
        }
        BlockKind::Paragraph { text } => out.push_str(&format!("{text}{tag}\n\n")),
        BlockKind::Definition { term, text } => {
            out.push_str(&format!("**{term}**: {text}{tag}\n\n"))
        }
        BlockKind::KeyResult { title, text } => out.push_str(&format!(
            "> **{}.** {text}{tag}\n\n",
            title.as_deref().unwrap_or("Key result")
        )),
        BlockKind::WorkedExample {
            title,
            problem,
            steps,
            answer,
        } => {
            out.push_str(&format!("#### Worked example: {title}{tag}\n\n{problem}\n\n"));
            for (i, step) in steps.iter().enumerate() {
                out.push_str(&format!("{}. {step}\n", i + 1));
            }
            out.push('\n');
            if let Some(answer) = answer {
                out.push_str(&format!("**Answer:** {answer}\n\n"));
            }
        }
        BlockKind::Equation {
            latex,
            caption,
            number,
        } => {
            let tag_part = number.map(|n| format!(" \\tag{{{n}}}")).unwrap_or_default();
            out.push_str(&format!("$$\n{latex}{tag_part}\n$$\n\n"));
            if let Some(caption) = caption {
                out.push_str(&format!("_{caption}_\n\n"));
            }
        }
        BlockKind::Figure {
            caption,
            alt,
            text_fallback,
            ..
        } => {
            out.push_str(&format!("**Figure.** {caption}\n\n"));
            match text_fallback {
                Some(text) if !text.trim().is_empty() => {
                    for line in text.lines().filter(|l| !l.trim().is_empty()) {
                        out.push_str(&format!("- {}\n", line.trim_start_matches(['-', '*', ' '])));
                    }
                    out.push('\n');
                }
                _ => out.push_str(&format!("_{alt}_\n\n")),
            }
        }
        BlockKind::Table {
            caption,
            header,
            rows,
        } => {
            if let Some(caption) = caption {
                out.push_str(&format!("**{caption}**\n\n"));
            }
            out.push_str(&format!(
                "| {} |\n| {} |\n",
                header.iter().map(|c| cell(c)).collect::<Vec<_>>().join(" | "),
                header.iter().map(|_| "---").collect::<Vec<_>>().join(" | ")
            ));
            for row in rows {
                out.push_str(&format!(
                    "| {} |\n",
                    row.iter().map(|c| cell(c)).collect::<Vec<_>>().join(" | ")
                ));
            }
            out.push('\n');
        }
        BlockKind::Callout { variant, text } => {
            let label = match variant {
                CalloutVariant::Note => "Note",
                CalloutVariant::Tip => "Tip",
                CalloutVariant::Warning => "Warning",
                CalloutVariant::Misconception => "Common misconception",
            };
            out.push_str(&format!("> **{label}.** {text}{tag}\n\n"));
        }
        BlockKind::ReviewQuestions { questions } => {
            out.push_str("### Review questions\n\n");
            for (i, q) in questions.iter().enumerate() {
                out.push_str(&format!("{}. {}\n", i + 1, q.question));
                if let Some(answer) = &q.answer {
                    out.push_str(&format!("   - _Answer:_ {answer}\n"));
                }
            }
            out.push('\n');
        }
        BlockKind::FurtherReading { items } => {
            out.push_str("### Further reading\n\n");
            for item in items {
                match &item.note {
                    Some(note) => out.push_str(&format!("- {} — {note}\n", item.title)),
                    None => out.push_str(&format!("- {}\n", item.title)),
                }
            }
            out.push('\n');
        }
        BlockKind::Summary { points } => {
            out.push_str("### Summary\n\n");
            for point in points {
                out.push_str(&format!("- {point}\n"));
            }
            out.push('\n');
        }
    }
}

pub fn render_markdown(doc: &MaterialDoc, context: &RenderContext) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", doc.title));
    if let Some(subtitle) = &doc.subtitle {
        out.push_str(&format!("_{subtitle}_\n\n"));
    }
    let mut meta = vec![format!("{} {}", context.course_code, context.course_title)];
    if let Some(name) = &context.lecturer_name {
        meta.push(name.clone());
    }
    if let Some(session) = &context.academic_session {
        meta.push(session.clone());
    }
    out.push_str(&format!("{}\n\n", meta.join(" · ")));
    if !doc.meta.objectives.is_empty() {
        out.push_str("## Learning objectives\n\n");
        for objective in &doc.meta.objectives {
            out.push_str(&format!("- {objective}\n"));
        }
        out.push('\n');
    }
    for block in &doc.blocks {
        block_md(block, &mut out);
    }
    while out.ends_with("\n\n") {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::render::{sample_context, sample_doc};

    #[test]
    fn renders_every_block_type() {
        let md = render_markdown(&sample_doc(), &sample_context());
        assert!(md.starts_with("# Heat <transfer> & conduction"));
        assert!(md.contains("## Conduction"));
        assert!(md.contains("**Thermal conductivity**: How well"));
        assert!(md.contains("$$\nq = -k \\frac{dT}{dx} \\tag{1}\n$$"));
        assert!(md.contains("#### Worked example: Brick wall"));
        assert!(md.contains("| Material | k (W/mK) |\n| --- | --- |"));
        assert!(md.contains("> **Common misconception.** Cold does not flow. _(supplemented)_"));
        assert!(md.contains("1. State Fourier's law."));
        assert!(md.contains("### Summary"));
        assert!(!md.ends_with("\n\n"));
    }
}
