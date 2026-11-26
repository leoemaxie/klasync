# Assessments (quizzes and homework)

Lecture-linked quizzes and homework: a lecturer writes or AI-drafts questions
from a lecture, publishes them to the enrolled students, who take them from
their study archive (offline-safe). The lecturer grades blind, manually or with
AI, finalises, releases results and reads class reports. Design: the
"Assessments: Design Spec (v1 and v2)" doc and `tasks/assessments-v1.md`.

## Map

| Module | Role |
| --- | --- |
| `model.rs` | Question types, `AssessmentSettings`, the version snapshot, the **student views (no key fields)** and the blind grader view |
| `scoring.rs` | Deterministic scoring, short-answer matching, late penalties, which attempt counts |
| `delivery.rs` | Seeded shuffle, server deadline, start rules, offline/late sync judgement, integrity event vocabulary |
| `report.rs` | Pure analytics: summary, histogram, item analysis, topic mastery |
| `prompts.rs` | Generation, solve-back and grading prompts; tolerant parsing; injection defences |
| `store.rs` | Row types and shared SQL |
| `authoring.rs` | Create, edit, questions, versioned publish, close, accommodations, roster status |
| `attempts.rs` | Start/resume, idempotent sync, submit, integrity events, filtered results |
| `grading.rs` | Blind queue, append-only grades, finalise (reveal), release, named results |
| `reports.rs` | Loads class reports (aggregates only) and a student's own progress |
| `generate.rs` | The `assessment_generate` and `assessment_grade` jobs on the shared `ai_jobs` queue |
| `../api/handlers/assessments.rs` | HTTP API (routes in `api.rs`) |

Frontend: `src/lib/api/assessments.ts` (client), `src/lib/assessments/` (offline
attempt engine, integrity watcher, client rules), `src/lib/storage/attemptStore.ts`
(IndexedDB), `src/lib/components/assessments/` (student and lecturer screens).

## Invariants worth protecting

- **No answer key reaches a student before release.** Student types have no key fields; a test asserts the serialised attempt view contains none.
- **Grading is blind in the API.** The grader queue and AI prompts carry only an alias; identity is joined to a score only in `named_results`, which refuses until finalised. Tests assert this.
- **Grades are append-only.** An override supersedes (`overridden` / `rejected`), never deletes.
- **The server owns time.** The deadline is computed at start; offline submits are accepted past the deadline only if their measured elapsed time fits, and are flagged.
- **Migration `0020` is additive for `ai_jobs_job_type_check`.** It reads the existing allowed types (both `ARRAY[...]` and `'{...}'` forms) and extends them, so it cannot drop another feature's job types and is safe to re-run.

## Status

Built and tested: everything in the map above, plus the lecturer and student UI.
Verified end to end in a real browser against the real backend (including genuine
offline emulation); AI paths were exercised against a mock Gemini server because
the provider account was over its spend cap.

**Not built (do not assume it exists):**

- **Event relay.** `assessment_events` rows are written transactionally, but nothing reads them yet (no Redis mirror, no notifications, no gradebook consumer).
- **Batch mode for AI grading.** Jobs run interactively on the existing worker, four answers at a time.
- **Question pools**, **load test**, **retention job**, **institution switch to disable AI grading**, a full accessibility audit.
- **Accommodations UI.** The API and client functions exist; there is no screen yet.
- **Regrade on republish.** Republishing affects new attempts only; existing attempts keep their version.
- **Transcript jump links.** Source moments show as timestamps, not as links into the recording.
- **AI narrative summary** in reports.
- **Desktop (`src-tauri`) schema.** Not mirrored.
- **Discoverability for students.** The Assessments view lives in the archive sidebar, which is collapsed while a lecture is open.
