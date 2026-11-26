# Assessments v2: the gradebook

Status: **planned for v2.** Nothing here is built. v1 is specified in
`assessments-v1.md`; this document describes what v1 must leave room for and
what v2 adds.

Spec doc: Claude doc "Assessments: Design Spec (v1 and v2)".

## Why a gradebook, and why not in v1

In v1 an assessment produces scores. A lecturer's real question is "what is this
student's grade in the course?", which combines several assessments, other
graded work, weights, drop rules, late penalties and a letter scale. That is a
different feature with its own correctness bar (grades are legal records), so it
ships after assessments are proven.

## What v1 must do now so v2 is cheap

These are v1 obligations, listed here so they are not forgotten.

- [ ] Scores are stored per answer with an append-only grade history (`answer_grades`). The gradebook reads the **current confirmed** result and never recomputes from raw responses.
- [ ] `assessment_attempts` stores `final_score`, `max_score`, `late`, and which attempt counts under the keep policy (`best` / `last` / `average`).
- [ ] The `assessment_events` outbox emits `assessment.finalized` and `results.released` with enough data for a gradebook consumer to react without calling back.
- [ ] `assessments.course_id` is always set (gradebook is per course offering).
- [ ] Money-style rounding discipline: store scores as `numeric`, never `float`.
- [ ] Release is a separate flag from grading, so a gradebook can have its own release control without changing assessments.

## Concepts

- **Grade item**: one column in the gradebook. Kinds: `assessment` (linked), `manual` (typed in), `attendance` (derived from existing attendance scoring), `imported` (CSV/LMS), `calculated` (formula over other items).
- **Category**: a group of items with a weight (Quizzes 20%, Homework 30%, Exam 50%) and a rule set (drop lowest N, cap, best N of M).
- **Grade scheme**: the mapping from percentage to letter / GPA points / pass-fail. Per institution, overridable per course within limits.
- **Grade ledger**: append-only record of every grade value for a student and item, with who, when, why.
- **Course grade**: the computed total for a student, with a breakdown showing exactly how it was reached.
- **Release**: per item and per course, independent. Students never see a course grade the lecturer has not released.

## Data model (sketch, migrations in a later range)

```
gradebooks            id, course_id unique, scheme_id, rounding_rule,
                      missing_policy ('zero'|'excuse'|'exclude'), released_at
grade_categories      id, gradebook_id, name, weight_pct, rule jsonb, position
grade_items           id, gradebook_id, category_id, kind, title, max_points,
                      source_assessment_id, weight_in_category, due_at,
                      extra_credit bool, visible_to_students bool, position
grade_entries         id, item_id, student_account_id, points, status
                      ('graded'|'missing'|'excused'|'late'|'incomplete'),
                      source ('assessment'|'manual'|'import'|'override'),
                      note, created_by, created_at          -- append-only ledger
grade_entry_current   view/table: latest non-void entry per (item, student)
course_grades         id, gradebook_id, student_account_id, computed_pct,
                      letter, gpa_points, breakdown jsonb, computed_at,
                      released_at, overridden_by, override_reason
grade_schemes         id, institution_id?, name, bands jsonb
grade_audit           id, actor, action, subject, before, after, at
```

## Calculation engine

A **pure function** `compute(gradebook, entries, scheme) -> CourseGrade` with no
database access, so it is unit-testable and reproducible.

- [ ] Weighted categories; weights need not sum to 100 and are normalised when items are missing, per `missing_policy`.
- [ ] Rules: drop lowest N, keep best N of M, cap at max, extra credit.
- [ ] Late penalties applied from the assessment's late policy, recorded as a separate adjustment line so it is explainable.
- [ ] Excused items removed from both numerator and denominator.
- [ ] Deterministic rounding at one defined step only (final percentage), configurable half-up or half-even, tested with boundary cases (89.5, 89.4999).
- [ ] Output includes a full **breakdown** (per item contribution) rendered verbatim to the student.
- [ ] Property tests: reordering items does not change the result; adding an excused item changes nothing; weights scale invariantly.
- [ ] Golden files: real anonymised gradebooks from a few lecturers reproduced to the digit.

## Lifecycle and consistency

- Recompute is triggered by outbox events (`assessment.finalized`, `grade.entry.changed`, roster change), never inline in a request.
- A course grade is a **cache** of the ledger plus rules. If a rule changes, recompute and show a diff to the lecturer before applying ("12 students change letter grade").
- Override: a lecturer can override a course grade with a mandatory reason. The override is an audit event and stays visible beside the computed value.
- Late enrolment and withdrawal: new students start with `missing` entries per policy; withdrawn students are retained but hidden and excluded from class statistics.
- Concurrent edits to the same cell: last-writer-wins with a visible conflict banner and ledger history.
- Locking: a gradebook can be **finalised** for the term; further changes require an unlock with reason, audited.

## Student view

