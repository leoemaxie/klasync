//! Renderers turn a [`MaterialDoc`](super::blocks::MaterialDoc) into a file.
//! Each format is a [`Renderer`](super::registry::Renderer) plug-in; the
//! shared pieces (templates, numbering, escaping) live here.

pub mod docx;
pub mod epub;
pub mod html;
pub mod latex;
pub mod markdown;
pub mod pdf;
pub mod svg;

use super::{
    blocks::{Block, BlockKind, MaterialDoc},
    registry::{RenderContext, RenderInput, RenderOptions, RenderedFile, Renderer, Template},
};

pub fn builtin_templates() -> Vec<Template> {
    vec![
        Template {
            id: "default",
            name: "Klasync",
            accent: "#1f5fbf",
            font_stack: "'Inter', 'Segoe UI', Helvetica, Arial, sans-serif",
            base_size: "11pt",
            footer: Some("Prepared with Klasync"),
        },
        Template {
            id: "classic",
            name: "Classic",
            accent: "#7a1f2b",
            font_stack: "'Georgia', 'Times New Roman', serif",
            base_size: "11.5pt",
            footer: None,
        },
        Template {
            id: "compact",
            name: "Compact",
            accent: "#2b6b4f",
            font_stack: "'Inter', 'Segoe UI', Helvetica, Arial, sans-serif",
            base_size: "10pt",
            footer: Some("Prepared with Klasync"),
        },
    ]
}

/// Renders with the given renderer id from the registry.
pub async fn render_with(
    renderer: &dyn Renderer,
    doc: &MaterialDoc,
    context: &RenderContext,
    template: &Template,
    options: RenderOptions,
) -> Result<RenderedFile, super::registry::RenderError> {
    renderer
        .render(&RenderInput {
            doc,
            context,
            template,
            options,
        })
        .await
}

/// Heading entries for a table of contents: `(level, text, anchor)`.
pub fn outline(doc: &MaterialDoc) -> Vec<(u8, String, String)> {
    doc.blocks
        .iter()
        .filter_map(|block| match &block.kind {
            BlockKind::Heading { level, text } if *level <= 3 => {
                Some((*level, super::blocks::strip_inline(text), anchor(block)))
            }
            _ => None,
        })
        .collect()
}

pub fn anchor(block: &Block) -> String {
    let cleaned: String = block
        .id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    format!("b-{cleaned}")
}

/// Safe file name stem for downloads.
pub fn file_stem(context: &RenderContext) -> String {
    let raw = format!("{}-{}", context.course_code, context.heading);
    let stem: String = raw
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let collapsed = stem
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if collapsed.is_empty() {
        "lecture-material".to_owned()
    } else {
        collapsed.chars().take(80).collect()
    }
}

#[cfg(test)]
pub(crate) fn sample_doc() -> MaterialDoc {
    use super::blocks::{CalloutVariant, ReadingItem, ReviewQuestion, SourceRange};
    let mut doc = MaterialDoc::new("Heat <transfer> & conduction");
    doc.subtitle = Some("Lecture 3".into());
    doc.meta.objectives = vec![
        "Explain **Fourier's law**".into(),
        "Apply it to a wall".into(),
    ];
    doc.blocks = vec![
        Block::new(
            "h1",
            BlockKind::Heading {
                level: 1,
                text: "Conduction".into(),
            },
        ),
        Block::new(
            "p1",
            BlockKind::Paragraph {
                text: "Heat flows from *hot* to cold, as in `q = -k dT/dx`.".into(),
            },
        )
        .with_sources(vec![SourceRange {
            start_sec: 5,
            end_sec: 70,
        }]),
        Block::new(
            "d1",
            BlockKind::Definition {
                term: "Thermal conductivity".into(),
                text: "How well a material conducts heat.".into(),
            },
        ),
        Block::new(
            "e1",
            BlockKind::Equation {
                latex: "q = -k \\frac{dT}{dx}".into(),
                caption: Some("Fourier's law".into()),
                number: Some(1),
            },
        ),
        Block::new(
            "w1",
            BlockKind::WorkedExample {
                title: "Brick wall".into(),
                problem: "A wall is 0.2 m thick.".into(),
                steps: vec!["Take k = 0.7.".into(), "Compute q.".into()],
                answer: Some("q = 35 W/m2".into()),
            },
        ),
        Block::new(
            "t1",
            BlockKind::Table {
                caption: Some("Conductivities".into()),
                header: vec!["Material".into(), "k (W/mK)".into()],
                rows: vec![
                    vec!["Copper".into(), "400".into()],
                    vec!["Brick".into(), "0.7".into()],
                ],
            },
        ),
        Block::new(
            "c1",
            BlockKind::Callout {
                variant: CalloutVariant::Misconception,
                text: "Cold does not flow.".into(),
            },
        )
        .supplemented(),
        Block::new(
            "q1",
            BlockKind::ReviewQuestions {
                questions: vec![ReviewQuestion {
                    question: "State Fourier's law.".into(),
                    answer: Some("q = -k dT/dx".into()),
                    difficulty: None,
                }],
            },
        ),
        Block::new(
            "r1",
            BlockKind::FurtherReading {
                items: vec![ReadingItem {
                    title: "Incropera, ch. 1".into(),
                    note: None,
                }],
            },
        ),
        Block::new(
            "s1",
            BlockKind::Summary {
                points: vec!["Heat flows down gradients.".into()],
            },
        ),
    ];
    doc
}

#[cfg(test)]
pub(crate) fn sample_context() -> RenderContext {
    RenderContext {
        course_code: "ME 501".into(),
        course_title: "Heat Transfer".into(),
        heading: "Conduction".into(),
        lecturer_name: Some("Dr. A. Lecturer".into()),
        academic_session: Some("2025/2026".into()),
        semester: Some("Second Semester".into()),
        generated_at: chrono::Utc::now(),
        version_label: Some("v2".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_stem_is_safe() {
        let mut cx = sample_context();
        cx.heading = "Heat/../transfer & more".into();
        assert_eq!(file_stem(&cx), "ME-501-Heat-transfer-more");
    }

    #[test]
    fn outline_lists_headings() {
        let doc = sample_doc();
        let entries = outline(&doc);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].1, "Conduction");
        assert_eq!(entries[0].2, "b-h1");
    }
}
