-- Persist provider/model usage so cost controls remain auditable after restart.
alter table ai_jobs
  add column if not exists provider text,
  add column if not exists model text,
  add column if not exists input_tokens bigint,
  add column if not exists output_tokens bigint,
  add column if not exists cost_usd numeric(14, 8);

create index if not exists ai_jobs_provider_cost_idx
  on ai_jobs(provider, created_at);

alter table ai_jobs drop constraint if exists ai_jobs_job_type_check;
alter table ai_jobs add constraint ai_jobs_job_type_check
  check (job_type in ('transcribe', 'summarize', 'flashcards', 'lecture_qa_index', 'explain', 'question_answer'));
