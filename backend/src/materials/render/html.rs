//! HTML renderer. Used for the in-app reading view and as the base for PDF.
//! Output is a standalone, accessible document (landmarks, one `h1`, table
//! captions and header scopes, figure text alternatives).

use async_trait::async_trait;

use super::{anchor, outline};
use crate::materials::{
    blocks::{parse_inline, Block, BlockKind, CalloutVariant, MaterialDoc, Origin},
    registry::{
        RenderContext, RenderError, RenderInput, RenderOptions, RenderedFile, Renderer, Template,
    },
};

pub struct HtmlRenderer;

#[async_trait]
impl Renderer for HtmlRenderer {
    fn id(&self) -> &'static str {
        "html"
    }
    fn label(&self) -> &'static str {
        "Web page"
    }
    fn content_type(&self) -> &'static str {
        "text/html; charset=utf-8"
    }
    fn extension(&self) -> &'static str {
        "html"
    }
    async fn render(&self, input: &RenderInput<'_>) -> Result<RenderedFile, RenderError> {
        Ok(RenderedFile {
            bytes: render_html(input.doc, input.context, input.template, input.options)
                .into_bytes(),
            content_type: "text/html; charset=utf-8",
            extension: "html",
        })
    }
}

pub fn escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

pub fn inline(text: &str) -> String {
    let mut out = String::new();
    for span in parse_inline(text) {
        let mut piece = escape(&span.text);
        if span.code {
            piece = format!("<code>{piece}</code>");
        }
        if span.italic {
            piece = format!("<em>{piece}</em>");
        }
        if span.bold {
            piece = format!("<strong>{piece}</strong>");
        }
        out.push_str(&piece);
    }
    out
}

