//! DOCX renderer, built directly from the block tree (not converted from PDF),
//! so lecturers get a clean, editable Word file with real heading styles, a
//! table of contents field and native tables.

use std::io::{Cursor, Write};

use async_trait::async_trait;
use zip::{write::SimpleFileOptions, ZipWriter};

use super::outline;
use crate::materials::{
    blocks::{parse_inline, Block, BlockKind, CalloutVariant, MaterialDoc, Origin},
    registry::{
        RenderContext, RenderError, RenderInput, RenderOptions, RenderedFile, Renderer, Template,
    },
};

pub struct DocxRenderer;

const CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document";

#[async_trait]
impl Renderer for DocxRenderer {
    fn id(&self) -> &'static str {
        "docx"
    }
    fn label(&self) -> &'static str {
        "Word document"
    }
    fn content_type(&self) -> &'static str {
        CONTENT_TYPE
    }
    fn extension(&self) -> &'static str {
        "docx"
    }
    async fn render(&self, input: &RenderInput<'_>) -> Result<RenderedFile, RenderError> {
        let bytes = render_docx(input.doc, input.context, input.template, input.options)
            .map_err(RenderError::Failed)?;
        Ok(RenderedFile {
            bytes,
            content_type: CONTENT_TYPE,
            extension: "docx",
        })
    }
}

fn xml(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            // XML 1.0 forbids most control characters.
            c if (c as u32) < 0x20 && c != '\t' && c != '\n' => {}
            _ => out.push(c),
        }
    }
    out
}

fn run(text: &str, bold: bool, italic: bool, code: bool, extra: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut props = String::new();
    if bold {
        props.push_str("<w:b/>");
    }
    if italic {
        props.push_str("<w:i/>");
    }
    if code {
        props.push_str(
            "<w:rFonts w:ascii=\"Consolas\" w:hAnsi=\"Consolas\" w:cs=\"Consolas\"/><w:shd w:val=\"clear\" w:color=\"auto\" w:fill=\"EEF1F6\"/>",
        );
    }
    props.push_str(extra);
    format!(
        "<w:r><w:rPr>{props}</w:rPr><w:t xml:space=\"preserve\">{}</w:t></w:r>",
        xml(text)
    )
}

/// Inline Markdown into runs. `bold` forces every run bold (headers, labels).
fn runs(text: &str, force_bold: bool) -> String {
    parse_inline(text)
        .iter()
        .map(|s| run(&s.text, s.bold || force_bold, s.italic, s.code, ""))
        .collect()
}

fn para(style: Option<&str>, ppr_extra: &str, content: &str) -> String {
    let style = style
        .map(|s| format!("<w:pStyle w:val=\"{s}\"/>"))
        .unwrap_or_default();
    format!("<w:p><w:pPr>{style}{ppr_extra}</w:pPr>{content}</w:p>")
}

fn list_item(marker: &str, content: &str) -> String {
    para(
        Some("ListItem"),
        "",
        &format!(
            "{}{content}",
            run(&format!("{marker}\u{a0}"), false, false, false, "")
        ),
    )
}

fn table(header: &[String], rows: &[Vec<String>]) -> String {
    let mut out = String::from(
        "<w:tbl><w:tblPr><w:tblStyle w:val=\"TableGrid\"/><w:tblW w:w=\"5000\" w:type=\"pct\"/><w:tblBorders>",
    );
    for side in ["top", "left", "bottom", "right", "insideH", "insideV"] {
        out.push_str(&format!(
            "<w:{side} w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"C9CFDB\"/>"
        ));
    }
    out.push_str("</w:tblBorders></w:tblPr><w:tblGrid>");
    let width = (9000 / header.len().max(1)) as u32;
    for _ in header {
        out.push_str(&format!("<w:gridCol w:w=\"{width}\"/>"));
    }
    out.push_str("</w:tblGrid><w:tr><w:trPr><w:tblHeader/></w:trPr>");
    for cell in header {
        out.push_str(&format!(
            "<w:tc><w:tcPr><w:shd w:val=\"clear\" w:color=\"auto\" w:fill=\"EEF1F6\"/></w:tcPr>{}</w:tc>",
            para(Some("TableText"), "", &runs(cell, true))
        ));
    }
    out.push_str("</w:tr>");
    for row in rows {
        out.push_str("<w:tr>");
        for cell in row {
            out.push_str(&format!(
                "<w:tc>{}</w:tc>",
                para(Some("TableText"), "", &runs(cell, false))
            ));
        }
        out.push_str("</w:tr>");
    }
    out.push_str("</w:tbl>");
    out.push_str(&para(None, "", ""));
    out
}

