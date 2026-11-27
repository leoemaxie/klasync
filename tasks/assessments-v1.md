# Assessments v1: quizzes and homework from a lecture

Spec (full rationale, data model, diagrams): Claude doc "Assessments: Design
Spec (v1 and v2)". This file is the working checklist.

Status: **built and verified** (backend, frontend, tests, and a real-browser run) except
the items left unticked below. See `backend/src/assessments/README.md` for the map and the
list of what is not built. Depends softly on the materials engine (optional generator
input) and not at all on its worker split.

## Goal

A lecturer turns a lecture into a graded assessment: writes questions, has AI
draft them, or both. They publish it to the enrolled students, who take it from
their study archive, even on a flaky connection. Scoring is manual, AI-proposed
and lecturer-confirmed, or AI-final. The lecturer grades **blind**. Reports show
how the class is learning. The gradebook is v2.

## Decisions locked (from the product owner)

1. Scope includes every question type below, manual scoring, AI scoring, and reports.
2. **Integrity is fine-grained per assessment**, set by the lecturer.
3. The gradebook is v2 and is documented in `assessments-v2-gradebook.md`.
4. **Attempts must survive loss of connectivity.**
5. Students take assessments **only through their student account** (archive / dashboard). No anonymous or code-only access.
6. **The lecturer does not see who a student is while grading.** Identity is revealed only after grading is finalised.

## Out of scope for v1

Gradebook and weighted course grades, LMS grade passback, reusable question bank,
numeric/code/file-upload questions, student regrade requests, TA/co-grader roles,
similarity or plagiarism analytics, group work. All tracked in v2 files.

## Vocabulary

- **Assessment**: one quiz or homework, anchored to a lecture session.
- **Version**: immutable snapshot cut on publish. Attempts pin a version.
- **Attempt**: one student's go at a version.
- **Answer**: a student's response to one question within an attempt.
- **Grade**: a score for an answer, from auto, AI or the lecturer. Grades are append-only.
- **Release**: making results visible to the student. Independent of grading.
- **Reveal**: the lecturer seeing identities. Happens at finalise.

## Data model (migrations `0020`-`0029`)

All ids are `uuid`. `*_at` are `timestamptz`.

```
assessments
  id, course_id -> courses, session_id -> lecture_sessions (primary anchor),
  lecturer_id -> lecturers, title, instructions_md,
  kind ('quiz'|'homework'|'practice'),
  status ('draft'|'published'|'closed'|'finalized'|'archived'),
  settings jsonb            -- typed AssessmentSettings, see below
  draft_rev int,            -- optimistic concurrency for the editor
  current_version_id, published_version_id,
  opens_at, due_at, closes_at,
  results_released_at,      -- null until released
  identity_revealed_at,     -- null until finalized
  created_at, updated_at

assessment_sources          -- multi-lecture ready; v1 uses one row
  assessment_id, session_id, primary bool

assessment_questions        -- the editable draft rows
  id (stable across versions), assessment_id, position,
  type ('mcq_single'|'mcq_multi'|'true_false'|'short_answer'|'long_answer'),
  prompt_md, points numeric(6,2), difficulty ('easy'|'medium'|'hard'),
  topic_tag, chapter_id -> session_chapters,
  public_payload jsonb      -- options, constraints (safe for students)
  key_payload jsonb         -- correct answers, accepted answers, rubric (NEVER to students)
  explanation_md,
  origin ('lecturer'|'ai'|'ai_edited'),
  source_ranges jsonb,      -- transcript spans the question was drawn from
  ai_job_id -> ai_jobs, locked bool, created_at, updated_at

assessment_versions         -- immutable
  id, assessment_id, version_no, snapshot jsonb (questions + settings),
  content_hash, published_by, created_at

assessment_accommodations   -- per-student overrides
  assessment_id, student_account_id, extra_time_pct, extra_attempts,
  due_override, reason, created_by

assessment_attempts
  id, assessment_id, version_id, student_account_id, attempt_no,
  status ('in_progress'|'submitted'|'auto_submitted'|'void'|'graded'),
  grading_alias text,       -- "Candidate 17", unique per assessment
  started_at, deadline_at,  -- SERVER-computed
  submitted_at, submitted_offline bool, claimed_submit_at,
  question_order jsonb, option_orders jsonb, seed bigint,
  device_id, integrity_summary jsonb,
  final_score numeric, max_score numeric, late bool,
  unique (assessment_id, student_account_id, attempt_no)

attempt_answers
  id, attempt_id, question_id, response jsonb, revision int,
  client_op_id uuid unique, time_spent_ms, answered_at,
  flagged_for_review bool,
  current_grade_id -> answer_grades
  unique (attempt_id, question_id)

answer_grades               -- append-only audit trail
  id, answer_id, source ('auto'|'ai'|'lecturer'),
  status ('proposed'|'confirmed'|'overridden'|'rejected'),
  score, max_score, feedback_md, rubric_scores jsonb,
  confidence numeric, evidence jsonb,
  model, prompt_version, ai_job_id, created_by, created_at

assessment_integrity_events
  id, attempt_id, kind, at, client_at, meta jsonb

assessment_events           -- transactional outbox, same pattern as material_events
  id, kind, payload jsonb, created_at, relayed_at
```

