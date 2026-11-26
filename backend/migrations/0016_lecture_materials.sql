-- Lecture materials engine (see docs/lecture-materials-spec in the design doc).
--
-- One `lecture_materials` row per lecture session holds the working draft; every
-- AI draft and every publish is an immutable `material_versions` snapshot. A
-- course offering (`courses` row) owns one `course_books` row that composes the
-- published materials. Exports, deliveries, embeddings, comparisons and the
-- plug-in settings all hang off versions so new plug-ins need no schema change.

-- 1. New job kinds in the shared AI queue.
alter table ai_jobs drop constraint if exists ai_jobs_job_type_check;
alter table ai_jobs add constraint ai_jobs_job_type_check
  check (job_type in ('transcribe', 'summarize', 'flashcards', 'lecture_qa_index', 'explain',
                      'question_answer', 'chapters', 'visualization', 'quiz', 'embed',
                      'material_outline', 'material_section', 'material_assemble', 'material_embed'));

-- 2. Per-course generation settings (audience level, depth, language, tone, template).
create table if not exists course_material_settings (
  course_id uuid primary key references courses(id) on delete cascade,
  auto_generate boolean not null default true,
  audience_level text not null default 'undergraduate',
  depth text not null default 'standard' check (depth in ('concise', 'standard', 'detailed')),
  language text not null default 'English',
  tone text not null default 'academic',
  template text not null default 'default',
  seed_from_previous boolean not null default true,
  updated_at timestamptz not null default now()
);

-- 3. The material (one per lecture session) and its immutable versions.
create table if not exists lecture_materials (
  id uuid primary key default gen_random_uuid(),
  session_id uuid not null unique references lecture_sessions(id) on delete cascade,
  course_id uuid not null references courses(id) on delete cascade,
  lecturer_id uuid not null references lecturers(id),
  status text not null default 'draft'
    check (status in ('draft', 'in_review', 'published', 'archived')),
  template text not null default 'default',
  language text not null default 'English',
  -- Working draft: the block document the lecturer edits. Versions are cut on
  -- AI generation and on publish only.
  draft_doc jsonb,
  draft_rev integer not null default 0,
  current_version_id uuid,
  published_version_id uuid,
  -- Generation bookkeeping. `generation` advances on every (re)run so that
  -- retries and double triggers are idempotent per generation.
  generation integer not null default 0,
  generation_state text not null default 'idle'
    check (generation_state in ('idle', 'queued', 'generating', 'ready', 'failed')),
  generation_error text,
  gen_state jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);
create index if not exists lecture_materials_course_idx on lecture_materials(course_id, status);

create table if not exists material_versions (
  id uuid primary key default gen_random_uuid(),
  material_id uuid not null references lecture_materials(id) on delete cascade,
  version_no integer not null,
  origin text not null check (origin in ('ai', 'lecturer')),
  doc jsonb not null,
  model text,
  prompt_version text,
  input_hash text,
  published_at timestamptz,
  created_at timestamptz not null default now(),
  unique (material_id, version_no)
);
create index if not exists material_versions_material_idx on material_versions(material_id, version_no desc);

