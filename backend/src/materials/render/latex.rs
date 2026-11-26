//! LaTeX renderer: a standalone `.tex` file (article class) that compiles with
//! plain `pdflatex`/`xelatex`. Equations are passed through as written, since
//! the block document stores them as LaTeX; everything else is escaped.

use async_trait::async_trait;

use crate::materials::{
    blocks::{parse_inline, Block, BlockKind, CalloutVariant, MaterialDoc},
    registry::{RenderContext, RenderError, RenderInput, RenderedFile, Renderer},
};

pub struct LatexRenderer;

#[async_trait]
impl Renderer for LatexRenderer {
    fn id(&self) -> &'static str {
        "tex"
    }
    fn label(&self) -> &'static str {
        "LaTeX"
    }
    fn content_type(&self) -> &'static str {
        "application/x-tex; charset=utf-8"
    }
    fn extension(&self) -> &'static str {
        "tex"
    }
    async fn render(&self, input: &RenderInput<'_>) -> Result<RenderedFile, RenderError> {
        Ok(RenderedFile {
            bytes: render_latex(input.doc, input.context, input.options.toc).into_bytes(),
            content_type: "application/x-tex; charset=utf-8",
            extension: "tex",
        })
    }
}

/// Escapes LaTeX special characters in running text.
pub fn escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '\\' => out.push_str("\\textbackslash{}"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '$' => out.push_str("\\$"),
            '&' => out.push_str("\\&"),
            '#' => out.push_str("\\#"),
            '%' => out.push_str("\\%"),
            '_' => out.push_str("\\_"),
            '^' => out.push_str("\\textasciicircum{}"),
            '~' => out.push_str("\\textasciitilde{}"),
            '\n' | '\r' => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// Inline Markdown (`**b**`, `*i*`, `` `c` ``) to LaTeX.
pub fn inline(text: &str) -> String {
    let mut out = String::new();
    for span in parse_inline(text) {
        let mut piece = escape(&span.text);
        if span.code {
            piece = format!("\\texttt{{{piece}}}");
        }
        if span.italic {
            piece = format!("\\emph{{{piece}}}");
        }
        if span.bold {
            piece = format!("\\textbf{{{piece}}}");
        }
        out.push_str(&piece);
    }
    out
}

fn itemize(items: impl Iterator<Item = String>, ordered: bool, out: &mut String) {
    let env = if ordered { "enumerate" } else { "itemize" };
    out.push_str(&format!("\\begin{{{env}}}\n"));
    for item in items {
        out.push_str(&format!("\\item {item}\n"));
    }
    out.push_str(&format!("\\end{{{env}}}\n\n"));
}

fn block_tex(block: &Block, out: &mut String) {
    match &block.kind {
        BlockKind::Heading { level, text } => {
            let cmd = match level {
                1 => "section",
                2 => "subsection",
                3 => "subsubsection",
                _ => "paragraph",
            };
            out.push_str(&format!("\\{cmd}{{{}}}\n\n", inline(text)));
        }
        BlockKind::Paragraph { text } => out.push_str(&format!("{}\n\n", inline(text))),
        BlockKind::Definition { term, text } => out.push_str(&format!(
            "\\begin{{quote}}\\textbf{{{}}}: {}\\end{{quote}}\n\n",
            inline(term),
            inline(text)
        )),
        BlockKind::KeyResult { title, text } => out.push_str(&format!(
            "\\begin{{quote}}\\textbf{{{}.}} {}\\end{{quote}}\n\n",
            escape(title.as_deref().unwrap_or("Key result")),
            inline(text)
        )),
        BlockKind::WorkedExample {
            title,
            problem,
            steps,
            answer,
        } => {
            out.push_str(&format!(
                "\\paragraph{{Worked example: {}}} {}\n\n",
                inline(title),
                inline(problem)
            ));
            itemize(steps.iter().map(|s| inline(s)), true, out);
            if let Some(answer) = answer {
                out.push_str(&format!("\\textbf{{Answer:}} {}\n\n", inline(answer)));
            }
        }
        BlockKind::Equation {
            latex,
            caption,
            number,
        } => {
            let tag = number.map(|n| format!(" \\tag{{{n}}}")).unwrap_or_default();
            out.push_str(&format!("\\[\n{latex}{tag}\n\\]\n\n"));
            if let Some(caption) = caption {
                out.push_str(&format!(
                    "\\begin{{center}}\\emph{{{}}}\\end{{center}}\n\n",
                    inline(caption)
                ));
            }
        }
        BlockKind::Figure {
            caption,
            alt,
            text_fallback,
            ..
        } => {
            out.push_str(&format!("\\paragraph{{Figure.}} {}\n\n", inline(caption)));
            match text_fallback {
                Some(text) if !text.trim().is_empty() => itemize(
                    text.lines()
                        .filter(|l| !l.trim().is_empty())
                        .map(|l| inline(l.trim_start_matches(['-', '*', ' ']))),
                    false,
                    out,
                ),
                _ => out.push_str(&format!("\\emph{{{}}}\n\n", inline(alt))),
            }
        }
        BlockKind::Table {
            caption,
            header,
            rows,
        } => {
            let spec = "l".repeat(header.len().max(1));
            out.push_str("\\begin{table}[h]\\centering\n");
            out.push_str(&format!("\\begin{{tabular}}{{{spec}}}\n\\hline\n"));
            let line = |cells: &[String]| {
                cells
                    .iter()
                    .map(|c| inline(c))
                    .collect::<Vec<_>>()
                    .join(" & ")
            };
            out.push_str(&format!(
                "\\textbf{{{}}} \\\\\n\\hline\n",
                line(header).replace(" & ", "} & \\textbf{")
            ));
            for row in rows {
                out.push_str(&format!("{} \\\\\n", line(row)));
            }
            out.push_str("\\hline\n\\end{tabular}\n");
            if let Some(caption) = caption {
                out.push_str(&format!("\\caption*{{{}}}\n", inline(caption)));
            }
            out.push_str("\\end{table}\n\n");
        }
        BlockKind::Callout { variant, text } => {
            let label = match variant {
                CalloutVariant::Note => "Note",
                CalloutVariant::Tip => "Tip",
                CalloutVariant::Warning => "Warning",
                CalloutVariant::Misconception => "Common misconception",
            };
            out.push_str(&format!(
                "\\begin{{quote}}\\textbf{{{label}.}} {}\\end{{quote}}\n\n",
                inline(text)
            ));
        }
        BlockKind::ReviewQuestions { questions } => {
            out.push_str("\\subsection*{Review questions}\n");
            itemize(
                questions.iter().map(|q| match &q.answer {
                    Some(a) => format!(
                        "{} \\\\ \\emph{{Answer:}} {}",
                        inline(&q.question),
                        inline(a)
                    ),
                    None => inline(&q.question),
                }),
                true,
                out,
            );
        }
        BlockKind::FurtherReading { items } => {
            out.push_str("\\subsection*{Further reading}\n");
            itemize(
                items.iter().map(|i| match &i.note {
                    Some(n) => format!("{} --- {}", inline(&i.title), inline(n)),
                    None => inline(&i.title),
                }),
                false,
                out,
            );
        }
        BlockKind::Summary { points } => {
            out.push_str("\\subsection*{Summary}\n");
            itemize(points.iter().map(|p| inline(p)), false, out);
        }
    }
}

pub fn render_latex(doc: &MaterialDoc, context: &RenderContext, toc: bool) -> String {
    let mut out = String::new();
    out.push_str("\\documentclass[11pt]{article}\n");
    out.push_str("\\usepackage[utf8]{inputenc}\n\\usepackage[T1]{fontenc}\n");
    out.push_str("\\usepackage{amsmath,amssymb,caption,hyperref}\n");
    out.push_str("\\usepackage[margin=2.5cm]{geometry}\n");
    out.push_str(&format!("\\title{{{}}}\n", escape(&doc.title)));
    let mut author = vec![format!("{} {}", context.course_code, context.course_title)];
    if let Some(name) = &context.lecturer_name {
        author.push(name.clone());
    }
    out.push_str(&format!("\\author{{{}}}\n", escape(&author.join(" -- "))));
    out.push_str(&format!(
        "\\date{{{}}}\n\\begin{{document}}\n\\maketitle\n\n",
        context.generated_at.format("%-d %B %Y")
    ));
    if let Some(subtitle) = &doc.subtitle {
        out.push_str(&format!(
            "\\begin{{center}}\\emph{{{}}}\\end{{center}}\n\n",
            inline(subtitle)
        ));
    }
    if toc {
        out.push_str("\\tableofcontents\n\\newpage\n\n");
    }
    if !doc.meta.objectives.is_empty() {
        out.push_str("\\section*{Learning objectives}\n");
        itemize(
            doc.meta.objectives.iter().map(|o| inline(o)),
            false,
            &mut out,
        );
    }
    for block in &doc.blocks {
        block_tex(block, &mut out);
    }
    out.push_str("\\end{document}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::render::{sample_context, sample_doc};

    #[test]
    fn escapes_specials_and_keeps_math() {
        assert_eq!(escape("a_b & 50% #1 {x}"), "a\\_b \\& 50\\% \\#1 \\{x\\}");
        let tex = render_latex(&sample_doc(), &sample_context(), false);
        assert!(tex.starts_with("\\documentclass"));
        assert!(tex.contains("\\title{Heat <transfer> \\& conduction}"));
        assert!(tex.contains("\\[\nq = -k \\frac{dT}{dx} \\tag{1}\n\\]"));
        assert!(tex.contains("\\section{Conduction}"));
        assert!(tex.contains("\\textbf{Fourier's law}"));
        assert!(tex.trim_end().ends_with("\\end{document}"));
    }

    #[test]
    fn environments_balance() {
        let tex = render_latex(&sample_doc(), &sample_context(), true);
        assert_eq!(
            tex.matches("\\begin{").count(),
            tex.matches("\\end{").count()
        );
        assert!(tex.contains("\\tableofcontents"));
    }
}
