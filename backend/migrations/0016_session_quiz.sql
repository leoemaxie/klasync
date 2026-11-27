-- Self-study quizzes generated from a recorded lecture (distinct from live
-- polls, which are answered in the moment and have no archive).
create table if not exists session_quiz_questions (
  id uuid primary key default gen_random_uuid(),
  session_id uuid not null references lecture_sessions(id) on delete cascade,
  question text not null,
  options jsonb not null,
  correct_index integer not null,
  explanation text,
  topic_tag varchar(100),
  difficulty varchar(20) not null default 'medium',
  created_at timestamptz not null default now()
);
create index if not exists session_quiz_questions_session_idx
  on session_quiz_questions(session_id, created_at, id);