-- 4. Course books: an ordered composition of published materials.
create table if not exists course_books (
  id uuid primary key default gen_random_uuid(),
  course_id uuid not null unique references courses(id) on delete cascade,
  title text not null,
  front_matter jsonb not null default '{}'::jsonb,
  -- Ordered units: [{"title": "...", "material_ids": ["..."]}]. Empty = all
  -- published materials by lecture date, as one unit per lecture.
  structure jsonb not null default '[]'::jsonb,
  stale boolean not null default true,
  chapter_cache jsonb not null default '{}'::jsonb,
  published_version_id uuid,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists book_versions (
  id uuid primary key default gen_random_uuid(),
  book_id uuid not null references course_books(id) on delete cascade,
  version_no integer not null,
  doc jsonb not null,
  -- Exact material versions this book contains: [{"material_id", "version_id"}].
  pins jsonb not null default '[]'::jsonb,
  published_at timestamptz,
  created_at timestamptz not null default now(),
  unique (book_id, version_no)
);

-- 5. Rendered files, cached per version, format and template.
create table if not exists material_exports (
  id uuid primary key default gen_random_uuid(),
  version_id uuid references material_versions(id) on delete cascade,
  book_version_id uuid references book_versions(id) on delete cascade,
  format text not null,
  template text not null,
  storage_key text not null,
  content_type text not null,
  byte_size bigint not null,
  checksum text not null,
  created_at timestamptz not null default now(),
  check ((version_id is not null) <> (book_version_id is not null))
);
create unique index if not exists material_exports_version_uq
  on material_exports(version_id, format, template) where version_id is not null;
create unique index if not exists material_exports_book_uq
  on material_exports(book_version_id, format, template) where book_version_id is not null;

-- 6. Who received what.
create table if not exists material_deliveries (
  id uuid primary key default gen_random_uuid(),
  version_id uuid references material_versions(id) on delete cascade,
  book_version_id uuid references book_versions(id) on delete cascade,
  channel text not null,
  audience jsonb not null default '{}'::jsonb,
  status text not null default 'queued'
    check (status in ('queued', 'sending', 'sent', 'partial', 'failed')),
  recipients integer not null default 0,
  delivered integer not null default 0,
  error text,
  created_by uuid references lecturers(id),
  created_at timestamptz not null default now(),
  sent_at timestamptz,
  check ((version_id is not null) <> (book_version_id is not null))
);
create index if not exists material_deliveries_status_idx on material_deliveries(status, created_at);

-- 7. Section vectors for comparison and reuse (1536 dimensions, cosine).
create table if not exists material_section_embeddings (
  id uuid primary key default gen_random_uuid(),
  version_id uuid not null references material_versions(id) on delete cascade,
  block_id text not null,
  heading text not null,
  body text not null,
  embedding vector(1536) not null,
  model text not null,
  unique (version_id, block_id, model)
);

-- 8. Comparison reports between two course offerings.
create table if not exists material_comparisons (
  id uuid primary key default gen_random_uuid(),
  course_a uuid not null references courses(id) on delete cascade,
  course_b uuid not null references courses(id) on delete cascade,
  report jsonb not null,
  created_by uuid references lecturers(id),
  created_at timestamptz not null default now()
);
create index if not exists material_comparisons_courses_idx on material_comparisons(course_a, course_b, created_at desc);

-- 9. Plug-in settings: institution-wide ("global") or per course. The registry
--    reads these, so enabling a plug-in never needs a deploy.
create table if not exists material_plugin_settings (
  id uuid primary key default gen_random_uuid(),
  scope text not null check (scope in ('global', 'course')),
  course_id uuid references courses(id) on delete cascade,
  kind text not null check (kind in ('generator', 'renderer', 'delivery', 'analyzer', 'template')),
  plugin_id text not null,
  enabled boolean not null default true,
  config jsonb not null default '{}'::jsonb,
  updated_at timestamptz not null default now(),
  check ((scope = 'global') = (course_id is null))
);
create unique index if not exists material_plugin_settings_global_uq
  on material_plugin_settings(kind, plugin_id) where scope = 'global';
create unique index if not exists material_plugin_settings_course_uq
  on material_plugin_settings(course_id, kind, plugin_id) where scope = 'course';

-- 10. Transactional outbox: written in the same transaction as the change and
--     relayed to Redis and to registered plug-ins, so plug-ins react to events
--     without being called by the core.
create table if not exists material_events (
  id bigserial primary key,
  event_type text not null,
  payload jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  relayed_at timestamptz
);
create index if not exists material_events_pending_idx on material_events(id) where relayed_at is null;

-- 11. Published materials surface through the existing `notes` resource.
alter table lecture_resources add column if not exists material_version_id uuid;
alter table lecture_resources add column if not exists original_filename text;
alter table lecture_resources add column if not exists content_type text;
alter table lecture_resources add column if not exists byte_size bigint;
create index if not exists lecture_resources_material_idx
  on lecture_resources(material_version_id) where material_version_id is not null;
