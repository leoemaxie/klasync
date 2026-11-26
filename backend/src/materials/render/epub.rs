//! EPUB 3 renderer: a hand-built zip with one XHTML chapter per level-1 section
//! (or one chapter for a lecture handout), a navigation document and metadata.
//! Block markup is shared with the HTML renderer, so the two stay in step.

use std::io::{Cursor, Write};

use async_trait::async_trait;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

use crate::materials::{
    blocks::{strip_inline, BlockKind, MaterialDoc},
    registry::{RenderContext, RenderError, RenderInput, RenderedFile, Renderer},
};

use super::html::{block_html, escape, inline};

pub struct EpubRenderer;

#[async_trait]
impl Renderer for EpubRenderer {
    fn id(&self) -> &'static str {
        "epub"
    }
    fn label(&self) -> &'static str {
        "EPUB"
    }
    fn content_type(&self) -> &'static str {
        "application/epub+zip"
    }
    fn extension(&self) -> &'static str {
        "epub"
    }
    async fn render(&self, input: &RenderInput<'_>) -> Result<RenderedFile, RenderError> {
        let bytes = build_epub(input.doc, input.context).map_err(RenderError::Failed)?;
        Ok(RenderedFile {
            bytes,
            content_type: "application/epub+zip",
            extension: "epub",
        })
    }
}

const CSS: &str = "body{font-family:serif;line-height:1.5;margin:1em}\
h1,h2,h3,h4{font-family:sans-serif;line-height:1.25}\
table{border-collapse:collapse;width:100%}th,td{border:1px solid #999;padding:.3em .5em;text-align:left}\
.tex,code{font-family:monospace}.callout,.card,blockquote{border-left:3px solid #888;padding:.2em .8em;margin:.8em 0}\
.badge{font-size:.75em;color:#666}";

fn xhtml(title: &str, body: &str, lang: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\" lang=\"{lang}\" xml:lang=\"{lang}\"><head><meta charset=\"utf-8\"/><title>{}</title><link rel=\"stylesheet\" type=\"text/css\" href=\"style.css\"/></head><body>{body}</body></html>",
        escape(title)
    )
}

/// Splits blocks into chapters at level-1 headings; everything before the
/// first heading (or a doc with none) forms the opening chapter.
fn chapters(doc: &MaterialDoc) -> Vec<(String, std::ops::Range<usize>)> {
    let mut starts: Vec<(String, usize)> = Vec::new();
    for (i, b) in doc.blocks.iter().enumerate() {
        if let BlockKind::Heading { level: 1, text } = &b.kind {
            starts.push((strip_inline(text), i));
        }
    }
    if starts.first().map(|(_, i)| *i != 0).unwrap_or(true) {
        starts.insert(0, (doc.title.clone(), 0));
    }
    let mut out = Vec::new();
    for (n, (title, start)) in starts.iter().enumerate() {
        let end = starts
            .get(n + 1)
            .map(|(_, i)| *i)
            .unwrap_or(doc.blocks.len());
        out.push((title.clone(), *start..end));
    }
    out
}