fn style(template: &Template) -> String {
    format!(
        r#"
:root {{ --accent: {accent}; --ink: #1c1f26; --muted: #5b6270; --rule: #d9dde5; --tint: #f3f6fb; }}
@media (prefers-color-scheme: dark) {{ .screen :root, :root.screen {{ --ink: #e8eaf0; --muted: #a2a9b8; --rule: #343a46; --tint: #1b2029; }} }}
* {{ box-sizing: border-box; }}
html {{ font-size: {size}; }}
body {{ margin: 0; font-family: {font}; color: var(--ink); line-height: 1.55; background: #fff; }}
main {{ max-width: 46rem; margin: 0 auto; padding: 2.5rem 1.25rem 4rem; }}
h1, h2, h3, h4 {{ line-height: 1.25; margin: 1.8em 0 .5em; }}
h1 {{ font-size: 1.7rem; color: var(--accent); border-bottom: 2px solid var(--accent); padding-bottom: .2em; }}
h2 {{ font-size: 1.35rem; }} h3 {{ font-size: 1.15rem; }} h4 {{ font-size: 1rem; }}
.title-block h1 {{ margin-top: 0; font-size: 2rem; border: 0; }}
.subtitle, .meta {{ color: var(--muted); margin: .2em 0; }}
.objectives {{ background: var(--tint); border-left: 4px solid var(--accent); padding: .8rem 1.1rem; margin: 1.5rem 0; }}
.objectives h2 {{ margin: 0 0 .4em; font-size: 1.05rem; }}
.objectives ul {{ margin: 0; padding-left: 1.2rem; }}
p {{ margin: .7em 0; }}
.definition {{ margin: 1rem 0; padding-left: 1rem; border-left: 3px solid var(--accent); }}
.definition dt {{ font-weight: 700; }} .definition dd {{ margin: .15rem 0 0; }}
.key-result, .callout {{ background: var(--tint); border-radius: 6px; padding: .7rem 1rem; margin: 1rem 0; }}
.key-result {{ border-left: 4px solid var(--accent); }}
.callout .label {{ font-weight: 700; text-transform: uppercase; font-size: .72rem; letter-spacing: .05em; color: var(--accent); display: block; }}
.callout.misconception .label {{ color: #b3261e; }} .callout.warning .label {{ color: #8a5a00; }}
.example {{ border: 1px solid var(--rule); border-radius: 6px; padding: .8rem 1rem; margin: 1.1rem 0; }}
.example h4 {{ margin: 0 0 .4em; }} .example ol {{ margin: .4rem 0; padding-left: 1.3rem; }}
.equation {{ margin: 1rem 0; text-align: center; font-family: 'Cambria Math', 'Latin Modern Math', serif; display: grid; grid-template-columns: 3rem 1fr 3rem; align-items: center; }}
.equation .tex {{ grid-column: 2; overflow-x: auto; }} .equation .num {{ grid-column: 3; text-align: right; color: var(--muted); }}
.equation.plain .tex {{ font-family: ui-monospace, Menlo, Consolas, monospace; font-size: .92em; }}
.equation-caption {{ text-align: center; color: var(--muted); font-size: .9rem; margin-top: -.5rem; }}
figure {{ margin: 1.2rem 0; }} figcaption {{ color: var(--muted); font-size: .9rem; }}
figure ul {{ margin: .4rem 0; padding-left: 1.2rem; }}
table {{ border-collapse: collapse; width: 100%; margin: 1rem 0; font-size: .95rem; }}
caption {{ text-align: left; color: var(--muted); font-size: .9rem; padding-bottom: .3rem; }}
th, td {{ border: 1px solid var(--rule); padding: .4rem .6rem; text-align: left; vertical-align: top; }}
th {{ background: var(--tint); }}
.questions li {{ margin: .5rem 0; }} .questions details {{ margin-top: .25rem; color: var(--muted); }}
.badge {{ font-size: .7rem; font-weight: 600; border: 1px solid var(--rule); border-radius: 999px; padding: 0 .5em; color: var(--muted); margin-left: .4em; white-space: nowrap; }}
nav.toc {{ margin: 1.5rem 0 2rem; padding: 1rem 1.2rem; border: 1px solid var(--rule); border-radius: 6px; }}
nav.toc h2 {{ margin: 0 0 .5em; font-size: 1.1rem; }} nav.toc ol {{ list-style: none; padding-left: 0; margin: 0; }}
nav.toc li.l2 {{ padding-left: 1.2rem; }} nav.toc li.l3 {{ padding-left: 2.4rem; }} nav.toc a {{ color: inherit; text-decoration: none; }}
footer {{ margin-top: 3rem; padding-top: .8rem; border-top: 1px solid var(--rule); color: var(--muted); font-size: .85rem; }}
code {{ font-family: ui-monospace, Menlo, Consolas, monospace; font-size: .9em; background: var(--tint); padding: 0 .25em; border-radius: 3px; }}
@page {{ size: A4; margin: 18mm 16mm 20mm; }}
@media print {{ main {{ max-width: none; padding: 0; }} h1.chapter, h1 {{ break-after: avoid; }} .example, .key-result, figure, table, .equation, .definition {{ break-inside: avoid; }} nav.toc {{ break-after: page; }} .badge, details summary {{ display: none; }} details {{ display: block; }} }}
"#,
        accent = template.accent,
        size = template.base_size,
        font = template.font_stack,
    )
}

fn block_html(block: &Block, out: &mut String, katex: bool, chapter_breaks: bool) {
    let id = anchor(block);
    let badge = if block.origin == Origin::Supplemented {
        "<span class=\"badge\" title=\"Added by the AI beyond the lecture\">supplemented</span>"
    } else {
        ""
    };
    match &block.kind {
        BlockKind::Heading { level, text } => {
            let level = (*level).clamp(1, 4);
            let class = if level == 1 && chapter_breaks {
                " class=\"chapter\" style=\"break-before: page\""
            } else {
                ""
            };
            out.push_str(&format!(
                "<h{level} id=\"{id}\"{class}>{}</h{level}>\n",
                inline(text)
            ));
        }
        BlockKind::Paragraph { text } => {
            out.push_str(&format!("<p id=\"{id}\">{}{badge}</p>\n", inline(text)))
        }
        BlockKind::Definition { term, text } => out.push_str(&format!(
            "<dl class=\"definition\" id=\"{id}\"><dt>{}</dt><dd>{}{badge}</dd></dl>\n",
            inline(term),
            inline(text)
        )),
        BlockKind::KeyResult { title, text } => out.push_str(&format!(
            "<div class=\"key-result\" id=\"{id}\"><strong>{}</strong> {}{badge}</div>\n",
            inline(title.as_deref().unwrap_or("Key result")),
            inline(text)
        )),
        BlockKind::WorkedExample {
            title,
            problem,
            steps,
            answer,
        } => {
            out.push_str(&format!(
                "<section class=\"example\" id=\"{id}\"><h4>Worked example: {}{badge}</h4><p>{}</p><ol>",
                inline(title),
                inline(problem)
            ));
            for step in steps {
                out.push_str(&format!("<li>{}</li>", inline(step)));
            }
            out.push_str("</ol>");
            if let Some(answer) = answer {
                out.push_str(&format!(
                    "<p><strong>Answer:</strong> {}</p>",
                    inline(answer)
                ));
            }
            out.push_str("</section>\n");
        }
        BlockKind::Equation {
            latex,
            caption,
            number,
        } => {
            let tex = escape(latex);
            let (class, body) = if katex {
                ("equation", format!("\\[{tex}\\]"))
            } else {
                ("equation plain", tex)
            };
            let num = number.map(|n| format!("({n})")).unwrap_or_default();
            out.push_str(&format!(
                "<div class=\"{class}\" id=\"{id}\" role=\"math\" aria-label=\"{}\"><div class=\"tex\">{body}</div><div class=\"num\">{num}</div></div>\n",
                escape(latex)
            ));
            if let Some(caption) = caption {
                out.push_str(&format!(
                    "<p class=\"equation-caption\">{}</p>\n",
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
            out.push_str(&format!(
                "<figure id=\"{id}\" role=\"group\" aria-label=\"{}\">",
                escape(alt)
            ));
            match text_fallback {
                Some(text) if !text.trim().is_empty() => {
                    out.push_str("<ul>");
                    for line in text.lines().filter(|l| !l.trim().is_empty()) {
                        out.push_str(&format!(
                            "<li>{}</li>",
                            inline(line.trim_start_matches(['-', '*', ' ']))
                        ));
                    }
                    out.push_str("</ul>");
                }
                _ => out.push_str(&format!("<p>{}</p>", inline(alt))),
            }
            out.push_str(&format!(
                "<figcaption>{}</figcaption></figure>\n",
                inline(caption)
            ));
        }
        BlockKind::Table {
            caption,
            header,
            rows,
        } => {
            out.push_str(&format!("<table id=\"{id}\">"));
            if let Some(caption) = caption {
                out.push_str(&format!("<caption>{}</caption>", inline(caption)));
            }
            out.push_str("<thead><tr>");
            for cell in header {
                out.push_str(&format!("<th scope=\"col\">{}</th>", inline(cell)));
            }
            out.push_str("</tr></thead><tbody>");
            for row in rows {
                out.push_str("<tr>");
                for cell in row {
                    out.push_str(&format!("<td>{}</td>", inline(cell)));
                }
                out.push_str("</tr>");
            }
            out.push_str("</tbody></table>\n");
        }
        BlockKind::Callout { variant, text } => {
            let (class, label) = match variant {
                CalloutVariant::Note => ("note", "Note"),
                CalloutVariant::Tip => ("tip", "Tip"),
                CalloutVariant::Warning => ("warning", "Warning"),
                CalloutVariant::Misconception => ("misconception", "Common misconception"),
            };
            out.push_str(&format!(
                "<aside class=\"callout {class}\" id=\"{id}\"><span class=\"label\">{label}{badge}</span>{}</aside>\n",
                inline(text)
            ));
        }
        BlockKind::ReviewQuestions { questions } => {
            out.push_str(&format!(
                "<section id=\"{id}\"><h3>Review questions</h3><ol class=\"questions\">"
            ));
            for q in questions {
                out.push_str(&format!("<li>{}", inline(&q.question)));
                if let Some(answer) = &q.answer {
                    out.push_str(&format!(
                        "<details><summary>Answer</summary>{}</details>",
                        inline(answer)
                    ));
                }
                out.push_str("</li>");
            }
            out.push_str("</ol></section>\n");
        }
        BlockKind::FurtherReading { items } => {
            out.push_str(&format!(
                "<section id=\"{id}\"><h3>Further reading</h3><ul>"
            ));
            for item in items {
                out.push_str(&format!("<li>{}", inline(&item.title)));
                if let Some(note) = &item.note {
                    out.push_str(&format!(" &mdash; {}", inline(note)));
                }
                out.push_str("</li>");
            }
            out.push_str("</ul></section>\n");
        }
        BlockKind::Summary { points } => {
            out.push_str(&format!("<section id=\"{id}\"><h3>Summary</h3><ul>"));
            for point in points {
                out.push_str(&format!("<li>{}</li>", inline(point)));
            }
            out.push_str("</ul></section>\n");
        }
    }
}

/// A standalone HTML document.
pub fn render_html(
    doc: &MaterialDoc,
    context: &RenderContext,
    template: &Template,
    options: RenderOptions,
) -> String {
    let katex_base = std::env::var("MATERIALS_KATEX_URL")
        .ok()
        .filter(|v| !v.is_empty());
    let language = doc.meta.language.as_deref().unwrap_or("en");
    let lang_code = if language.eq_ignore_ascii_case("english") {
        "en"
    } else {
        language
    };
    let mut out = String::with_capacity(8 * 1024);
    out.push_str(&format!(
        "<!doctype html>\n<html lang=\"{}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>{}</title>",
        escape(lang_code),
        escape(&doc.title)
    ));
    if let Some(base) = &katex_base {
        let base = base.trim_end_matches('/');
        out.push_str(&format!(
            "<link rel=\"stylesheet\" href=\"{b}/katex.min.css\"><script defer src=\"{b}/katex.min.js\"></script><script defer src=\"{b}/contrib/auto-render.min.js\" onload=\"renderMathInElement(document.body,{{delimiters:[{{left:'\\\\[',right:'\\\\]',display:true}},{{left:'$',right:'$',display:false}}]}})\"></script>",
            b = escape(base)
        ));
    }
    out.push_str(&format!(
        "<style>{}</style></head><body><main>",
        style(template)
    ));

    out.push_str("<header class=\"title-block\">");
    out.push_str(&format!(
        "<p class=\"meta\">{} &middot; {}</p>",
        escape(&context.course_code),
        escape(&context.course_title)
    ));
    out.push_str(&format!("<h1>{}</h1>", escape(&doc.title)));
    if let Some(subtitle) = &doc.subtitle {
        out.push_str(&format!("<p class=\"subtitle\">{}</p>", escape(subtitle)));
    }
    let mut meta: Vec<String> = Vec::new();
    if let Some(name) = &context.lecturer_name {
        meta.push(escape(name));
    }
    if let Some(session) = &context.academic_session {
        meta.push(escape(session));
    }
    if let Some(semester) = &context.semester {
        meta.push(escape(semester));
    }
    if !meta.is_empty() {
        out.push_str(&format!(
            "<p class=\"meta\">{}</p>",
            meta.join(" &middot; ")
        ));
    }
    if doc.meta.ai_drafted {
        out.push_str(
            "<p class=\"meta\"><em>AI-drafted; not yet reviewed by the lecturer.</em></p>",
        );
    }
    out.push_str("</header>");

    if !doc.meta.objectives.is_empty() {
        out.push_str("<section class=\"objectives\"><h2>Learning objectives</h2><ul>");
        for objective in &doc.meta.objectives {
            out.push_str(&format!("<li>{}</li>", inline(objective)));
        }
        out.push_str("</ul></section>");
    }

    if options.toc {
        let entries = outline(doc);
        if !entries.is_empty() {
            out.push_str(
                "<nav class=\"toc\" aria-label=\"Table of contents\"><h2>Contents</h2><ol>",
            );
            for (level, text, target) in entries {
                out.push_str(&format!(
                    "<li class=\"l{level}\"><a href=\"#{target}\">{}</a></li>",
                    escape(&text)
                ));
            }
            out.push_str("</ol></nav>");
        }
    }

    for block in &doc.blocks {
        block_html(
            block,
            &mut out,
            katex_base.is_some(),
            options.chapter_breaks,
        );
    }

    let mut foot: Vec<String> = Vec::new();
    if let Some(footer) = template.footer {
        foot.push(escape(footer));
    }
    if let Some(version) = &context.version_label {
        foot.push(format!("Version {}", escape(version)));
    }
    foot.push(context.generated_at.format("%-d %B %Y").to_string());
    out.push_str(&format!(
        "<footer>{}</footer></main></body></html>",
        foot.join(" &middot; ")
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::render::{builtin_templates, sample_context, sample_doc};

    #[test]
    fn escapes_everything_user_supplied() {
        let doc = sample_doc();
        let html = render_html(
            &doc,
            &sample_context(),
            &builtin_templates()[0],
            RenderOptions::default(),
        );
        assert!(html.contains("Heat &lt;transfer&gt; &amp; conduction"));
        assert!(!html.contains("<transfer>"));
        assert!(html.contains("<strong>Fourier&#39;s law</strong>"));
        assert!(html.contains("<h1 id=\"b-h1\">Conduction</h1>"));
        assert!(html.contains("role=\"math\""));
        assert!(html.contains("<th scope=\"col\">Material</th>"));
        assert!(html.contains("supplemented"));
        assert!(html.contains("lang=\"en\""));
    }

    #[test]
    fn toc_and_chapter_breaks_for_books() {
        let doc = sample_doc();
        let html = render_html(
            &doc,
            &sample_context(),
            &builtin_templates()[1],
            RenderOptions {
                toc: true,
                chapter_breaks: true,
            },
        );
        assert!(html.contains("<nav class=\"toc\""));
        assert!(html.contains("href=\"#b-h1\""));
        assert!(html.contains("break-before: page"));
    }
}
