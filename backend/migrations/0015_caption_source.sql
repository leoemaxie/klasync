-- Where a caption came from. The live audio socket transcribes speech on the
-- server and must not treat its own captions as proof that the lecturer's
-- browser is already captioning (which made it drop most of the lecture).
alter table caption_chunks add column if not exists source text not null default 'client';
