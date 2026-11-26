# Assessments v2: backlog

Everything deferred from v1 except the gradebook (see
`assessments-v2-gradebook.md`). Ordered roughly by value. Each item states the
problem, the approach, what v1 must preserve to make it cheap, and acceptance.

Spec doc: Claude doc "Assessments: Design Spec (v1 and v2)".

## 1. Question bank

**Problem.** Lecturers rewrite similar questions every year. Good questions
have measured quality that is lost when an assessment is archived.

**Approach.**
- `question_bank_items` with a stable id, tags (topic, Bloom level, difficulty), course/department scope and sharing rules (private, course, department).
- Questions in assessments link to a bank item version; bank edits do not alter published assessments.
- Item statistics accumulate across every use: difficulty, discrimination, distractor performance, usage count, last used.
- "Insert from bank" and "save to bank" in the builder. Filters by topic, difficulty, stats, last used.
- Retire items that are consistently mis-keyed or non-discriminating.
- Seed a new offering's assessment from last year's, parallel to the materials "seed from previous offering".

**v1 must preserve.** Stable `assessment_questions.id`, `topic_tag`, `origin`, and answer-level data that item analysis already uses.

**Done when.** A lecturer builds a quiz from bank items and sees historical difficulty beside each item.

## 2. More question types

| Type | Notes |
| --- | --- |
| Numeric | Tolerance (absolute or relative), units, significant figures, formula-derived variants per student |
| Fill in the blank / cloze | Multiple blanks, accepted answers per blank |
| Matching and ordering | Partial credit per pair or per position |
| Hotspot / diagram labeling | Click regions on a figure, including the lecture concept map |
| Equation entry | LaTeX input and symbolic equivalence check |
| Code | Sandboxed execution against tests, resource limits, language allowlist; needs a separate isolated runner, never in the API process |
| File upload / handwritten work | Image or PDF upload with optional OCR; storage in R2; size and type limits; AI-assisted marking with rubric |
| Audio / oral (viva) | Record an answer; transcribed by the existing pipeline; graded against a rubric |

Each type needs: a payload schema, an authoring editor, a take component, a
grader, an item-analysis rule, and an accessibility review. Add types behind the
same plug-in style interface the materials engine uses (a registry; no
migration for a new type because payloads are jsonb).

## 3. Advanced integrity and analytics

- **Response similarity** across students: shared wrong answers, identical rare phrasing, answer order patterns, using embeddings and set statistics. Output is a ranked review list with the evidence, never a verdict.
- **Timing anomalies**: implausibly fast correct answers, synchronous submissions.
- **Essay originality**: similarity to the lecture material, the web and other submissions. Treat AI-text detectors as unreliable; do not use them to penalise. Prefer process evidence (revision history, typing cadence) with consent.
- **Light proctoring** (optional, institution-enabled): periodic webcam snapshots with consent, retained briefly, reviewed by a human. High privacy cost; ship only if an institution requires it.
- **Secure browser integration** for institutions that mandate it (for example Safe Exam Browser config export).
- **Forensic report** per attempt: timeline of focus changes, paste events, offline gaps and sync order for a lecturer's review.

Principle unchanged from v1: flags support a human decision.

## 4. Collaborative marking and roles

- Roles: co-lecturer, TA marker, moderator, external examiner (read-only).
- **Double-blind second marking**: two markers grade independently, a moderator resolves disagreements above a threshold; agreement statistics reported.
- Marker allocation by question or by script; progress and inter-marker consistency dashboards.
- Needs the v2 role model in auth (today there are `Lecturer` and `Student` roles).

## 5. Student regrade requests and appeals

- Student raises a query on one question with a reason, inside a window.
- Lecturer sees it with the grade history, responds, and optionally changes the grade (new append-only entry).
- Optional escalation to a moderator. Fully audited. Notification through email.
- Bulk correction when a key error is found: "everyone who chose B on Q4 gets credit", with a diff preview.

## 6. Adaptive practice and learning loops

- Practice mode generated from weak topics, using mastery from past assessments.
- Connect to existing flashcards: missed questions create flashcards; spaced repetition schedule.
- "Re-teach" suggestions for lecturers: which lecture moments to revisit, linked to chapters.
- Adaptive difficulty selection (needs enough data; later IRT, see below).

## 7. Psychometrics

- Item response theory (Rasch / 2PL) when cohorts are large enough.
- Reliability (Cronbach's alpha), standard error of measurement, test-level difficulty targeting.
- Equating across versions so grades are comparable year to year.

## 8. Groups and peer work

- Group assessments with shared and individual components and contribution notes.
- Peer review with anonymised assignment, rubric, calibration, and aggregation rules.

## 9. Rubric library and templates

- Reusable rubrics with criteria, levels and descriptors; share across courses.
- Assessment templates (settings presets): "weekly low-stakes quiz", "invigilated mid-term", "take-home".
- Institution-level defaults and locked policies (for example, integrity level minimums for exams).

## 10. Early warning and cohort analytics

- At-risk indicators combining attendance (`attendance_score`), assessment trend, missing work and time-on-task.
- Lecturer nudge tools: select students (by indicator), send a templated email or in-app note.
- Department dashboards: cohort comparison across offerings, topic mastery over years, alongside the materials comparison report.
- Privacy: minimum cohort sizes, role-limited access, retention rules.

## 11. Accessibility and inclusion

- Student-level accommodation profiles that apply automatically to every assessment (extra time, screen-reader mode, reduced motion, large type, alternative formats), managed by an authorised role, with a reason and expiry.
- Full WCAG audit of the take and grading UIs; keyboard-only grading; captions for audio questions.
- Multilingual assessments: translate a published assessment with lecturer review; per-student language option.

## 12. Notifications and delivery

- Assessment published, due soon, closing, results released, regrade answered: in-app and email.
- Digest options. Reuse the materials delivery-channel interface so email, in-app and LMS notifications share one path.
- Opt-in push on mobile.

## 13. Platform and operations

- Institution admin console: policies, AI budgets per department, AI-grading enable/disable, retention.
- Export and deletion tooling for student data requests.
- Warehouse export of anonymised assessment data for research and accreditation.
- Mobile apps: first-class offline-first take experience (the Android build already exists in CI).
- Observability: dashboards for autosave failure rate, sync lag, AI grading latency and cost per assessment.

## Sequencing suggestion

1. Gradebook G0-G3 (highest demand once assessments are used).
2. Question bank and numeric/cloze/matching types.
3. Regrade requests and TA roles.
4. Integrity analytics and early warning.
5. Code, file-upload and oral types; psychometrics; peer and group work.

## Open questions

- [ ] Which v2 item do the first institutions ask for first? Reorder accordingly.
- [ ] Is code assessment in scope for the intended courses, or mostly written and quantitative work?
- [ ] Are there legal constraints on proctoring and biometric data in the target countries?
- [ ] Is an external examiner workflow needed?