`AssessmentSettings` (validated in Rust, stored as jsonb; every field has a safe default):

| Group | Settings |
| --- | --- |
| Availability | `opens_at`, `due_at`, `closes_at`; late policy `none` \| `flag` \| `penalty_pct_per_day` \| `reject`; grace seconds for in-flight submit |
| Attempts | `max_attempts`; keep `best` \| `last` \| `average`; cooldown between attempts |
| Timing | `time_limit_sec` (0 = none); auto-submit at deadline; per-question time limit (optional) |
| Delivery | shuffle questions; shuffle options; one question per page; allow backtracking; question pools (phase 6) |
| Integrity | level `open` \| `monitored` \| `strict`; fullscreen required; block paste; auto-submit after N violations (optional) |
| Offline | `allow_offline_continue` (default true); `allow_offline_start` (default false); `max_offline_gap_sec` (optional) |
| Scoring | mode `manual` \| `ai_confirm` \| `ai_auto`; per-question-type override; partial credit rule; negative marking |
| Release | `immediate` \| `after_due` \| `after_finalize` \| `manual`; visibility `score` \| `+correctness` \| `+answers` \| `+feedback` |
| Audience | all enrolled (v1) |

## Lifecycle

Assessment: `draft -> published -> closed -> finalized -> archived`.
Editing a published assessment cuts a new version. Existing attempts stay on
their version. Changing a question after attempts exist requires choosing
"regrade affected attempts" or "apply to new attempts only".

Attempt: `in_progress -> submitted | auto_submitted | void -> graded`.
Release is a flag on the assessment, not an attempt state.

## Blind grading (decision 6): how it is enforced

- Grader endpoints return `grading_alias`, never `student_account_id`, name or matric number.
- AI grading jobs receive the alias only. Identity is never placed in a prompt.
- The submit timestamp and attempt number are hidden in the grading view (they can identify a student).
- The lecturer **can** see roster-level status (not started / in progress / submitted) so they can chase missing work, but not who scored what.
- `finalize` sets `identity_revealed_at`, locks all grades, and from then on the lecturer endpoints return names with scores. Reveal is irreversible and audit-logged.
- A unit test asserts the grader response types contain no identity fields.

## Integrity (decision 2): honest framing

A browser cannot stop cheating. v1 aims to **deter, record, and let the lecturer
decide**, never to auto-accuse.

- Server-authoritative clock: `deadline_at` is computed on the server at attempt start. The client countdown is cosmetic.
- Answer keys and explanations never reach the client before release (separate serialiser types).
- Seeded shuffle of questions and options, stored per attempt so a resume is identical.
- One active device per attempt. A takeover is allowed and logged as an integrity event.
- `monitored` logs: focus loss, tab hidden, paste, copy, fullscreen exit, offline gaps, clock skew, device change.
- `strict` adds: fullscreen required, paste blocked in answer fields, one question per page, no backtracking, optional auto-submit after N violations.
- Events are **flags for review**, shown to the lecturer after identity reveal or in the aggregate. They are never used to change a score automatically.
- Question pools (draw N of M by topic) are the strongest anti-collusion lever and land in phase 6.
- Accommodations (extra time, extra attempts, due override) are per student and logged.

## Offline and flaky connectivity (decision 4)

Principle: once an attempt has started, the student's work cannot be lost to a
dropped connection.

1. **Start needs the server** by default. It stamps `started_at` and `deadline_at` and returns the full question set (without keys). Optional `allow_offline_start` uses a pre-fetched, signed start token; it is off by default because it weakens the time guarantee.
2. The question set is cached in IndexedDB (extend `klasync_client_db`; bump `DB_VERSION`).
3. Each answer change is appended to a local op log with a client sequence number and `client_op_id`. Autosave is debounced (2-5 s) and batched.
4. The timer uses a monotonic clock anchored to the server deadline plus a measured server offset, so changing the device clock does nothing.
5. `POST /attempts/{id}/sync` accepts a batch of ops idempotently (unique `client_op_id`). Per-question last-write-wins by `revision`.
6. An offline submit queues locally and is sent on reconnect with the claimed monotonic elapsed time. The server accepts it if it is consistent with the deadline plus the grace period; otherwise the attempt is accepted but flagged for the lecturer. It is never silently dropped.
7. UI: reuse `OfflineStatusIndicator`; show "saved on this device" vs "saved to server" per answer; a submit receipt id is shown once the server confirms.
8. `max_offline_gap_sec` (optional) flags long offline gaps in strict assessments.
9. Check whether the desktop app (`src-tauri/migrations`) mirrors the backend schema and needs these tables (open question).

