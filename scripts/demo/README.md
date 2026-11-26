# Demo recording

Scripts that record and edit the KLASYNC product demo: a lecturer on desktop
running a live lecture, students on phones, and the study archive afterwards.
Everything runs against the real app and backend with live Gemini; nothing is
mocked in the UI.

## What it does

| Step | Script | Output |
| --- | --- | --- |
| Voices | `tts.py lecture` / `tts.py narration` | `lecture48.wav` (the lecturer, played into Chrome's fake microphone), `out/narr/*.wav` (narrator, Nigerian English accent) |
| Reset | `reset.sh` | Fresh database, backend and seeded course (roster, past lectures, attendance history) |
| Live lecture | `live.cjs` | `out/live_desktop` + `out/live_phone` videos, `out/live_marks.json` |
| Study archive | `study.cjs` | `out/study_phone` video, `out/study_marks.json` (waits for the post-lecture AI jobs) |
| Edit | `compose.py` | `out/klasync_demo.mp4` (1080p, narrated, sped-up stretches) |

The lecturer plays Gemini TTS audio through a fake microphone, so the captions,
rolling summaries, live quiz, chapters, flashcards, concept map, study quiz and
Ask answers are all generated live. Ten of the eleven attendees are simulated
through the API (joins and heartbeats); the phone is a real student session.

## Prerequisites

- Backend built (`cargo build` in `backend/`) and an env file for it
  (`DEMO_BACKEND_ENV`, default `/tmp/backend.env`) pointing at Postgres with
  pgvector, TLS Redis (`scripts/gen-dev-certs.sh`), an S3-compatible store
  (R2 settings) and Gemini credentials. See `docker-compose.yml` for the shape.
- Frontend dev server on `http://127.0.0.1:1420` (`pnpm dev`) with the backend
  allowing that origin in `CORS_ALLOWED_ORIGINS`.
- `node` with `playwright` (Chromium), `ffmpeg` (with drawtext), `psql`,
  `redis-cli`, `python3`.
- `GEMINI_API_KEY` for `tts.py` unless your environment injects credentials.

## Run it

```sh
cd scripts/demo
python3 tts.py lecture            # once; commits nothing (outputs are gitignored)
python3 tts.py narration
./reset.sh
node live.cjs                      # about 3.5 minutes
node study.cjs                     # waits for chapters/cards/quiz/map, then ~100 s
python3 compose.py                # -> out/klasync_demo.mp4
```

Edit `narration.json` to change the narration (the scene lengths follow it) and
the `segs` list in `compose.py` to change the cut. Post-lecture jobs are queued
roughly 2-4 minutes after a session ends, so `study.cjs` can wait a while.

## Notes

- The join endpoint is rate limited to 30 per minute per IP; `reset.sh` clears
  the counters after seeding.
- Narration uses `gemini-3.1-flash-tts-preview`; `tts.py` retries lines that
  trail off or are falsely blocked, using simpler accent wording.
- Voices and the accent come from prompting the TTS model. They are not
  verified by ear in CI.