pub fn build_epub(doc: &MaterialDoc, context: &RenderContext) -> Result<Vec<u8>, String> {
    let lang = match doc.meta.language.as_deref() {
        None => "en",
        Some(l) if l.eq_ignore_ascii_case("english") => "en",
        Some(l) => l,
    };
    let chapter_list = chapters(doc);

    let mut files: Vec<(String, String)> = Vec::new();
    let mut title_page = format!("<h1>{}</h1>", escape(&doc.title));
    if let Some(subtitle) = &doc.subtitle {
        title_page.push_str(&format!("<p><em>{}</em></p>", escape(subtitle)));
    }
    title_page.push_str(&format!(
        "<p>{} &#183; {}</p>",
        escape(&context.course_code),
        escape(&context.course_title)
    ));
    if let Some(name) = &context.lecturer_name {
        title_page.push_str(&format!("<p>{}</p>", escape(name)));
    }
    if !doc.meta.objectives.is_empty() {
        title_page.push_str("<h2>Learning objectives</h2><ul>");
        for o in &doc.meta.objectives {
            title_page.push_str(&format!("<li>{}</li>", inline(o)));
        }
        title_page.push_str("</ul>");
    }
    files.push(("title.xhtml".into(), xhtml(&doc.title, &title_page, lang)));

    for (n, (title, range)) in chapter_list.iter().enumerate() {
        let mut body = String::new();
        for block in &doc.blocks[range.clone()] {
            block_html(block, &mut body, false, false);
        }
        files.push((format!("ch{}.xhtml", n + 1), xhtml(title, &body, lang)));
    }

    let uid = format!("urn:klasync:{}", context.generated_at.timestamp());
    let mut manifest = String::from(
        "<item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>\
<item id=\"css\" href=\"style.css\" media-type=\"text/css\"/>\
<item id=\"title\" href=\"title.xhtml\" media-type=\"application/xhtml+xml\"/>",
    );
    let mut spine = String::from("<itemref idref=\"title\"/>");
    let mut nav_items = String::from("<li><a href=\"title.xhtml\">Title</a></li>");
    for (n, (title, _)) in chapter_list.iter().enumerate() {
        let id = n + 1;
        let has_svg = files
            .iter()
            .any(|(name, body)| *name == format!("ch{id}.xhtml") && body.contains("<svg"));
        let props = if has_svg { " properties=\"svg\"" } else { "" };
        manifest.push_str(&format!(
            "<item id=\"ch{id}\" href=\"ch{id}.xhtml\" media-type=\"application/xhtml+xml\"{props}/>"
        ));
        spine.push_str(&format!("<itemref idref=\"ch{id}\"/>"));
        nav_items.push_str(&format!(
            "<li><a href=\"ch{id}.xhtml\">{}</a></li>",
            escape(title)
        ));
    }
    let opf = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\" unique-identifier=\"uid\" xml:lang=\"{lang}\"><metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:identifier id=\"uid\">{uid}</dc:identifier><dc:title>{}</dc:title><dc:language>{lang}</dc:language><dc:creator>{}</dc:creator><meta property=\"dcterms:modified\">{}</meta></metadata><manifest>{manifest}</manifest><spine>{spine}</spine></package>",
        escape(&doc.title),
        escape(context.lecturer_name.as_deref().unwrap_or("Klasync")),
        context.generated_at.format("%Y-%m-%dT%H:%M:%SZ"),
    );
    let nav = xhtml(
        "Contents",
        &format!("<nav epub:type=\"toc\"><h1>Contents</h1><ol>{nav_items}</ol></nav>"),
        lang,
    );

    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    // The mimetype entry must come first and be stored uncompressed.
    writer
        .start_file(
            "mimetype",
            SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
        )
        .map_err(|e| e.to_string())?;
    writer
        .write_all(b"application/epub+zip")
        .map_err(|e| e.to_string())?;
    let deflate = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut put = |name: &str, content: &str| -> Result<(), String> {
        writer
            .start_file(name, deflate)
            .map_err(|e| e.to_string())?;
        writer
            .write_all(content.as_bytes())
            .map_err(|e| e.to_string())
    };
    put(
        "META-INF/container.xml",
        "<?xml version=\"1.0\"?>\n<container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\"><rootfiles><rootfile full-path=\"OEBPS/content.opf\" media-type=\"application/oebps-package+xml\"/></rootfiles></container>",
    )?;
    put("OEBPS/content.opf", &opf)?;
    put("OEBPS/nav.xhtml", &nav)?;
    put("OEBPS/style.css", CSS)?;
    for (name, content) in &files {
        put(&format!("OEBPS/{name}"), content)?;
    }
    writer
        .finish()
        .map(|c| c.into_inner())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::render::{sample_context, sample_doc};
    use std::io::Read;

    fn part(bytes: &[u8], name: &str) -> String {
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).expect("valid zip");
        let mut f = zip.by_name(name).expect("part");
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();
        s
    }

    #[test]
    fn mimetype_is_first_and_stored() {
        let bytes = build_epub(&sample_doc(), &sample_context()).unwrap();
        let mut zip = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
        let first = zip.by_index(0).unwrap();
        assert_eq!(first.name().unwrap(), "mimetype");
        assert_eq!(first.compression(), CompressionMethod::Stored);
    }

    #[test]
    fn package_lists_every_chapter() {
        let bytes = build_epub(&sample_doc(), &sample_context()).unwrap();
        let opf = part(&bytes, "OEBPS/content.opf");
        assert!(opf.contains("Heat &lt;transfer&gt; &amp; conduction"));
        let chapters = opf.matches("<itemref").count();
        assert!(chapters >= 2, "title page plus at least one chapter");
        let nav = part(&bytes, "OEBPS/nav.xhtml");
        assert!(nav.contains("epub:type=\"toc\""));
        let ch1 = part(&bytes, "OEBPS/ch1.xhtml");
        assert!(ch1.contains("Thermal conductivity") || ch1.contains("conduction"));
    }
}
