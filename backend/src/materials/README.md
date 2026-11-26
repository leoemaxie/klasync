# Lecture materials engine

After each lecture Klasync drafts a structured lecture material for the
lecturer, who reviews, edits and publishes it. Students read it in the app or
export it (PDF, DOCX, Markdown, HTML). A course offering's materials compose
into a **course book**; the same data powers **email delivery** and
**cross-offering comparison**. Design: the "Lecture Materials: Design Spec"
doc. This file is the map for whoever builds on top.

## Status

Built and tested end to end: schema, block document, plug-in registry, the
generation pipeline, review/edit/publish, six renderers, course book, email
and passive delivery channels, comparison analyzer, event outbox, HTTP API,
post-lecture trigger, typed TypeScript client (`src/lib/api/materials.ts`).

**Frontend** (`src/lib/components/materials/`, `src/lib/materials/`):
lecturer *Materials* tab (every lecture of the course, review/edit with the
transcript side-by-side, autosave with `base_rev` conflict handling,
per-section rewrite, lock, publish with optional email, export, versions,
deliveries), course book builder, offering comparison, course settings and
per-course plug-in toggles; student *Handout* tool in the lecture studio and a
*Course book* viewer on each course card. Equations and inline `$math$` render
with KaTeX (lazy-loaded); concept-map figures are drawn as SVG. Pure edit and
validation logic is in `src/lib/materials/doc.ts` (mirrors `blocks.rs`) and is
unit-tested.

**Also built:**

- **Plug-in settings API** (`GET /courses/{id}/materials/plugins`,
  `PUT|DELETE .../plugins/{kind}/{plugin_id}`). A course can turn a renderer
  off: export then refuses it and students no longer see it. `in_app` delivery
  and the `html` renderer are protected.
- **EPUB 3 and LaTeX exports** (`render/epub.rs`, `render/latex.rs`).
- **Signed export URLs**: `?mode=url` on every export route answers
  `{url, filename, expires_in}` (5 min) when the storage backend can sign
  (R2/S3); `url: null` means stream the bytes. The web client uses it and falls
  back to streaming.
- **Email attachments**: `EmailMessage.attachments`; lecture emails carry the
  PDF (DOCX without a browser) up to 8 MB. Books stay link-only.
- **Concept-map graphics**: `Figure.diagram` (nodes/edges) is drawn as SVG in
  HTML, PDF and EPUB (`render/svg.rs`), and as PNG in DOCX. Markdown and LaTeX keep the text outline.
- **Separate worker process**: `KLASYNC_ROLE=worker` runs only the job loops
  (no HTTP), `api` only HTTP, `all` (default) both.
- **Drafts may be incomplete**: saving a draft only checks schema and size;
  the full validation runs on publish, so a new empty block never loses edits.
- `GET .../materials/transcript` (lecturer transcript for review) and the
  course materials list now returns every lecture, with `has_material`.
- **Provider batch mode** (`MATERIALS_BATCH=1`): section jobs go through the
  provider batch API (about half price, minutes to hours of latency). Answers
  are validated and stored through the same path as live ones; anything
  unusable is re-queued interactively. Outline, assembly, summary and the other
  post-stage generators stay interactive. Verified with a real Gemini batch.
- **Diagrams in DOCX**: concept-map figures are rasterised to PNG with the
  headless browser (the one used for PDF) and embedded; without a browser the
  text outline stays.
- **LMS push**: the `webhook` delivery channel posts a signed JSON message
  (`X-Klasync-Signature: sha256=…`, HMAC of the body) with 24-hour download
  links to a per-course URL (plug-in `config` `{url, secret}`; the secret is
  write-only). https only, private/loopback addresses refused, no redirects
  (`MATERIALS_WEBHOOK_ALLOW_HTTP` / `_ALLOW_PRIVATE` relax this for local
  development). A connector, Zapier/Make flow or script turns it into a Canvas
  page, Moodle resource and so on.
- **Slides & notes**: a lecturer can upload a `.pptx` (text read from the
  OOXML), a `.txt`/`.md`, or paste text; it is added to the outline and section
  prompts as context (`PUT .../materials/slides[/pptx]`, migration 0017).

**Not built (do not assume it exists):**

- **Native LMS integrations** (Canvas/Moodle/Blackboard APIs): the webhook is
  the generic hook; each LMS needs its own connector on the receiving side.
- **PDF and legacy `.ppt` slides**: no text extractor; export to `.pptx` or
  paste the text. Slides are text only (images are not read).
- **Institution-wide plug-in settings UI**: course overrides have an API and
  UI; `scope = 'global'` rows are managed in the database.

## Map

| Module | Role |
| --- | --- |
| `blocks.rs` | The typed block document, validation, inline Markdown. **Single source of truth.** |
| `registry.rs` | Plug-in traits (`Generator`, `Renderer`, `DeliveryChannel`, `Analyzer`, `Template`) and `Registry::builtin()` |
| `prompts.rs` | Pure generation logic: prompts, tolerant parsing, grounding check, repair, locked-block merge |
| `generate.rs` | The `ai_jobs` pipeline and built-in generators |
| `render/` | `html`, `markdown`, `docx` (hand-built OOXML), `pdf` (headless Chromium), `epub`, `latex`, `svg` (diagrams) |
| `book.rs` | Course book assembly (pure) and persistence |
| `delivery.rs` | `in_app`, `link`, `email` channels; delivery requests and processing |
| `compare.rs` | Alignment, block diff, report, `material_embed` job, analyzer |
| `events.rs` | Outbox relay (Redis stream + plug-in dispatch) |
| `service.rs` | save draft, publish, unpublish, export |
| `store.rs` | SQL helpers, job enqueue (idempotent) |
| `../api/handlers/materials.rs` | HTTP API |

