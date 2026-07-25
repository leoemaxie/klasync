-- Production identity, ownership, archive, and retention model.

create type student_account_status as enum ('pending_verification', 'active', 'suspended');
create table student_accounts (
  id uuid primary key default gen_random_uuid(),
  matric_number text not null unique,
  email text not null unique,
  display_name text not null,
  password_hash text not null,
  status student_account_status not null default 'pending_verification',
  verified_at timestamptz,
  created_at timestamptz not null default now()
);

create type account_role as enum ('lecturer', 'student');
create table auth_sessions (
  id uuid primary key default gen_random_uuid(),
  account_id uuid not null,
  account_role account_role not null,
  refresh_token_hash text not null unique,
  expires_at timestamptz not null,
  revoked_at timestamptz,
  created_at timestamptz not null default now()
);

create table password_reset_tokens (
  id uuid primary key default gen_random_uuid(),
  account_id uuid not null,
  account_role account_role not null,
  token_hash text not null unique,
  expires_at timestamptz not null,
  used_at timestamptz,
  created_at timestamptz not null default now()
);

alter table lecture_sessions
  add column if not exists allow_late_join boolean not null default true,
  add column if not exists guest_access_expires_at timestamptz,
  add column if not exists lecturer_id uuid references lecturers(id),
  add column if not exists transcript_status text not null default 'pending';

update lecture_sessions s
set lecturer_id = c.lecturer_id
from courses c
where s.course_id = c.id and s.lecturer_id is null;

alter table lecture_sessions alter column lecturer_id set not null;

create table session_invites (
  id uuid primary key default gen_random_uuid(),
  session_id uuid not null references lecture_sessions(id) on delete cascade,
  token uuid not null unique default gen_random_uuid(),
  short_code text not null unique,
  created_by uuid not null references lecturers(id),
  expires_at timestamptz,
  revoked_at timestamptz,
  created_at timestamptz not null default now()
);

create table caption_chunks (
  id uuid primary key default gen_random_uuid(),
  session_id uuid not null references lecture_sessions(id) on delete cascade,
  sequence_number bigint not null,
  text text not null check (length(trim(text)) > 0),
  is_final boolean not null default true,
  created_at timestamptz not null default now(),
  unique(session_id, sequence_number)
);

create table attendance_events (
  id uuid primary key default gen_random_uuid(),
  participant_id uuid not null references session_participants(id) on delete cascade,
  event_type text not null check (event_type in ('joined', 'heartbeat', 'left', 'flagged', 'approved', 'rejected')),
  occurred_at timestamptz not null default now(),
  metadata jsonb not null default '{}'::jsonb
);

alter table session_participants
  add column if not exists attendance_score numeric(5,2) not null default 0,
  add column if not exists reviewed_by uuid references lecturers(id),
  add column if not exists reviewed_at timestamptz,
  add column if not exists review_note text,
  add constraint session_participants_unique_identity unique(session_id, matric_number);

create table lecture_resources (
  id uuid primary key default gen_random_uuid(),
  session_id uuid not null references lecture_sessions(id) on delete cascade,
  resource_type text not null check (resource_type in ('audio', 'recording', 'transcript', 'summary', 'flashcards', 'notes')),
  storage_key text,
  content jsonb,
  checksum text,
  created_at timestamptz not null default now(),
  expires_at timestamptz
);

create table resource_access_grants (
  id uuid primary key default gen_random_uuid(),
  resource_id uuid not null references lecture_resources(id) on delete cascade,
  student_account_id uuid not null references student_accounts(id) on delete cascade,
  granted_at timestamptz not null default now(),
  unique(resource_id, student_account_id)
);

create table student_session_claims (
  id uuid primary key default gen_random_uuid(),
  participant_id uuid not null unique references session_participants(id) on delete cascade,
  student_account_id uuid not null references student_accounts(id) on delete cascade,
  verified_at timestamptz,
  created_at timestamptz not null default now()
);

create index lecture_sessions_lecturer_id_idx on lecture_sessions(lecturer_id);
create index caption_chunks_session_created_idx on caption_chunks(session_id, created_at);
create index attendance_events_participant_idx on attendance_events(participant_id, occurred_at);
create index lecture_resources_session_idx on lecture_resources(session_id, resource_type);
