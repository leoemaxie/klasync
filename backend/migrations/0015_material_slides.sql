-- Lecturer-supplied slides / notes (plain text, extracted from a .pptx or
-- pasted) used as extra context when drafting the handout.
alter table lecture_materials add column if not exists slides_text text;
alter table lecture_materials add column if not exists slides_name text;