- Per-course grade page: current grade (if released), a per-item table, category subtotals, and the breakdown.
- **What-if**: the student enters hypothetical scores for unreleased items and sees the projected grade. Clearly labelled hypothetical; never stored as a grade.
- "Needed to reach X" calculator using remaining weight.
- Progress and mastery from v1 reports, side by side with the grade.
- Appeal / query button per item (see backlog: regrade requests).

## Lecturer view

- Spreadsheet-style grid: students down, items across, categories grouped, live totals. Keyboard navigation, paste from a spreadsheet, undo.
- Column menu: hide from students, release, excuse all, set due date, view the linked assessment's report.
- Filters: missing, late, below threshold, overridden.
- Identity is shown here by design (the gradebook is post-reveal). Linked assessment items are only filled once the assessment is finalised, preserving blind grading.
- Export CSV and XLSX; import CSV with a column-mapping step and a dry-run diff.

## Integrations

- **LMS passback**: existing `lms_course_sync` stores Canvas / Moodle / Blackboard connections and a roster sync handler exists. v2 adds grade passback: LTI Advantage Assignment and Grade Services for Canvas and Blackboard, and the Moodle web service for Moodle. Push is per item and idempotent, with a per-student result log and a retry queue.
- **Attendance**: expose the existing `attendance_score` (migration `0005`) as a derived grade item, with its own category rule.
- **Early warning**: combine attendance, assessment trend and missing work into an at-risk indicator for lecturers and departments (see backlog).
- **Materials**: when the materials comparison analyzer exists, allow comparing grade distributions across offerings next to content changes.
- **Email**: grade-released notifications through the existing email module and the materials delivery-channel interface.

## Permissions

- Course lecturer: full control of their course gradebook.
- Co-lecturer / TA (needs the v2 role model): configurable per gradebook - view, enter grades for named items, no override.
- Department head: read-only aggregate views (v2 admin console).
- Student: own grades only, only released.
- Every write goes through the audit log.

## Privacy, compliance, retention

- Grades are student records: define retention, export on request, and deletion behaviour with the institution.
- Hidden from other students always; class statistics only above a minimum cohort size (suppress small cells).
- Student grades are never sent to an AI provider unless the lecturer enables the narrative feature, and then only aggregates.
- Record who changed what; keep the ledger even when an entry is "deleted" (voided).

## API sketch

Lecturer:
- `GET|PUT /courses/{offering}/gradebook`
- `GET|POST|PATCH /courses/{offering}/gradebook/categories`, `.../items`
- `PUT /gradebook/items/{id}/entries` (bulk upsert, with dry-run)
- `POST /gradebook/recompute` and `GET /gradebook/recompute/{id}` (preview diff)
- `POST /gradebook/release`, `/finalize`, `/unlock`
- `POST /gradebook/import`, `GET /gradebook/export?format=csv|xlsx`
- `POST /gradebook/passback` (LMS)

Student:
- `GET /students/courses/{offering}/grades`
- `POST /students/courses/{offering}/grades/what-if`

## Phases

### G0: Foundations
- [ ] Migrations for gradebook tables in a new reserved range
- [ ] `compute` engine, property tests, golden files
- [ ] Ledger and audit

### G1: Lecturer gradebook
- [ ] Categories, items, linked assessments, manual items
- [ ] Grid UI with import/export
- [ ] Recompute preview and override

### G2: Student grades
- [ ] Released grade page and breakdown
- [ ] What-if and needed-to-reach

### G3: Governance
- [ ] Release control per item and per course
- [ ] Finalise and unlock with audit
- [ ] Late enrolment, withdrawal, excused handling

### G4: LMS passback and notifications
- [ ] Canvas and Moodle passback with retry and result log
- [ ] Grade-released emails

### G5: Analytics
- [ ] Cohort distributions, cross-offering comparison, at-risk signals

## Risks

| Risk | Mitigation |
| --- | --- |
| A wrong grade reaches a student | Computation is a pure tested function; preview diffs before apply; release is explicit; every override audited |
| Rounding disputes | One documented rounding step; golden-file tests; breakdown shown to students |
| Institutions differ on grading rules | Rules are data (jsonb) not code branches; scheme per institution; start with the common subset |
| LMS APIs vary and change | Passback behind an interface; idempotent; per-provider adapter with contract tests |
| Blind grading undone by gradebook | Linked assessment items stay empty until `finalized` |
| Scope creep into a full SIS | Hard line: no transcripts or enrolment authority in this module |

## Open questions

- [ ] Whose rules govern: department or lecturer? Can a lecturer pick any scheme?
- [ ] Is there a fixed national scale (for example 5.0 / 4.0 GPA or class of degree) the product must default to?
- [ ] Who may unlock a finalised gradebook?
- [ ] Must grade data live in a particular region or system of record?
- [ ] Which LMS do the first institutions use, so passback starts with one adapter?
- [ ] Do students need to see class rank or only their own grade and the class average?
