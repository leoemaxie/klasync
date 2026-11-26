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
