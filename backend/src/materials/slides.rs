//! Lecturer slides as generation context.
//!
//! The model drafts from the lecture transcript; slides add the lecturer's own
//! terminology, structure and anything said too quietly to transcribe. Slides
//! arrive as plain text (paste or `.txt`/`.md`) or as a `.pptx`, whose text we
//! read straight from the OOXML (a zip of XML) with no extra dependency. PDF
//! and the legacy `.ppt` are not supported.

use std::io::{Cursor, Read};

/// Longest slide text kept (characters); the prompts use a smaller window.
pub const MAX_SLIDES_CHARS: usize = 60_000;
/// Largest upload accepted.
pub const MAX_UPLOAD_BYTES: usize = 15 * 1024 * 1024;

/// `<a:t>` text of one slide, paragraphs separated by newlines.
fn slide_text(xml: &str) -> String {
    let mut out = String::new();
    let mut rest = xml;
    while let Some(start) = rest.find('<') {
        rest = &rest[start..];
        if rest.starts_with("<a:t>") || rest.starts_with("<a:t ") {
            let open_end = match rest.find('>') {
                Some(i) => i + 1,
                None => break,
            };
            let close = rest[open_end..].find("</a:t>").map(|i| i + open_end);
            let Some(close) = close else { break };
            out.push_str(&unescape(&rest[open_end..close]));
            rest = &rest[close + 6..];
        } else if rest.starts_with("</a:p>") {
            if !out.ends_with('\n') && !out.is_empty() {
                out.push('\n');
            }
            rest = &rest[6..];
        } else {
            rest = &rest[1..];
        }
    }
    out.trim().to_owned()
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// Text of every slide, in slide order, as `Slide N:` blocks.
pub fn extract_pptx(bytes: &[u8]) -> Result<String, String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| "that file is not a .pptx presentation".to_owned())?;
    let mut slides: Vec<(u32, String)> = Vec::new();
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).map_err(|e| e.to_string())?;
        let Ok(name) = file.name().map(|n| n.to_owned()) else {
            continue;
        };
        let Some(number) = name
            .strip_prefix("ppt/slides/slide")
            .and_then(|n| n.strip_suffix(".xml"))
            .and_then(|n| n.parse::<u32>().ok())
        else {
            continue;
        };
        // Guard against zip bombs: a slide's XML is never this large.
        if file.size() > 8 * 1024 * 1024 {
            return Err("a slide in that file is too large".to_owned());
        }
        let mut xml = String::new();
        file.read_to_string(&mut xml).map_err(|e| e.to_string())?;
        let text = slide_text(&xml);
        if !text.is_empty() {
            slides.push((number, text));
        }
    }
    if slides.is_empty() {
        return Err("no text was found in that presentation".to_owned());
    }
    slides.sort_by_key(|(n, _)| *n);
    Ok(slides
        .into_iter()
        .map(|(n, t)| format!("Slide {n}:\n{t}"))
        .collect::<Vec<_>>()
        .join("\n\n"))
}

/// Normalises pasted or uploaded text: trims, drops control characters and
/// enforces the size cap.
pub fn clean_text(raw: &str) -> Result<String, String> {
    let text: String = raw
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
        .collect();
    let text = text.trim().to_owned();
    if text.chars().count() > MAX_SLIDES_CHARS {
        return Err(format!(
            "slides can be at most {MAX_SLIDES_CHARS} characters; shorten them or paste the key slides"
        ));
    }
    Ok(text)
}

/// The slice of slides given to a prompt.
pub fn prompt_window(slides: &str, max_chars: usize) -> String {
    slides.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn pptx(slides: &[(&str, &str)]) -> Vec<u8> {
        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, xml) in slides {
            w.start_file(*name, SimpleFileOptions::default()).unwrap();
            w.write_all(xml.as_bytes()).unwrap();
        }
        w.finish().unwrap().into_inner()
    }

    #[test]
    fn reads_slides_in_order_with_entities() {
        let s2 = "<p:sld><a:p><a:r><a:t>Second &amp; last</a:t></a:r></a:p></p:sld>";
        let s10 = "<p:sld><a:p><a:r><a:t>Tenth</a:t></a:r></a:p></p:sld>";
        let s1 = "<p:sld><a:p><a:r><a:t>Title</a:t></a:r></a:p><a:p><a:r><a:t>Bullet</a:t></a:r></a:p></p:sld>";
        let bytes = pptx(&[
            ("ppt/slides/slide10.xml", s10),
            ("ppt/slides/slide2.xml", s2),
            ("ppt/slides/slide1.xml", s1),
            ("ppt/slides/_rels/slide1.xml.rels", "ignored"),
        ]);
        let text = extract_pptx(&bytes).unwrap();
        assert_eq!(
            text,
            "Slide 1:\nTitle\nBullet\n\nSlide 2:\nSecond & last\n\nSlide 10:\nTenth"
        );
    }

    #[test]
    fn rejects_non_presentations_and_empty_ones() {
        assert!(extract_pptx(b"not a zip").is_err());
        let empty = pptx(&[("ppt/slides/slide1.xml", "<p:sld></p:sld>")]);
        assert!(extract_pptx(&empty).is_err());
    }

    #[test]
    fn clean_text_strips_controls_and_caps_size() {
        assert_eq!(clean_text("  a\u{0}b\nc  ").unwrap(), "ab\nc");
        assert!(clean_text(&"x".repeat(MAX_SLIDES_CHARS + 1)).is_err());
    }
}
