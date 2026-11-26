//! PDF renderer: HTML (print stylesheet) to PDF in a headless Chromium.
//!
//! The browser is heavy, so rendering is bounded by a semaphore
//! (`MATERIALS_PDF_CONCURRENCY`, default 2) and each run has a timeout. Set
//! `MATERIALS_PDF_BROWSER` to the browser binary; otherwise common install
//! locations are probed. Without a browser the format reports itself
//! unavailable and the rest of the system keeps working.

use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
    time::Duration,
};

use async_trait::async_trait;
use tokio::{process::Command, sync::Semaphore};
use uuid::Uuid;

use super::html::render_html;
use crate::materials::registry::{RenderError, RenderInput, RenderedFile, Renderer};

const RENDER_TIMEOUT: Duration = Duration::from_secs(90);

static PERMITS: OnceLock<Semaphore> = OnceLock::new();

fn permits() -> &'static Semaphore {
    PERMITS.get_or_init(|| {
        let n = std::env::var("MATERIALS_PDF_CONCURRENCY")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|n| *n > 0)
            .unwrap_or(2);
        Semaphore::new(n)
    })
}

pub struct PdfRenderer {
    browser: Option<PathBuf>,
}

impl PdfRenderer {
    pub fn new(browser: Option<PathBuf>) -> Self {
        Self { browser }
    }

    pub fn from_env() -> Self {
        Self::new(find_browser())
    }
}

fn find_browser() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("MATERIALS_PDF_BROWSER") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
        tracing::warn!(path = %path.display(), "MATERIALS_PDF_BROWSER does not exist; PDF export disabled");
        return None;
    }
    let fixed = [
        "/opt/pw-browsers/chromium",
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
        "/usr/bin/google-chrome",
        "/usr/bin/google-chrome-stable",
    ];
    if let Some(found) = fixed.iter().map(Path::new).find(|p| p.is_file()) {
        return Some(found.to_path_buf());
    }
    // Playwright installs under versioned directories.
    let root = Path::new("/opt/pw-browsers");
    for entry in std::fs::read_dir(root).ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("chromium") {
            for tail in ["chrome-linux/chrome", "chrome-linux64/chrome"] {
                let candidate = entry.path().join(tail);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

#[async_trait]
impl Renderer for PdfRenderer {
    fn id(&self) -> &'static str {
        "pdf"
    }
    fn label(&self) -> &'static str {
        "PDF"
    }
    fn content_type(&self) -> &'static str {
        "application/pdf"
    }
    fn extension(&self) -> &'static str {
        "pdf"
    }
    fn available(&self) -> bool {
        self.browser.is_some()
    }

    async fn render(&self, input: &RenderInput<'_>) -> Result<RenderedFile, RenderError> {
        let browser = self.browser.as_ref().ok_or_else(|| {
            RenderError::Unavailable(
                "no headless browser is configured (MATERIALS_PDF_BROWSER)".into(),
            )
        })?;
        let html = render_html(input.doc, input.context, input.template, input.options);
        let _permit = permits()
            .acquire()
            .await
            .map_err(|_| RenderError::Failed("the PDF queue is closed".into()))?;

        let work = std::env::temp_dir().join(format!("klasync-pdf-{}", Uuid::now_v7()));
        tokio::fs::create_dir_all(&work)
            .await
            .map_err(|e| RenderError::Failed(e.to_string()))?;
        let result = run_browser(browser, &work, &html).await;
        let _ = tokio::fs::remove_dir_all(&work).await;
        result.map(|bytes| RenderedFile {
            bytes,
            content_type: "application/pdf",
            extension: "pdf",
        })
    }
}

async fn run_browser(browser: &Path, work: &Path, html: &str) -> Result<Vec<u8>, RenderError> {
    let input = work.join("material.html");
    let output = work.join("material.pdf");
    tokio::fs::write(&input, html)
        .await
        .map_err(|e| RenderError::Failed(e.to_string()))?;

    let mut command = Command::new(browser);
    command
        .arg("--headless")
        .arg("--no-sandbox")
        .arg("--disable-gpu")
        .arg("--disable-dev-shm-usage")
        .arg("--no-pdf-header-footer")
        .arg("--export-tagged-pdf")
        .arg(format!(
            "--user-data-dir={}",
            work.join("profile").display()
        ))
        .arg(format!("--print-to-pdf={}", output.display()))
        .arg(format!("file://{}", input.display()))
        .kill_on_drop(true)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());

    let child = command
        .spawn()
        .map_err(|e| RenderError::Failed(format!("could not start the browser: {e}")))?;
    let finished = tokio::time::timeout(RENDER_TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| RenderError::Failed("the PDF render timed out".into()))?
        .map_err(|e| RenderError::Failed(e.to_string()))?;
    let bytes = tokio::fs::read(&output).await.map_err(|_| {
        let stderr = String::from_utf8_lossy(&finished.stderr);
        RenderError::Failed(format!(
            "the browser produced no PDF ({})",
            stderr.lines().last().unwrap_or("no output")
        ))
    })?;
    if !bytes.starts_with(b"%PDF") {
        return Err(RenderError::Failed(
            "the browser output is not a PDF".into(),
        ));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::{
        registry::RenderOptions,
        render::{builtin_templates, sample_context, sample_doc},
    };

    #[tokio::test]
    async fn unavailable_without_a_browser() {
        let renderer = PdfRenderer::new(None);
        assert!(!renderer.available());
        let doc = sample_doc();
        let context = sample_context();
        let templates = builtin_templates();
        let error = renderer
            .render(&RenderInput {
                doc: &doc,
                context: &context,
                template: &templates[0],
                options: RenderOptions::default(),
            })
            .await
            .unwrap_err();
        assert!(matches!(error, RenderError::Unavailable(_)));
    }

    /// Runs only where a browser is installed (this environment ships one).
    #[tokio::test]
    async fn renders_a_real_pdf_when_a_browser_exists() {
        let renderer = PdfRenderer::from_env();
        if !renderer.available() {
            eprintln!("no browser found; skipping");
            return;
        }
        let doc = sample_doc();
        let context = sample_context();
        let templates = builtin_templates();
        let file = renderer
            .render(&RenderInput {
                doc: &doc,
                context: &context,
                template: &templates[0],
                options: RenderOptions {
                    toc: true,
                    chapter_breaks: false,
                },
            })
            .await
            .expect("pdf renders");
        assert!(file.bytes.starts_with(b"%PDF"));
        assert!(file.bytes.len() > 1_000);
    }
}
