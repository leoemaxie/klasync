-- Lecturer live controls and moderation state.
create table if not exists session_live_controls (
  session_id uuid primary key references lecture_sessions(id) on delete cascade,
  captions_paused boolean not null default false,
  audio_ingestion_active boolean not null default false,
  late_join_policy text not null default 'allowed' check (late_join_policy in ('allowed', 'roster_only', 'closed')),
  updated_by uuid references lecturers(id),
  updated_at timestamptz not null default now()
);

alter table session_participants
  add column if not exists muted_at timestamptz,
  add column if not exists removed_at timestamptz,
  add column if not exists removal_reason text;

alter table caption_chunks
  add column if not exists edited_at timestamptz,
  add column if not exists edited_by uuid references lecturers(id),
  add column if not exists is_hidden boolean not null default false,
  add column if not exists moderation_note text;

-- Session lifecycle metadata is additive so existing status values remain compatible.
alter table lecture_sessions
  add column if not exists scheduled_start_at timestamptz,
  add column if not exists timezone text not null default 'UTC',
  add column if not exists archived_at timestamptz,
  add column if not exists deleted_at timestamptz,
  add column if not exists reopen_count integer not null default 0;

-- One-time university-email claim verification. Codes are stored as hashes only.
create table if not exists student_claim_verifications (
  id uuid primary key default gen_random_uuid(),
  student_account_id uuid not null references student_accounts(id) on delete cascade,
  participant_id uuid not null references session_participants(id) on delete cascade,
  email text not null,
  code_hash text not null,
  attempts integer not null default 0,
  expires_at timestamptz not null,
  verified_at timestamptz,
  consumed_at timestamptz,
  created_at timestamptz not null default now()
);

create index if not exists student_claim_verifications_lookup_idx
  on student_claim_verifications(student_account_id, participant_id, expires_at);

create unique index if not exists student_claim_active_idx
  on student_claim_verifications(student_account_id, participant_id)
  where consumed_at is null and verified_at is null;