fn block_xml(block: &Block, out: &mut String, options: RenderOptions) {
    let tag = if block.origin == Origin::Supplemented {
        run(
            " (supplemented)",
            false,
            true,
            false,
            "<w:color w:val=\"6B7280\"/>",
        )
    } else {
        String::new()
    };
    match &block.kind {
        BlockKind::Heading { level, text } => {
            let level = (*level).clamp(1, 4);
            let brk = if level == 1 && options.chapter_breaks {
                "<w:pageBreakBefore/>"
            } else {
                ""
            };
            out.push_str(&para(
                Some(&format!("Heading{level}")),
                brk,
                &runs(text, false),
            ));
        }
        BlockKind::Paragraph { text } => {
            out.push_str(&para(None, "", &format!("{}{tag}", runs(text, false))))
        }
        BlockKind::Definition { term, text } => out.push_str(&para(
            Some("Definition"),
            "",
            &format!(
                "{}{}{tag}",
                runs(term, true),
                run(": ", true, false, false, "") + &runs(text, false)
            ),
        )),
        BlockKind::KeyResult { title, text } => out.push_str(&para(
            Some("Callout"),
            "",
            &format!(
                "{}{}{tag}",
                run(
                    &format!("{}. ", title.as_deref().unwrap_or("Key result")),
                    true,
                    false,
                    false,
                    ""
                ),
                runs(text, false)
            ),
        )),
        BlockKind::WorkedExample {
            title,
            problem,
            steps,
            answer,
        } => {
            out.push_str(&para(
                Some("ExampleTitle"),
                "",
                &format!(
                    "{}{tag}",
                    run(&format!("Worked example: {title}"), true, false, false, "")
                ),
            ));
            out.push_str(&para(None, "", &runs(problem, false)));
            for (i, step) in steps.iter().enumerate() {
                out.push_str(&list_item(&format!("{}.", i + 1), &runs(step, false)));
            }
            if let Some(answer) = answer {
                out.push_str(&para(
                    None,
                    "",
                    &format!(
                        "{}{}",
                        run("Answer: ", true, false, false, ""),
                        runs(answer, false)
                    ),
                ));
            }
        }
        BlockKind::Equation {
            latex,
            caption,
            number,
        } => {
            let num = number.map(|n| format!("    ({n})")).unwrap_or_default();
            out.push_str(&para(
                Some("Equation"),
                "",
                &run(&format!("{latex}{num}"), false, false, false, ""),
            ));
            if let Some(caption) = caption {
                out.push_str(&para(
                    Some("Caption"),
                    "<w:jc w:val=\"center\"/>",
                    &runs(caption, false),
                ));
            }
        }
        BlockKind::Figure {
            caption,
            alt,
            text_fallback,
            ..
        } => {
            match text_fallback {
                Some(text) if !text.trim().is_empty() => {
                    for line in text.lines().filter(|l| !l.trim().is_empty()) {
                        out.push_str(&list_item(
                            "\u{2022}",
                            &runs(line.trim_start_matches(['-', '*', ' ']), false),
                        ));
                    }
                }
                _ => out.push_str(&para(None, "", &runs(alt, false))),
            }
            out.push_str(&para(
                Some("Caption"),
                "",
                &runs(&format!("Figure. {caption}"), false),
            ));
        }
        BlockKind::Table {
            caption,
            header,
            rows,
        } => {
            if let Some(caption) = caption {
                out.push_str(&para(Some("Caption"), "", &runs(caption, false)));
            }
            out.push_str(&table(header, rows));
        }
        BlockKind::Callout { variant, text } => {
            let label = match variant {
                CalloutVariant::Note => "Note",
                CalloutVariant::Tip => "Tip",
                CalloutVariant::Warning => "Warning",
                CalloutVariant::Misconception => "Common misconception",
            };
            out.push_str(&para(
                Some("Callout"),
                "",
                &format!(
                    "{}{}{tag}",
                    run(&format!("{label}. "), true, false, false, ""),
                    runs(text, false)
                ),
            ));
        }
        BlockKind::ReviewQuestions { questions } => {
            out.push_str(&para(
                Some("Heading3"),
                "",
                &run("Review questions", false, false, false, ""),
            ));
            for (i, q) in questions.iter().enumerate() {
                out.push_str(&list_item(
                    &format!("{}.", i + 1),
                    &runs(&q.question, false),
                ));
                if let Some(answer) = &q.answer {
                    out.push_str(&para(
                        Some("ListItem"),
                        "<w:ind w:left=\"1080\"/>",
                        &format!(
                            "{}{}",
                            run("Answer: ", false, true, false, ""),
                            runs(answer, false)
                        ),
                    ));
                }
            }
        }
        BlockKind::FurtherReading { items } => {
            out.push_str(&para(
                Some("Heading3"),
                "",
                &run("Further reading", false, false, false, ""),
            ));
            for item in items {
                let note = item
                    .note
                    .as_deref()
                    .map(|n| format!(" \u{2014} {}", runs(n, false)))
                    .unwrap_or_default();
                // `note` is already run markup; keep the title and note separate.
                let mut content = runs(&item.title, false);
                if item.note.is_some() {
                    content.push_str(&run(" \u{2014} ", false, false, false, ""));
                    content.push_str(&runs(item.note.as_deref().unwrap_or_default(), false));
                }
                let _ = note;
                out.push_str(&list_item("\u{2022}", &content));
            }
        }
        BlockKind::Summary { points } => {
            out.push_str(&para(
                Some("Heading3"),
                "",
                &run("Summary", false, false, false, ""),
            ));
            for point in points {
                out.push_str(&list_item("\u{2022}", &runs(point, false)));
            }
        }
    }
}

