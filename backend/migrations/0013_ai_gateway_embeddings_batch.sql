-- AI gateway accounting, pgvector embeddings, batch jobs and post-lecture prep.

create extension if not exists vector;

-- 1. New job kind: embedding a lecture for semantic search.
alter table ai_jobs drop constraint if exists ai_jobs_job_type_check;
alter table ai_jobs add constraint ai_jobs_job_type_check
  check (job_type in ('transcribe', 'summarize', 'flashcards', 'lecture_qa_index', 'explain',
                      'question_answer', 'chapters', 'visualization', 'quiz', 'embed'));

-- 2. Batch execution: discounted, asynchronous, used for after-lecture work.
create table if not exists ai_batches (
  id uuid primary key default gen_random_uuid(),
  provider text not null,
  provider_batch_id text not null,
  model text not null,
  status text not null default 'running' check (status in ('running', 'succeeded', 'failed')),
  request_count integer not null default 0,
  error text,
  created_at timestamptz not null default now(),
  last_polled_at timestamptz,
  completed_at timestamptz
);
create index if not exists ai_batches_status_idx on ai_batches(status, created_at);

alter table ai_jobs
  add column if not exists mode text not null default 'interactive'
    check (mode in ('interactive', 'batch')),
  add column if not exists batch_id uuid references ai_batches(id) on delete set null,
  add column if not exists run_after timestamptz not null default now();
create index if not exists ai_jobs_batch_queue_idx on ai_jobs(mode, status, run_after);

-- 3. Every model call, priced, so cost per feature is visible and a daily cap works.
create table if not exists ai_usage_log (
  id bigserial primary key,
  created_at timestamptz not null default now(),
  task text not null,
  tier text not null,
  provider text not null,
  model text not null,
  input_tokens bigint not null default 0,
  output_tokens bigint not null default 0,
  cached_tokens bigint not null default 0,
  thinking_tokens bigint not null default 0,
  audio_input_tokens bigint not null default 0,
  cost_usd numeric(14, 8) not null default 0,
  ok boolean not null default true,
  error text,
  via text not null default 'sync',
  session_id uuid,
  job_id uuid,
  duration_ms bigint not null default 0
);
create index if not exists ai_usage_log_created_idx on ai_usage_log(created_at);
create index if not exists ai_usage_log_task_idx on ai_usage_log(task, created_at);

-- 4. Lectures queued for automatic study-material generation after they end.
create table if not exists session_post_processing (
  session_id uuid primary key references lecture_sessions(id) on delete cascade,
  queued_at timestamptz not null default now()
);

-- 5. Embedded lecture passages (1536 dimensions, cosine distance).
create table if not exists lecture_chunks (
  id uuid primary key default gen_random_uuid(),
  session_id uuid not null references lecture_sessions(id) on delete cascade,
  chunk_index integer not null,
  start_sec integer not null default 0,
  end_sec integer not null default 0,
  text text not null,
  embedding vector(1536) not null,
  model text not null,
  created_at timestamptz not null default now(),
  unique (session_id, model, chunk_index)
);
create index if not exists lecture_chunks_session_idx on lecture_chunks(session_id, model);
create index if not exists lecture_chunks_embedding_idx
  on lecture_chunks using hnsw (embedding vector_cosine_ops);
