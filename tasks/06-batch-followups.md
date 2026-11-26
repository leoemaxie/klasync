# 06 - Batch mode follow-ups and stuck-generation watchdog

**Why.** `MATERIALS_BATCH=1` batches only section writing. While testing,
generation also sat in `generating` for minutes when the AI provider was rate
limiting, with nothing telling the lecturer.

**Build.**
- Record the real model for batch answers (`note_model` currently gets
  `"batch"`; use the model from the batch output).
- Optionally batch the post-stage generators (summary, review questions,
  glossary) too; keep outline and assembly interactive.
- A watchdog: if a generation has been `generating` longer than N minutes with
  no job making progress, mark it `failed` with a readable reason, or fall back
  to transcript sections, so the UI never spins forever. Surface "waiting on the
  AI provider (rate limited)" in the panel.
- Show "batch" vs "live" for the run in the panel (batch can take hours).

**Done when.**
- A simulated rate limit ends in a clear state within the watchdog window.
- `gen_state.models` lists the real model for batched sections.
- Tests for the watchdog and the batch result path (`batch.rs`).

**Start at.** `backend/src/batch.rs`, `backend/src/materials/generate.rs`
(`finish_section_job`, `note_model`), `LectureMaterialPanel.svelte`.