fn toc_xml(doc: &MaterialDoc) -> String {
    let entries = outline(doc);
    if entries.is_empty() {
        return String::new();
    }
    let mut out = para(
        Some("TocHeading"),
        "",
        &run("Contents", false, false, false, ""),
    );
    let last = entries.len() - 1;
    for (i, (level, text, _)) in entries.iter().enumerate() {
        let mut content = String::new();
        if i == 0 {
            content.push_str(
                "<w:r><w:fldChar w:fldCharType=\"begin\"/></w:r><w:r><w:instrText xml:space=\"preserve\"> TOC \\o \"1-3\" \\h \\z \\u </w:instrText></w:r><w:r><w:fldChar w:fldCharType=\"separate\"/></w:r>",
            );
        }
        content.push_str(&run(text, false, false, false, ""));
        if i == last {
            content.push_str("<w:r><w:fldChar w:fldCharType=\"end\"/></w:r>");
        }
        out.push_str(&para(
            Some("TocEntry"),
            &format!("<w:ind w:left=\"{}\"/>", (*level as u32 - 1) * 360),
            &content,
        ));
    }
    out.push_str(&para(None, "<w:pageBreakBefore/>", ""));
    out
}

fn document_xml(doc: &MaterialDoc, context: &RenderContext, options: RenderOptions) -> String {
    let mut body = String::new();
    body.push_str(&para(
        Some("Meta"),
        "",
        &run(
            &format!("{} \u{b7} {}", context.course_code, context.course_title),
            false,
            false,
            false,
            "",
        ),
    ));
    body.push_str(&para(
        Some("Title"),
        "",
        &run(&doc.title, false, false, false, ""),
    ));
    if let Some(subtitle) = &doc.subtitle {
        body.push_str(&para(
            Some("Subtitle"),
            "",
            &run(subtitle, false, false, false, ""),
        ));
    }
    let mut meta: Vec<String> = Vec::new();
    if let Some(name) = &context.lecturer_name {
        meta.push(name.clone());
    }
    if let Some(session) = &context.academic_session {
        meta.push(session.clone());
    }
    if let Some(semester) = &context.semester {
        meta.push(semester.clone());
    }
    if !meta.is_empty() {
        body.push_str(&para(
            Some("Meta"),
            "",
            &run(&meta.join(" \u{b7} "), false, false, false, ""),
        ));
    }
    if doc.meta.ai_drafted {
        body.push_str(&para(
            Some("Meta"),
            "",
            &run(
                "AI-drafted; not yet reviewed by the lecturer.",
                false,
                true,
                false,
                "",
            ),
        ));
    }
    if !doc.meta.objectives.is_empty() {
        body.push_str(&para(
            Some("Heading3"),
            "",
            &run("Learning objectives", false, false, false, ""),
        ));
        for objective in &doc.meta.objectives {
            body.push_str(&list_item("\u{2022}", &runs(objective, false)));
        }
    }
    if options.toc {
        body.push_str(&toc_xml(doc));
    }
    for block in &doc.blocks {
        block_xml(block, &mut body, options);
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><w:body>{body}<w:sectPr><w:footerReference w:type=\"default\" r:id=\"rId2\"/><w:pgSz w:w=\"11906\" w:h=\"16838\"/><w:pgMar w:top=\"1134\" w:right=\"1134\" w:bottom=\"1134\" w:left=\"1134\" w:header=\"567\" w:footer=\"567\" w:gutter=\"0\"/></w:sectPr></w:body></w:document>"
    )
}

