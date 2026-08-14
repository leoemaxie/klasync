-- Migration: Academic Session & Calendar Awareness for Course Offerings and Multi-Course Student Enrollment

-- 1. Add academic session, semester, and active status to courses
alter table courses
  add column if not exists academic_session text,
  add column if not exists semester text,
  add column if not exists is_active boolean not null default true;

-- 2. Backfill existing rows, assigning distinct historical academic sessions to any existing duplicate codes
with ranked as (
  select id, row_number() over (
    partition by lecturer_id, lower(trim(code))
    order by created_at desc
  ) as rn
  from courses
  where academic_session is null
)
update courses c
set
  academic_session = case
    when r.rn = 1 then '2025/2026'
    else format('202%s/202%s (Archived #%s)', 5 - r.rn + 1, 6 - r.rn + 1, r.rn - 1)
  end,
  semester = coalesce(c.semester, 'Second Semester')
from ranked r
where c.id = r.id;

-- Fallback for any remaining nulls
update courses set academic_session = '2025/2026' where academic_session is null;
update courses set semester = 'Second Semester' where semester is null;

-- 3. Make columns strictly required
alter table courses alter column academic_session set not null;
alter table courses alter column semester set not null;

-- 4. Composite unique constraint allowing a lecturer to teach the same course across multiple sessions/semesters
create unique index if not exists courses_lecturer_offering_idx on courses(lecturer_id, lower(trim(code)), academic_session, semester);

-- 5. Student multi-course enrollments table
create table if not exists student_course_enrollments (
  id uuid primary key default gen_random_uuid(),
  student_account_id uuid not null references student_accounts(id) on delete cascade,
  course_id uuid not null references courses(id) on delete cascade,
  enrollment_type text not null default 'claimed' check (enrollment_type in ('claimed', 'direct', 'roster_matched')),
  enrolled_at timestamptz not null default now(),
  unique(student_account_id, course_id)
);

create index if not exists idx_student_enrollments_student on student_course_enrollments(student_account_id);
create index if not exists idx_student_enrollments_course on student_course_enrollments(course_id);
create index if not exists idx_courses_academic_session on courses(academic_session, semester);
