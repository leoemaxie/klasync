# 12 - Measure the effect of slides on output quality

**Why.** Slides are stored and injected into the outline and section prompts,
but nobody checked they improve the handout (or that a bad deck does not hurt
it).

**Build.**
- Pick 3-5 lectures with slides. Generate with and without them and compare:
  terminology match, section order, hallucinated content not in the recording
  (the grounding check flags this).
- Tune the prompt wording and the character windows (6,000 per section, 8,000 for
  the outline) from what you see.

**Done when.** A short write-up in this folder (or the README) with the findings
and any prompt changes, and a decision on the default windows.

**Start at.** `section_writer_input` and `run_outline` in
`backend/src/materials/generate.rs`, `backend/src/materials/slides.rs`.