fn styles_xml(template: &Template) -> String {
    let accent = template.accent.trim_start_matches('#');
    let serif = template.font_stack.contains("serif") || template.font_stack.contains("Georgia");
    let font = if serif { "Cambria" } else { "Calibri" };
    let half_points: u32 = match template.base_size {
        "10pt" => 20,
        "11.5pt" => 23,
        _ => 22,
    };
    let heading = |id: &str, name: &str, size: u32, color: &str, before: u32| {
        format!(
            "<w:style w:type=\"paragraph\" w:styleId=\"{id}\"><w:name w:val=\"{name}\"/><w:basedOn w:val=\"Normal\"/><w:next w:val=\"Normal\"/><w:qFormat/><w:pPr><w:keepNext/><w:spacing w:before=\"{before}\" w:after=\"80\"/><w:outlineLvl w:val=\"{}\"/></w:pPr><w:rPr><w:b/><w:color w:val=\"{color}\"/><w:sz w:val=\"{size}\"/></w:rPr></w:style>",
            id.trim_start_matches("Heading").parse::<u32>().unwrap_or(1) - 1
        )
    };
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii=\"{font}\" w:hAnsi=\"{font}\" w:cs=\"{font}\" w:eastAsia=\"{font}\"/><w:sz w:val=\"{half_points}\"/><w:szCs w:val=\"{half_points}\"/><w:lang w:val=\"en-GB\"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after=\"120\" w:line=\"300\" w:lineRule=\"auto\"/></w:pPr></w:pPrDefault></w:docDefaults>\
<w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:name w:val=\"Normal\"/><w:qFormat/></w:style>\
<w:style w:type=\"paragraph\" w:styleId=\"Title\"><w:name w:val=\"Title\"/><w:basedOn w:val=\"Normal\"/><w:qFormat/><w:pPr><w:spacing w:before=\"120\" w:after=\"120\"/></w:pPr><w:rPr><w:b/><w:color w:val=\"{accent}\"/><w:sz w:val=\"52\"/></w:rPr></w:style>\
<w:style w:type=\"paragraph\" w:styleId=\"Subtitle\"><w:name w:val=\"Subtitle\"/><w:basedOn w:val=\"Normal\"/><w:qFormat/><w:rPr><w:color w:val=\"5B6270\"/><w:sz w:val=\"28\"/></w:rPr></w:style>\
<w:style w:type=\"paragraph\" w:styleId=\"Meta\"><w:name w:val=\"Meta\"/><w:basedOn w:val=\"Normal\"/><w:pPr><w:spacing w:after=\"40\"/></w:pPr><w:rPr><w:color w:val=\"5B6270\"/></w:rPr></w:style>\
{h1}{h2}{h3}{h4}\
<w:style w:type=\"paragraph\" w:styleId=\"Definition\"><w:name w:val=\"Definition\"/><w:basedOn w:val=\"Normal\"/><w:pPr><w:pBdr><w:left w:val=\"single\" w:sz=\"18\" w:space=\"8\" w:color=\"{accent}\"/></w:pBdr><w:ind w:left=\"240\"/></w:pPr></w:style>\
<w:style w:type=\"paragraph\" w:styleId=\"Callout\"><w:name w:val=\"Callout\"/><w:basedOn w:val=\"Normal\"/><w:pPr><w:pBdr><w:left w:val=\"single\" w:sz=\"24\" w:space=\"8\" w:color=\"{accent}\"/></w:pBdr><w:shd w:val=\"clear\" w:color=\"auto\" w:fill=\"F3F6FB\"/><w:ind w:left=\"240\"/></w:pPr></w:style>\
<w:style w:type=\"paragraph\" w:styleId=\"ExampleTitle\"><w:name w:val=\"Example Title\"/><w:basedOn w:val=\"Normal\"/><w:pPr><w:keepNext/><w:spacing w:before=\"200\"/></w:pPr><w:rPr><w:color w:val=\"{accent}\"/></w:rPr></w:style>\
<w:style w:type=\"paragraph\" w:styleId=\"Equation\"><w:name w:val=\"Equation\"/><w:basedOn w:val=\"Normal\"/><w:pPr><w:jc w:val=\"center\"/><w:spacing w:before=\"120\" w:after=\"120\"/></w:pPr><w:rPr><w:rFonts w:ascii=\"Cambria Math\" w:hAnsi=\"Cambria Math\"/></w:rPr></w:style>\
<w:style w:type=\"paragraph\" w:styleId=\"Caption\"><w:name w:val=\"caption\"/><w:basedOn w:val=\"Normal\"/><w:qFormat/><w:rPr><w:i/><w:color w:val=\"5B6270\"/><w:sz w:val=\"20\"/></w:rPr></w:style>\
<w:style w:type=\"paragraph\" w:styleId=\"ListItem\"><w:name w:val=\"List Item\"/><w:basedOn w:val=\"Normal\"/><w:pPr><w:spacing w:after=\"60\"/><w:ind w:left=\"720\" w:hanging=\"360\"/></w:pPr></w:style>\
<w:style w:type=\"paragraph\" w:styleId=\"TableText\"><w:name w:val=\"Table Text\"/><w:basedOn w:val=\"Normal\"/><w:pPr><w:spacing w:after=\"0\"/></w:pPr><w:rPr><w:sz w:val=\"20\"/></w:rPr></w:style>\
<w:style w:type=\"paragraph\" w:styleId=\"TocHeading\"><w:name w:val=\"TOC Heading\"/><w:basedOn w:val=\"Normal\"/><w:pPr><w:spacing w:before=\"240\"/></w:pPr><w:rPr><w:b/><w:sz w:val=\"32\"/><w:color w:val=\"{accent}\"/></w:rPr></w:style>\
<w:style w:type=\"paragraph\" w:styleId=\"TocEntry\"><w:name w:val=\"toc 1\"/><w:basedOn w:val=\"Normal\"/><w:pPr><w:spacing w:after=\"40\"/></w:pPr></w:style>\
<w:style w:type=\"paragraph\" w:styleId=\"Footer\"><w:name w:val=\"footer\"/><w:basedOn w:val=\"Normal\"/><w:rPr><w:color w:val=\"5B6270\"/><w:sz w:val=\"18\"/></w:rPr></w:style>\
<w:style w:type=\"table\" w:default=\"1\" w:styleId=\"TableNormal\"><w:name w:val=\"Normal Table\"/><w:tblPr><w:tblInd w:w=\"0\" w:type=\"dxa\"/><w:tblCellMar><w:top w:w=\"0\" w:type=\"dxa\"/><w:left w:w=\"108\" w:type=\"dxa\"/><w:bottom w:w=\"0\" w:type=\"dxa\"/><w:right w:w=\"108\" w:type=\"dxa\"/></w:tblCellMar></w:tblPr></w:style>\
<w:style w:type=\"table\" w:styleId=\"TableGrid\"><w:name w:val=\"Table Grid\"/><w:basedOn w:val=\"TableNormal\"/></w:style>\
</w:styles>",
        h1 = heading("Heading1", "heading 1", 36, accent, 360),
        h2 = heading("Heading2", "heading 2", 30, "1C1F26", 280),
        h3 = heading("Heading3", "heading 3", 26, "1C1F26", 200),
        h4 = heading("Heading4", "heading 4", 22, "1C1F26", 160),
    )
}

