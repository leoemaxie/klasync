# Lecture materials engine

After each lecture Klasync drafts a structured lecture material for the
lecturer, who reviews, edits and publishes it. Students read it in the app or
export it (PDF, DOCX, Markdown, HTML). A course offering's materials compose
into a **course book**; the same data powers **email delivery** and
**cross-offering comparison**. Design: the "Lecture Materials: Design Spec"
doc. This file is the map for whoever builds on top.

## Status

Built and tested end to end: schema, block document, plug-in registry, the
generation pipeline, review/edit/publish, four renderers, course book, email
and passive delivery channels, comparison analyzer, event outbox, HTTP API,
post-lecture trigger, typed TypeScript client (`src/lib/api/materials.ts`).

**Frontend** (`src/lib/components/materials/`, `src/lib/materials/`):
lecturer *Materials* tab (lecture list, review/edit with transcript
side-by-side, autosave with `base_rev` conflict handling, per-section rewrite,
lock, publish with optional email, export, versions, deliveries), course book
builder, offering comparison, course settings and per-course plug-in toggles;
student *Handout* tool in the lecture studio with export buttons. Pure edit and
validation logic lives in `src/lib/materials/doc.ts` (mirrors `blocks.rs`
validation) and is unit-tested.

**Plug-in settings API** (`GET /courses/{id}/materials/plugins`,
`PUT|DELETE /courses/{id}/materials/plugins/{kind}/{plugin_id}`): course
overrides of the global/default enabled state. `in_app` delivery and the `html`
renderer are protected and cannot be disabled.

**Not built (do not assume it exists):**

- **Student course-book UI.** The student book endpoints and client exist
  (`fetchStudentBook`, `downloadStudentBook`) but no screen uses them; only the
  per-lecture handout is in the UI.
- **Lecture picker for lectures without a material.** The Materials tab lists
  lectures that already have a material, plus the lecture open in the Room tab.
  There is no course-wide session list API to offer older lectures.
- **KaTeX in the browser.** Equations render as monospace LaTeX in the app
  (HTML/PDF exports render them when `MATERIALS_KATEX_URL` is set).
- **Separate worker process** (spec Phase 0). Material jobs run in the API
  process through the existing `ai_worker::run_loop`, like every other job.
- **Provider batch mode** for materials. Jobs run interactively, ordered after
  other work, with the first draft delayed 4 minutes after the lecture so
  chapters and the concept map exist. `batch.rs` would need a materials-aware
  path (`prepare_text_job` does not know these job types).
- **Signed export URLs.** `StorageAdapter` has no presign, so
  `GET .../materials/export` streams the file (cached in object storage).
- **Email attachments.** The email module has no attachment support; email is
  link-only.
- **Real graphics for the concept-map figure.** Figures carry alt text and a
  text outline; the 3D map is not drawn into PDF/DOCX.
- **EPUB, LaTeX export, LMS push, slide upload** (spec Phase 6).
- **pgvector paths** (`material_embed` job, vector alignment) are unit-tested
  with synthetic vectors but were not run against a real pgvector/embedding
  provider; the lexical comparison path was run for real.

## Map

| Module | Role |
| --- | --- |
| `blocks.rs` | The typed block document, validation, inline Markdown. **Single source of truth.** |
| `registry.rs` | Plug-in traits (`Generator`, `Renderer`, `DeliveryChannel`, `Analyzer`, `Template`) and `Registry::builtin()` |
| `prompts.rs` | Pure generation logic: prompts, tolerant parsing, grounding check, repair, locked-block merge |
| `generate.rs` | The `ai_jobs` pipeline and built-in generators |
| `render/` | `html`, `markdown`, `docx` (hand-built OOXML), `pdf` (headless Chromium) |
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
| `GET .../materials/export?format=pdf&version=<id or draft>` | download |
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
cargo test                                   # 82 unit tests, no database needed
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
