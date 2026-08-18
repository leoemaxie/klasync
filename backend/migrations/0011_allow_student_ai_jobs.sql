-- Allow students and guest participants to request AI study jobs (flashcards, chapters)
alter table ai_jobs drop constraint if exists ai_jobs_requested_by_fkey;