fn footer_xml(template: &Template, context: &RenderContext) -> String {
    let label = template.footer.unwrap_or("");
    let text = if label.is_empty() {
        context.heading.clone()
    } else {
        format!("{label} \u{b7} {}", context.heading)
    };
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:ftr xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:p><w:pPr><w:pStyle w:val=\"Footer\"/><w:jc w:val=\"center\"/></w:pPr>{}<w:fldSimple w:instr=\" PAGE \"><w:r><w:t>1</w:t></w:r></w:fldSimple></w:p></w:ftr>",
        run(&format!("{text} \u{b7} Page "), false, false, false, "")
    )
}

/// Builds the `.docx` package in memory.
pub fn render_docx(
    doc: &MaterialDoc,
    context: &RenderContext,
    template: &Template,
    options: RenderOptions,
) -> Result<Vec<u8>, String> {
    let parts: Vec<(&str, String)> = vec![
        (
            "[Content_Types].xml",
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/><Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/><Override PartName=\"/word/settings.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml\"/><Override PartName=\"/word/footer1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml\"/><Override PartName=\"/docProps/core.xml\" ContentType=\"application/vnd.openxmlformats-package.core-properties+xml\"/></Types>".to_owned(),
        ),
        (
            "_rels/.rels",
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties\" Target=\"docProps/core.xml\"/></Relationships>".to_owned(),
        ),
        (
            "word/_rels/document.xml.rels",
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer\" Target=\"footer1.xml\"/><Relationship Id=\"rId3\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings\" Target=\"settings.xml\"/></Relationships>".to_owned(),
        ),
        (
            "word/settings.xml",
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:settings xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:updateFields w:val=\"true\"/></w:settings>".to_owned(),
        ),
        ("word/styles.xml", styles_xml(template)),
        ("word/footer1.xml", footer_xml(template, context)),
        ("word/document.xml", document_xml(doc, context, options)),
        (
            "docProps/core.xml",
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><cp:coreProperties xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:dcterms=\"http://purl.org/dc/terms/\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"><dc:title>{}</dc:title><dc:creator>{}</dc:creator><dcterms:created xsi:type=\"dcterms:W3CDTF\">{}</dcterms:created></cp:coreProperties>",
                xml(&doc.title),
                xml(context.lecturer_name.as_deref().unwrap_or("Klasync")),
                context.generated_at.format("%Y-%m-%dT%H:%M:%SZ")
            ),
        ),
    ];

    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let file_options =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, content) in parts {
        writer
            .start_file(name, file_options)
            .map_err(|error| error.to_string())?;
        writer
            .write_all(content.as_bytes())
            .map_err(|error| error.to_string())?;
    }
    writer
        .finish()
        .map(|cursor| cursor.into_inner())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::render::{builtin_templates, sample_context, sample_doc};
    use std::io::Read;

    fn read_part(bytes: &[u8], name: &str) -> String {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("valid zip");
        let mut file = archive.by_name(name).expect("part present");
        let mut out = String::new();
        file.read_to_string(&mut out).unwrap();
        out
    }

    /// Tags must nest: a cheap well-formedness check without an XML crate.
    fn assert_balanced(xml: &str) {
        let mut stack: Vec<String> = Vec::new();
        let mut rest = xml;
        while let Some(start) = rest.find('<') {
            let end = rest[start..].find('>').expect("tag closes") + start;
            let tag = &rest[start + 1..end];
            rest = &rest[end + 1..];
            if tag.starts_with('?') || tag.ends_with('/') {
                continue;
            }
            if let Some(name) = tag.strip_prefix('/') {
                assert_eq!(stack.pop().as_deref(), Some(name), "mismatched </{name}>");
            } else {
                stack.push(tag.split_whitespace().next().unwrap().to_owned());
            }
        }
        assert!(stack.is_empty(), "unclosed: {stack:?}");
    }

    #[test]
    fn package_has_all_parts_and_balanced_xml() {
        let bytes = render_docx(
            &sample_doc(),
            &sample_context(),
            &builtin_templates()[0],
            RenderOptions {
                toc: true,
                chapter_breaks: true,
            },
        )
        .unwrap();
        assert_eq!(&bytes[..2], b"PK");
        for part in [
            "[Content_Types].xml",
            "_rels/.rels",
            "word/_rels/document.xml.rels",
            "word/document.xml",
            "word/styles.xml",
            "word/settings.xml",
            "word/footer1.xml",
            "docProps/core.xml",
        ] {
            assert_balanced(&read_part(&bytes, part));
        }
        let document = read_part(&bytes, "word/document.xml");
        assert!(document.contains("Heat &lt;transfer&gt; &amp; conduction"));
        assert!(document.contains("w:val=\"Heading1\""));
        assert!(document.contains("<w:tbl>"));
        assert!(document.contains("TOC \\o"));
        assert!(document.contains("pageBreakBefore"));
        assert!(document.contains("q = -k \\frac{dT}{dx}    (1)"));
    }

    #[test]
    fn control_characters_never_reach_the_xml() {
        assert_eq!(xml("a\u{1}b<"), "ab&lt;");
    }
}
