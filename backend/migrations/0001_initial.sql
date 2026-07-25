-- KLASYNC data model (PostgreSQL)
create extension if not exists pgcrypto;

create table lecturers (
  id uuid primary key default gen_random_uuid(),
  name text not null,
  email text not null unique,
  password_hash text not null,
  created_at timestamptz not null default now()
);

create table courses (
  id uuid primary key default gen_random_uuid(),
  lecturer_id uuid not null references lecturers(id),
  code text not null,
  title text not null,
  created_at timestamptz not null default now()
);

create table roster_students (
  id uuid primary key default gen_random_uuid(),
  course_id uuid not null references courses(id) on delete cascade,
  matric_number text not null,
  full_name text not null,
  email text,
  unique(course_id, matric_number)
);

create type session_status as enum ('scheduled', 'live', 'ended');
create table lecture_sessions (
  id uuid primary key default gen_random_uuid(),
  course_id uuid not null references courses(id),
  title text not null,
  short_code text not null unique,
  invite_token uuid not null unique default gen_random_uuid(),
  status session_status not null default 'scheduled',
  started_at timestamptz,
  ended_at timestamptz
);

create type verification_status as enum ('verified', 'provisional', 'flagged', 'approved', 'rejected');
create table session_participants (
  id uuid primary key default gen_random_uuid(),
  session_id uuid not null references lecture_sessions(id) on delete cascade,
  roster_student_id uuid references roster_students(id),
  student_account_id uuid,
  matric_number text not null,
  display_name text not null,
  verification_status verification_status not null default 'provisional',
  joined_at timestamptz not null default now(),
  last_seen_at timestamptz not null default now(),
  heartbeat_count integer not null default 0
);

create index session_participants_session_id_idx on session_participants(session_id);
create index session_participants_matric_number_idx on session_participants(matric_number);
