create table if not exists session_questions (
  id uuid primary key default gen_random_uuid(),
  session_code varchar(8) not null,
  participant_id uuid references session_participants(id) on delete set null,
  caption_id uuid references caption_chunks(id) on delete set null,
  question_text text not null,
  upvote_count integer not null default 0,
  is_resolved boolean not null default false,
  created_at timestamptz not null default now()
);
create index if not exists session_questions_code_idx on session_questions(session_code, is_resolved, upvote_count desc);

create table if not exists session_chapters (
  id uuid primary key default gen_random_uuid(),
  session_id uuid not null references lecture_sessions(id) on delete cascade,
  chapter_index integer not null,
  title varchar(255) not null,
  summary text not null,
  start_timestamp_sec integer not null,
  end_timestamp_sec integer not null,
  created_at timestamptz not null default now(),
  unique(session_id, chapter_index)
);

create table if not exists session_flashcards (
  id uuid primary key default gen_random_uuid(),
  session_id uuid not null references lecture_sessions(id) on delete cascade,
  prompt text not null,
  answer text not null,
  topic_tag varchar(100),
  difficulty varchar(20) not null default 'medium',
  created_at timestamptz not null default now()
);

create table if not exists attendance_audit_logs (
  id uuid primary key default gen_random_uuid(),
  session_id uuid not null references lecture_sessions(id) on delete cascade,
  matric_number varchar(50) not null,
  anomaly_type varchar(50) not null,
  description text not null,
  severity varchar(20) not null default 'warning',
  logged_at timestamptz not null default now()
);
create index if not exists attendance_audit_session_idx on attendance_audit_logs(session_id, logged_at desc);

create table if not exists lms_course_sync (
  id uuid primary key default gen_random_uuid(),
  course_id uuid not null references courses(id) on delete cascade,
  lms_provider varchar(50) not null check (lms_provider in ('canvas', 'moodle', 'blackboard')),
  external_course_id varchar(100) not null,
  api_endpoint varchar(255) not null,
  last_synced_at timestamptz,
  auto_sync_roster boolean not null default true,
  created_at timestamptz not null default now(),
  unique(course_id, lms_provider)
);

alter table ai_jobs drop constraint if exists ai_jobs_job_type_check;
alter table ai_jobs add constraint ai_jobs_job_type_check
  check (job_type in ('transcribe', 'summarize', 'flashcards', 'lecture_qa_index', 'explain', 'question_answer', 'chapters'));
