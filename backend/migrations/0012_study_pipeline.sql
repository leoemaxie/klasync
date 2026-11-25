-- Student/archive matching, rolling summaries, live interactive content and
-- the visualization pipeline.

-- 1. New AI job kinds (rolling summaries and visualizations run through the
--    same queue so they get retries, cost tracking and failure messages).
alter table ai_jobs drop constraint if exists ai_jobs_job_type_check;
alter table ai_jobs add constraint ai_jobs_job_type_check
  check (job_type in ('transcribe', 'summarize', 'flashcards', 'lecture_qa_index', 'explain',
                      'question_answer', 'chapters', 'visualization', 'quiz'));

-- 2. Optional per-job parameters (e.g. flashcard topic) and requester identity.
alter table ai_jobs add column if not exists params jsonb not null default '{}'::jsonb;

-- 3. Audio recordings need a duration/format so the player can show metadata.
alter table lecture_resources add column if not exists duration_sec integer;

-- 4. Rolling summaries generated while a lecture is live.
create table if not exists session_rolling_summaries (
  id uuid primary key default gen_random_uuid(),
  session_id uuid not null references lecture_sessions(id) on delete cascade,
  sequence integer not null,
  summary text not null,
  key_points jsonb not null default '[]'::jsonb,
  covers_through_caption bigint not null default 0,
  source text not null default 'ai',
  created_at timestamptz not null default now(),
  unique(session_id, sequence)
);
create index if not exists session_rolling_summaries_session_idx
  on session_rolling_summaries(session_id, sequence desc);

-- 5. Live quizzes / polls / comprehension checks.
create table if not exists session_polls (
  id uuid primary key default gen_random_uuid(),
  session_id uuid not null references lecture_sessions(id) on delete cascade,
  kind text not null default 'poll' check (kind in ('poll', 'quiz', 'comprehension')),
  question text not null,
  options jsonb not null,
  correct_index integer,
  explanation text,
  status text not null default 'open' check (status in ('open', 'closed')),
  source text not null default 'lecturer',
  created_at timestamptz not null default now(),
  closed_at timestamptz
);
create index if not exists session_polls_session_idx on session_polls(session_id, created_at desc);

create table if not exists session_poll_responses (
  poll_id uuid not null references session_polls(id) on delete cascade,
  participant_id uuid not null references session_participants(id) on delete cascade,
  choice_index integer not null,
  responded_at timestamptz not null default now(),
  primary key (poll_id, participant_id)
);

-- 6. Hand raises (the UI already had the button; the endpoint never existed).
alter table session_participants add column if not exists hand_raised_at timestamptz;

-- 7. Visualization pipeline output (concept maps / timelines).
create table if not exists session_visualizations (
  id uuid primary key default gen_random_uuid(),
  session_id uuid not null references lecture_sessions(id) on delete cascade,
  kind text not null default 'concept_map' check (kind in ('concept_map', 'timeline')),
  title text not null,
  spec jsonb not null,
  source text not null default 'ai',
  created_at timestamptz not null default now()
);
create index if not exists session_visualizations_session_idx
  on session_visualizations(session_id, created_at desc);

-- 8. Student/session matching lookups used by archive reconciliation.
create index if not exists session_participants_matric_lower_idx
  on session_participants(lower(matric_number));
create index if not exists roster_students_matric_lower_idx
  on roster_students(lower(matric_number));
create index if not exists student_session_claims_student_idx
  on student_session_claims(student_account_id);
create index if not exists lecture_resources_session_type_idx
  on lecture_resources(session_id, resource_type, created_at desc);
