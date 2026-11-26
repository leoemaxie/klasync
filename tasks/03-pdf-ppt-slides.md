# 03 - PDF and legacy PPT slides

**Why.** Slides upload accepts `.pptx`, `.txt`, `.md` and pasted text only. Many
lecturers have PDFs, and the old `.ppt` is still common.

**Build.**
- PDF text extraction (a Rust crate such as `pdf-extract`/`lopdf`, or the
  headless browser already used for PDF export). Keep it off the async threads
  (`spawn_blocking`) and cap size and pages.
- `.ppt`: either extract text or return a clear "export to .pptx" message.
- Optional later: read slide images (OCR or a vision model) for diagrams.

**Done when.**
- Uploading a text PDF stores its text and the UI shows the character count.
- A scanned (image-only) PDF returns a clear "no text found" error.
- Unit tests with small fixture files; the 15 MB cap still applies.

**Start at.** `backend/src/materials/slides.rs`, `upload_slides` in
`backend/src/api/handlers/materials.rs`, `SlidesPanel.svelte`.
