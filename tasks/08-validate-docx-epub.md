# 08 - Validate DOCX and EPUB in real readers

**Why.** DOCX and EPUB are hand-built zips. They pass our structural tests but
were not opened in Word, LibreOffice or an EPUB reader/validator.

**Build.**
- Open sample exports (with equations, tables, a figure) in Word and
  LibreOffice; fix any repair prompts or layout problems.
- Run `epubcheck` on an export and fix findings (for example, `properties="svg"`
  and XHTML validity of shared HTML blocks).
- Add a CI step that runs `epubcheck` (and a LibreOffice headless conversion for
  DOCX) when available.

**Done when.** No repair prompts or epubcheck errors on the sample set, and the
checks run in CI or are documented as manual steps.

**Start at.** `backend/src/materials/render/docx.rs`, `epub.rs`.
