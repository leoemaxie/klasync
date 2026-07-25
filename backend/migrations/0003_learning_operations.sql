-- Operational features: attendance review, object metadata, AI jobs, and audit history.

create type ai_job_status as enum ('queued', 'running', 'completed', 'failed', 'cancelled');
create table ai_jobs (
  id uuid primary key default gen_random_uuid(),
  session_id uuid not null references lecture_sessions(id) on delete cascade,
  requested_by uuid not null references lecturers(id),
  job_type text not null check (job_type in ('transcribe', 'summarize', 'flashcards', 'lecture_qa_index')),
  status ai_job_status not null default 'queued',
  input_resource_id uuid references lecture_resources(id) on delete set null,
  output_resource_id uuid references lecture_resources(id) on delete set null,
  error_message text,
  attempts integer not null default 0,
  created_at timestamptz not null default now(),
  started_at timestamptz,
  completed_at timestamptz
);

create table session_audit_events (
  id uuid primary key default gen_random_uuid(),
  session_id uuid not null references lecture_sessions(id) on delete cascade,
  actor_id uuid,
  actor_role account_role,
  event_type text not null,
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now()
);

alter table lecture_resources
  add column if not exists original_filename text,
  add column if not exists content_type text,
  add column if not exists byte_size bigint;

create index ai_jobs_session_status_idx on ai_jobs(session_id, status, created_at);
create index session_audit_events_session_idx on session_audit_events(session_id, created_at);