## AI generation

- Job type `assessment_generate`. Inputs: selected transcript spans and chapters, concept map, summaries, course settings, optionally the published lecture material. Controls: count, types, difficulty mix, topics.
- Output is validated JSON. Every question carries `source_ranges` and `topic_tag`.
- Quality pass, in order: schema validation; single-correct-answer check for MCQ; **solve-back** (a second call answers the question from the transcript without the key and must match); duplicate check by embedding against the assessment's other questions; low-confidence flags.
- Questions land as **drafts** marked AI. The lecturer accepts, edits or rejects each. Nothing AI-written is visible to students until published.
- Interactive priority (the lecturer is waiting) with progress reporting. An optional "auto-draft after lecture" runs in batch mode at low priority.

## AI grading

- Job type `assessment_grade`, batch mode by default (nobody is waiting), under the daily budget gateway with its own usage label.
- Objective types are graded deterministically on the server, no AI.
- Short answers: normalised accepted-answer match first (case, whitespace, listed synonyms); AI fallback only if the lecturer enabled it for that question.
- Long answers: rubric-based. The model returns per-criterion scores, feedback, confidence, and quoted evidence from the answer. Stored as `proposed`.
- Modes: `manual`; `ai_confirm` (AI proposes, lecturer confirms; low-confidence first, bulk-accept above a threshold); `ai_auto` (AI grade is final, lecturer can still override).
- Student answers are **untrusted input**: delimited in the prompt, structured output only, instructions inside an answer are ignored, and answers that address the grader are flagged.
- Calibration (phase 4 stretch): the lecturer grades a sample, which becomes few-shot context, and the UI shows AI-vs-lecturer agreement.

## Reports

Lecturer (aggregates before reveal; named per-student only after):

- Funnel: assigned, started, submitted, graded, released.
- Score distribution, mean, median, standard deviation, pass rate, time-on-task.
- **Item analysis** per question: difficulty (percent correct), discrimination (top vs bottom 27%), option pick rates, skip rate, average time. Flags: too hard, likely mis-keyed (strong students choose a distractor).
- Topic and chapter mastery, linked to the lecture chapter ("re-teach this").
- Optional AI-written narrative summary.
- Progress over time per student (after reveal). CSV export.

Student:

- Result per the release visibility setting. Per-question feedback with a jump to the transcript moment.
- Topic mastery and history across assessments in the course.

## API (REST; lecturer and student scopes, same style as the materials API)

Lecturer:

- `POST /sessions/{code}/assessments`, `GET|PATCH /assessments/{id}`
- `POST|PATCH|DELETE /assessments/{id}/questions`, `POST .../questions/reorder`
- `POST /assessments/{id}/generate`
- `POST /assessments/{id}/publish`, `/close`, `/finalize`, `/release`, `/archive`
- `GET /assessments/{id}/roster-status`
- `GET /assessments/{id}/grading/queue?question=`, `PUT /grading/answers/{id}`, `POST /grading/ai-run`
- `GET /assessments/{id}/report`, `/report/items`, `/report/topics`, `/export.csv`
- `PUT /assessments/{id}/accommodations`

Student:

- `GET /students/assessments`, `GET /students/assessments/{id}`
- `POST /students/assessments/{id}/attempts`, `GET /attempts/{id}`
- `PUT /attempts/{id}/answers`, `POST /attempts/{id}/sync`, `POST /attempts/{id}/events`
- `POST /attempts/{id}/submit`, `GET /attempts/{id}/result`
- `GET /students/progress`

## Frontend

- Lecturer: a new **Assess** tool in the lecture workspace (`tools.ts` gets a `ToolId`; coordinate with the materials agent, who may also add a tool tab). Builder, AI generate dialog, grouped settings panel, preview-as-student, live "who is in progress", blind grading workbench (one question at a time across all attempts), reports.
- Student: an **Assessments** section in the study archive per course and lecture: upcoming / open / in progress / submitted / graded. A rules screen before start, then the question view with navigator, flag-for-review, autosave indicator and offline badge.

## Events (outbox)

