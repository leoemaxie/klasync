# Tasks

Open work, one file per task. Each file says why it matters, what to build,
how to know it is done, and where to start in the code.

These follow on from the lecture-materials work in PR #31
(`backend/src/materials/README.md` is the map of what exists). Tasks marked
**needs #31** touch code that lands with that PR.

| # | Task | Size | Needs |
| --- | --- | --- | --- |
| 01 | [Native LMS connectors (Canvas, Moodle)](01-lms-connectors.md) | L | #31 |
| 02 | [Webhook hardening: retries, test button, DNS pinning](02-webhook-hardening.md) | M | #31 |
| 03 | [PDF and legacy PPT slides](03-pdf-ppt-slides.md) | M | #31 |
| 04 | [Institution-wide plug-in settings](04-global-plugin-settings.md) | M | #31 |
| 05 | [Verify email attachments with a real provider](05-email-attachments-verify.md) | S | #31 |
| 06 | [Batch mode follow-ups and stuck-generation watchdog](06-batch-followups.md) | M | #31 |
| 07 | [Diagrams everywhere: LaTeX/Markdown, fresh-map check](07-diagrams-everywhere.md) | M | #31 |
| 08 | [Validate DOCX/EPUB in real readers](08-validate-docx-epub.md) | S | #31 |
| 09 | [Commit the browser and real-stack test suites](09-e2e-suites.md) | M | #31 |
| 10 | [Editor accessibility and mobile layout](10-editor-a11y-mobile.md) | M | #31 |
| 11 | [Publish-blocking problems: jump to the block](11-publish-problems-ux.md) | S | #31 |
| 12 | [Measure the effect of slides on output quality](12-slides-quality-eval.md) | S | #31 |
| 13 | [Deploy: run the worker role](13-deploy-worker-role.md) | S | #31 |

Sizes: S = under a day, M = 1-3 days, L = a week or more.
# tasks/

Planning documents and task breakdowns that live next to the code. Each file is
a checklist you can work through; the long-form design rationale lives in the
Claude spec documents linked from each file.

| File | What it covers | State |
| --- | --- | --- |
| [assessments-v1.md](assessments-v1.md) | Lecture-linked quizzes and homework: authoring, AI generation, taking (offline-safe), blind grading, reports | Built and verified; see the unticked items |
| [assessments-v2-gradebook.md](assessments-v2-gradebook.md) | Gradebook: weighted categories, grade ledger, release, LMS passback | Planned for v2 |
| [assessments-v2-backlog.md](assessments-v2-backlog.md) | Everything else deferred from v1: question bank, richer question types, integrity analytics, TA roles, appeals | Planned for v2 |

Related work already in flight (not in this folder):

- **Lecture materials engine**: branch `claude/build-materials-engine`, spec
  "Lecture Materials: Design Spec". Owns migration numbers `0014`-`0019`.

## Conventions

- **Migration numbers.** Parallel features collide on `backend/migrations/NNNN_*.sql`.
  Reserved ranges: materials `0014`-`0019`, assessments `0020`-`0029`. Rebase and
  renumber inside your own range only.
- **`ai_jobs.job_type` CHECK constraint.** Every migration that adds a job type
  drops and re-creates `ai_jobs_job_type_check`. Each migration must therefore
  re-list *every* existing type or it silently removes another feature's types.
  Before merging, diff the final list against `main` and the other feature
  branches. Proposed follow-up: move the allowed set into a lookup table.
- **New modules are not wired until they compile.** Follow the materials
  engine's pattern: a `backend/src/assessments/` module with a `STATUS.md`,
  added to `lib.rs` only when it builds.
- **Never serialise answer keys to students.** Student-facing types are separate
  Rust structs that have no key or explanation fields, so a leak cannot compile.
- **Blind grading is enforced in the API layer**, not the UI. See
  `assessments-v1.md`, "Blind grading".
