-- Assessments: lecture-linked quizzes and homework (see tasks/assessments-v1.md).
--
-- An assessment belongs to a course and is anchored to a lecture session. Its
-- editable questions are snapshotted into an immutable version on publish;
-- attempts pin a version. Grades are an append-only history per answer, and
-- every student-facing read goes through types with no answer-key fields.
-- Migration range 0020-0029 is reserved for this feature.

-- 1. Job kinds. Additive on purpose: the allowed set is read from the existing
--    constraint and extended, so this migration can never drop another
--    feature's job types (the materials engine rewrote this constraint too).
do $$
declare
  def text;
  merged text[];
begin
  select pg_get_constraintdef(oid) into def
    from pg_constraint
   where conname = 'ai_jobs_job_type_check' and conrelid = 'ai_jobs'::regclass;
  select coalesce(array_agg(distinct m[1]), '{}'::text[]) into merged
    from regexp_matches(coalesce(def, ''), '''([a-z_]+)''', 'g') as m;
  select array_agg(distinct x) into merged
    from unnest(merged || array['assessment_generate', 'assessment_grade']) as x;
  alter table ai_jobs drop constraint if exists ai_jobs_job_type_check;
  execute format(
    'alter table ai_jobs add constraint ai_jobs_job_type_check check (job_type = any (%L::text[]))',
    merged
  );
end $$;

-- 2. The assessment.
create table if not exists assessments (
  id uuid primary key default gen_random_uuid(),
  course_id uuid not null references courses(id) on delete cascade,
  session_id uuid not null references lecture_sessions(id) on delete cascade,
  lecturer_id uuid not null references lecturers(id),
  title text not null,
  instructions_md text not null default '',
  kind text not null default 'quiz' check (kind in ('quiz', 'homework', 'practice')),
  status text not null default 'draft'
    check (status in ('draft', 'published', 'closed', 'finalized', 'archived')),
  -- Typed AssessmentSettings (validated in Rust); every field has a default.
  settings jsonb not null default '{}'::jsonb,
  draft_rev integer not null default 0,
  published_version_id uuid,
  opens_at timestamptz,
  due_at timestamptz,
  closes_at timestamptz,
  results_released_at timestamptz,
  -- Blind grading: null until the lecturer finalises.
  identity_revealed_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);
create index if not exists assessments_course_idx on assessments(course_id, status);
create index if not exists assessments_session_idx on assessments(session_id);

-- Which lectures feed an assessment: one in v1, many later.
create table if not exists assessment_sources (
  assessment_id uuid not null references assessments(id) on delete cascade,
  session_id uuid not null references lecture_sessions(id) on delete cascade,
  is_primary boolean not null default false,
  primary key (assessment_id, session_id)
);

-- 3. Editable draft questions. `id` stays stable across versions so item
--    analysis and a later question bank can group answers by question.
create table if not exists assessment_questions (
  id uuid primary key default gen_random_uuid(),
  assessment_id uuid not null references assessments(id) on delete cascade,
  position integer not null,
  question_type text not null
    check (question_type in ('mcq_single', 'mcq_multi', 'true_false', 'short_answer', 'long_answer')),
  prompt_md text not null,
  points numeric(8, 2) not null default 1 check (points >= 0),
  difficulty text not null default 'medium' check (difficulty in ('easy', 'medium', 'hard')),
  topic_tag text,
  chapter_index integer,
  -- Safe for students: options, length limits.
  public_payload jsonb not null default '{}'::jsonb,
  -- NEVER sent to students: correct answers, accepted answers, rubric.
  key_payload jsonb not null default '{}'::jsonb,
  explanation_md text not null default '',
  origin text not null default 'lecturer' check (origin in ('lecturer', 'ai', 'ai_edited')),
  source_ranges jsonb not null default '[]'::jsonb,
  low_confidence boolean not null default false,
  ai_job_id uuid,
  locked boolean not null default false,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);
create index if not exists assessment_questions_assessment_idx
  on assessment_questions(assessment_id, position);

-- 4. Immutable snapshot cut on publish. The snapshot holds the questions with
--    their keys; attempts read questions from here, not from the draft rows.
create table if not exists assessment_versions (
  id uuid primary key default gen_random_uuid(),
  assessment_id uuid not null references assessments(id) on delete cascade,
  version_no integer not null,
  snapshot jsonb not null,
  content_hash text not null,
  published_by uuid references lecturers(id),
  created_at timestamptz not null default now(),
  unique (assessment_id, version_no)
);
do $$
begin
  if not exists (select 1 from pg_constraint where conname = 'assessments_published_version_fk') then
    alter table assessments
      add constraint assessments_published_version_fk
      foreign key (published_version_id) references assessment_versions(id)
      deferrable initially deferred;
  end if;
