alter table session_participants
  add column if not exists attendance_score numeric(5, 2),
  add column if not exists duplicate_flag boolean not null default false,
  add column if not exists reviewed_at timestamptz,
  add column if not exists reviewed_by uuid references lecturers(id),
  add column if not exists review_note text;

create index if not exists session_participants_duplicate_idx
  on session_participants(session_id, matric_number, duplicate_flag);