## Pipeline

```
post_lecture / POST .../materials
   -> material_outline            plans sections (AI; falls back to chapters)
   -> N x material_section        one per section, reads only its transcript slice
   -> material_assemble           queued by the last section to finish
        post-stage generators: concept_map_figure, summary, review_questions, glossary
        repair -> validate -> merge locked blocks -> save version (origin "ai")
```

* Jobs are keyed on (material, generation, section, nonce): retries and double
  triggers never duplicate; a job from an older generation exits quietly.
* Every block cites transcript spans or is marked `supplemented` +
  `low_confidence` (`prompts::ground_blocks`).
* After the retries are spent a section falls back to transcript text rather
  than blocking the lecture.
* Lecturer-`locked` blocks survive regeneration (`prompts::merge_locked`,
  `replace_section`).

## Adding a plug-in

1. Implement the trait from `registry.rs` (depend only on `blocks.rs`).
2. Add it to `Registry::builtin()`.
3. Done. Enable/disable per course or globally with a
   `material_plugin_settings` row; no deploy of the core.

Plug-ins can react to events by implementing `on_event` on `DeliveryChannel` or
`Analyzer` (see `compare::ComparisonAnalyzer`, which queues section embeddings
on `material.published`). Events: `material.generated`, `material.published`,
`material.unpublished`, `material.delivery_requested`, `book.stale`,
`book.published`. They are written in the same transaction as the change and
relayed at least once; handlers must be idempotent.

## API

Lecturer, per lecture (`{code}` = session short code):

| Route | Purpose |
| --- | --- |
| `GET/POST/PATCH /sessions/code/{code}/materials` | read material + versions + deliveries / (re)generate / set status or template |
| `PUT .../materials/draft` | save edits (`{doc, base_rev}`; 409 on a stale `base_rev`) |
| `POST .../materials/sections/{block_id}/regenerate` | rewrite one section with an instruction |
| `POST .../materials/publish` | publish; `{deliver: ["email"], message}` |
| `POST .../materials/unpublish` | hide (`{archive: true}` to archive) |
| `GET .../materials/export?format=pdf&version=<id or draft>[&mode=url]` | download, or a signed URL |
| `GET .../materials/transcript` | transcript lines for review |
| `PUT .../materials/slides`, `PUT .../materials/slides/pptx` | slides/notes as text, or a `.pptx` upload |
| `GET .../materials/versions/{id}` | one version |

Course level: `GET /materials/capabilities`, `GET /courses/{id}/materials`,
`GET/PUT /courses/{id}/materials/settings`,
`GET /courses/{id}/materials/plugins`, `PUT|DELETE /courses/{id}/materials/plugins/{kind}/{plugin_id}`, `GET/PUT /courses/{id}/book`,
`POST .../book/rebuild`, `POST .../book/publish`, `GET .../book/export`,
`POST /courses/{id}/compare?a=&b=&narrative=`, `GET /courses/{id}/comparisons`,
`GET /comparisons/{id}`.

Students: `GET /students/sessions/{id}/material` (+ `/export?format=`),
`GET /students/courses/{id}/materials`, `GET /students/courses/{id}/book`
(+ `/export`). Access: attended, claimed, or enrolled.

## Configuration

| Variable | Default | Meaning |
| --- | --- | --- |
| `MATERIALS_PDF_BROWSER` | probed (`/opt/pw-browsers`, `/usr/bin/chromium*`) | Chromium binary for PDF. Without one, PDF reports unavailable and publish falls back to DOCX |
| `MATERIALS_PDF_CONCURRENCY` | `2` | Parallel PDF renders |
| `MATERIALS_KATEX_URL` | unset | Base URL of a KaTeX distribution; when set, HTML/PDF render equations and inline `$math$` |

Per-course settings (`course_material_settings`): auto-generate, audience
level, depth, language, tone, template, seed from previous offering.

## Testing

```
cargo test                                   # 89 unit tests, no database needed
TEST_DATABASE_URL=postgres://... cargo test --test materials_pipeline
GEMINI_API_KEY=... HTTP_EXTRA_CA_FILE=... TEST_DATABASE_URL=... \
  cargo test --test materials_pipeline real_model -- --ignored --nocapture
```

The database tests run the whole job chain without an AI provider (fallbacks),
then publish, book, delivery, events, exports and comparison. The ignored test
uses a real model and prints the generated handout. A local Postgres without
pgvector works if `vector(1536)` columns are swapped for `text` in migrations
0013/0014 (only the embedding paths need the real type).

Gotcha: models write LaTeX with single backslashes inside JSON strings, which
JSON reads as tab/form-feed (`\text` becomes TAB + "ext").
`prompts::fix_json_latex_escapes` repairs this before parsing.