end $$;

-- 5. Per-student overrides (extra time, extra attempts, a later due date).
create table if not exists assessment_accommodations (
  assessment_id uuid not null references assessments(id) on delete cascade,
  student_account_id uuid not null references student_accounts(id) on delete cascade,
  extra_time_pct integer not null default 0 check (extra_time_pct between 0 and 300),
  extra_attempts integer not null default 0 check (extra_attempts between 0 and 10),
  due_override timestamptz,
  reason text not null default '',
  created_by uuid references lecturers(id),
  created_at timestamptz not null default now(),
  primary key (assessment_id, student_account_id)
);

-- 6. Attempts. `deadline_at` is computed on the server when the attempt starts.
create table if not exists assessment_attempts (
  id uuid primary key default gen_random_uuid(),
  assessment_id uuid not null references assessments(id) on delete cascade,
  version_id uuid not null references assessment_versions(id),
  student_account_id uuid not null references student_accounts(id) on delete cascade,
  attempt_no integer not null,
  status text not null default 'in_progress'
    check (status in ('in_progress', 'submitted', 'auto_submitted', 'void', 'graded')),
  -- "Candidate"-style alias the grader sees instead of the student.
  grading_alias text not null,
  started_at timestamptz not null default now(),
  deadline_at timestamptz,
  submitted_at timestamptz,
  submitted_offline boolean not null default false,
  claimed_submit_at timestamptz,
  late boolean not null default false,
  question_order jsonb not null default '[]'::jsonb,
  option_orders jsonb not null default '{}'::jsonb,
  seed bigint not null default 0,
  device_id text,
  integrity_summary jsonb not null default '{}'::jsonb,
  final_score double precision,
  max_score double precision,
  receipt text,
  unique (assessment_id, student_account_id, attempt_no),
  unique (assessment_id, grading_alias)
);
create index if not exists assessment_attempts_student_idx
  on assessment_attempts(student_account_id, assessment_id);
-- At most one in-progress attempt per student per assessment.
create unique index if not exists assessment_attempts_one_open_idx
  on assessment_attempts(assessment_id, student_account_id) where status = 'in_progress';

create table if not exists attempt_answers (
  id uuid primary key default gen_random_uuid(),
  attempt_id uuid not null references assessment_attempts(id) on delete cascade,
  question_id uuid not null,
  response jsonb not null default '{}'::jsonb,
  revision integer not null default 0,
  client_op_id uuid,
  time_spent_ms integer not null default 0,
  flagged_for_review boolean not null default false,
  answered_at timestamptz not null default now(),
  current_grade_id uuid,
  unique (attempt_id, question_id)
);
-- Idempotency: the same client op never applies twice.
create unique index if not exists attempt_answers_op_idx
  on attempt_answers(client_op_id) where client_op_id is not null;

-- Applied ops, so a replayed batch is recognised even after a newer revision
-- replaced the stored `client_op_id`.
create table if not exists attempt_ops (
  client_op_id uuid primary key,
  attempt_id uuid not null references assessment_attempts(id) on delete cascade,
  applied_at timestamptz not null default now()
);

-- 7. Grades: append-only. The current grade of an answer is attempt_answers.current_grade_id.
create table if not exists answer_grades (
  id uuid primary key default gen_random_uuid(),
  answer_id uuid not null references attempt_answers(id) on delete cascade,
  source text not null check (source in ('auto', 'ai', 'lecturer')),
  status text not null check (status in ('proposed', 'confirmed', 'overridden', 'rejected')),
  score double precision not null,
  max_score double precision not null,
  feedback_md text not null default '',
  rubric_scores jsonb not null default '[]'::jsonb,
  confidence double precision,
  evidence jsonb not null default '[]'::jsonb,
  model text,
  prompt_version text,
  ai_job_id uuid,
  created_by uuid,
  created_at timestamptz not null default now()
);
create index if not exists answer_grades_answer_idx on answer_grades(answer_id, created_at desc);

-- 8. Integrity signals: flags for the lecturer to review, never auto-penalties.
create table if not exists assessment_integrity_events (
  id uuid primary key default gen_random_uuid(),
  attempt_id uuid not null references assessment_attempts(id) on delete cascade,
  kind text not null,
  at timestamptz not null default now(),
  client_at timestamptz,
  meta jsonb not null default '{}'::jsonb
);
create index if not exists assessment_integrity_attempt_idx
  on assessment_integrity_events(attempt_id, at);

-- 9. Transactional outbox, same pattern as material_events.
create table if not exists assessment_events (
  id uuid primary key default gen_random_uuid(),
  event_type text not null,
  payload jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  relayed_at timestamptz
);
create index if not exists assessment_events_pending_idx
  on assessment_events(created_at) where relayed_at is null;