`assessment.published`, `attempt.submitted`, `assessment.graded`,
`assessment.finalized`, `results.released`. Written in the same transaction as
the change, relayed through the existing Redis stream. Used for notifications
now and the gradebook in v2.

## Phases and tasks

### Phase 0: Foundations
- [x] Reserve migration range `0020`-`0029`; write `0020_assessments.sql` with tables above
- [x] Extend `ai_jobs_job_type_check` with `assessment_generate`, `assessment_grade`, `assessment_report` **re-listing every existing type including the materials types**
- [x] `backend/src/assessments/` module with `STATUS.md`; question schema, validation, serde types
- [x] Separate student-view and lecturer-view question types; test that the student type has no key fields
- [ ] Outbox relay (the table and transactional writes exist; nothing consumes the events yet)
- [x] Settings struct with defaults and validation

Done when: migrations apply cleanly on a fresh DB and an existing one; schema tests pass.

### Phase 1: Author, publish, take (online), auto-score
- [x] CRUD for assessments and questions; optimistic `draft_rev`
- [x] Question types: mcq_single, mcq_multi, true_false, short_answer, long_answer
- [x] Publish cuts a version; student list/detail endpoints scoped to enrolled students
- [x] Attempt start, resume, autosave, submit; server deadline
- [x] Deterministic auto-scoring for objective types
- [x] Lecturer builder UI and student take UI (online only)

Done when: a lecturer publishes a 5-question quiz and an enrolled student takes it and gets a score.

### Phase 2: AI generation
- [x] `assessment_generate` job, prompt versioning, validation, solve-back check, dedupe
- [x] Generate dialog, per-question accept/edit/reject, AI badge
- [ ] Source-range links back to the transcript (shown as timestamps, not yet links)

Done when: generating 10 questions from a lecture yields reviewable drafts with transcript links.

### Phase 3: Integrity and offline
- [x] Settings UI (availability, attempts, timing, delivery, integrity, offline)
- [x] Seeded shuffle; one-active-device lock; integrity event capture and storage
- [x] IndexedDB op log; sync endpoint; offline submit; clock handling; submit receipt
- [ ] Accommodations UI (the API and client exist; the backend rules are done and tested)
- [x] Tests: drop the network mid-attempt, change the device clock, double submit, two devices

Done when: an attempt started online, finished fully offline and synced later is stored once, correctly, and flagged only if inconsistent.

### Phase 4: Grading
- [x] Blind grading workbench: question-by-question across aliased attempts
- [x] `assessment_grade` job; `ai_confirm` and `ai_auto` modes; prompt-injection defences
- [x] Append-only grades, overrides, finalize and reveal, release policies
- [x] Test that no grader endpoint or AI prompt contains identity before finalize

Done when: a lecturer grades blind, finalizes, sees names, and releases; students see exactly what the visibility setting allows.

### Phase 5: Reports
- [x] Funnel, distribution, item analysis, topic mastery; CSV export
- [x] Student result and progress views; transcript-moment links
- [ ] Optional AI narrative

Done when: the lecturer can name the three hardest questions and the weakest topic for a class.

### Phase 6: Hardening
- [ ] Question pools
- [ ] Load test: 500 students start together, autosave every 5 s
- [ ] Accessibility pass (screen reader, keyboard, extended time)
- [ ] Retention job for integrity events; institution switch to disable AI grading
- [ ] Docs and runbook

## Risks

| Risk | Mitigation |
| --- | --- |
| AI mis-grades | Default `ai_confirm`; confidence ordering; append-only overrides; calibration view |
| Lecturers over-trust integrity flags | Wording is "for review"; never auto-penalise; document limits |
| De-anonymisation by metadata | Hide submit time and attempt number in grading view; aggregate-only before reveal |
| Offline start abused | Off by default; signed token; consistency check; flagged |
| Peak load when a class starts at once | Debounced batched autosave; indexed upserts; concurrency limits on grading jobs |
| Cost spikes from AI grading | Batch mode, budget gateway, per-assessment caps |
| `ai_jobs` constraint clobbering other features | Re-list all types; CI check; lookup-table follow-up |
| Student data sent to a model provider | Alias only, no identity; institution opt-out; disclosed to students |

## Open questions

- [ ] Do assessments need a `course`-level (no lecture) mode in v1, or is a lecture anchor always required? (v1 assumes a lecture anchor; `assessment_sources` makes multi-lecture possible later.)
- [ ] Should students see a draft preview? (Assumed no.)
- [ ] Which languages are needed for AI generation and grading in v1?
- [ ] Is the desktop app's local schema (`src-tauri/migrations`) expected to carry assessment tables?
- [ ] Default retention for integrity events and attempt logs?
- [ ] Does the institution need to disable AI grading globally or per course?
