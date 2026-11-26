# Klasync Backend API

> Standalone Axum API server for session orchestration, authentication, live captions, and persistence.

The Klasync backend is built with Rust and Axum. It handles lecturer authentication, course rosters, live session coordination, WebSocket streaming for captions, and PostgreSQL storage.

---

## Tech Stack

- **Framework**: [Axum](https://github.com/tokio-rs/axum) (Tokio)
- **Database**: PostgreSQL with [SQLx](https://github.com/launchbadge/sqlx)
- **Cache & Presence**: Redis
- **LLM Provider**: OpenRouter 
- **Object Storage**: AWS S3 / Cloudflare R2 compatible

---

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/) (2021 edition)
- [PostgreSQL](https://www.postgresql.org/)
- [Redis](https://redis.io/)

### Setup & Run

1. **Configure environment variables:**
   ```bash
   cp .env.example .env
   # Edit .env with your local PostgreSQL and Redis connection strings
   ```

2. **Run migrations:**
   ```bash
   # If sqlx-cli is installed
   cargo sqlx migrate run
   ```

3. **Start the development server:**
   ```bash
   cargo run
   ```

The server listens on `http://127.0.0.1:8787` by default.

## Running the stack without Docker

`docker compose up` is the supported route (see the root `docker-compose.yml`).
Where Docker is unavailable (a CI container, a sandbox) the same stack runs
natively. Postgres needs the `pgvector` extension and the `pg_cron` stub from
`deploy/postgres/`; Redis must speak TLS; any S3-compatible server works for
the object store.

```bash
# Postgres 16 + pgvector (Debian/Ubuntu: apt install postgresql-16 postgresql-16-pgvector)
cp deploy/postgres/pg_cron.control deploy/postgres/pg_cron--1.0.sql /usr/share/postgresql/16/extension/
initdb -D "$PGDATA" && pg_ctl -D "$PGDATA" start
psql -c "CREATE ROLE klasync LOGIN SUPERUSER PASSWORD 'klasync-dev'" -c "CREATE DATABASE klasync OWNER klasync"

# Redis over TLS, using the throwaway CA from scripts/gen-dev-certs.sh
./scripts/gen-dev-certs.sh
redis-server --port 0 --tls-port 6380 --tls-cert-file deploy/certs/redis.crt \
  --tls-key-file deploy/certs/redis.key --tls-ca-cert-file deploy/certs/ca.crt --tls-auth-clients no

# S3 mock (any S3-compatible server), then create the bucket
pip install "moto[server]" && moto_server -p 9090 &
curl -X PUT http://127.0.0.1:9090/klasync-lectures
```

Then run the backend with the variables from the `backend` service in
`docker-compose.yml`, pointing `DATABASE_URL`, `REDIS_URL=rediss://127.0.0.1:6380`
and `R2_ENDPOINT=http://127.0.0.1:9090` at these, and `SSL_CERT_FILE` /
`HTTP_EXTRA_CA_FILE` at a bundle containing the dev CA. With no AI key the
backend runs with AI disabled: chapters and concept maps fall back to
heuristics, and flashcards, Ask and semantic search report that they are
unavailable. `python3 scripts/e2e.py` exercises the whole lifecycle.

## AI architecture

All AI goes through one interface, `AiGateway` (`src/ai/`). Call sites say *what*
they need (a task, a tier, an input); the gateway picks the provider and model,
retries transient failures, falls back when a model is retired or errors,
**escalates** to the next model when a cheap model's output fails validation,
prices every call and records it in `ai_usage_log`.

| Layer | File | Knows about vendors? |
|---|---|---|
| Vocabulary (tiers, tasks, usage, errors) | `ai/types.rs` | no |
| Capability traits (`TextProvider`, `Transcriber`, `Embedder`, `BatchProvider`) | `ai/provider.rs` | no |
| Gemini (native `generateContent`, Files API, `embedContent`, `batchGenerateContent`) | `ai/gemini.rs` | yes |
| OpenRouter (OpenAI-compatible) | `ai/openrouter.rs` | yes |
| Routing, retries, escalation, budget, usage log | `ai/gateway.rs` | no |
| Price table (with scheduled price changes) | `ai/pricing.rs` | model names only |

**Provider selection** happens once at startup: a `GEMINI_API_KEY` (or
`GOOGLE_API_KEY`) routes to Gemini; otherwise OpenRouter. With both, OpenRouter
is a last-resort text fallback.

**Tiers**: *cheap* (rolling summaries, Q&A) and *mid* (flashcards, chapters,
concept maps, quizzes) each have an ordered model chain in `GEMINI_MODELS_*`;
*strong* is never used unless a call asks for it. Long transcripts that exceed a
provider's context are condensed section by section on the cheap tier first.

**Batch**: when a lecture ends, chapters/flashcards/concept map are queued as
`mode = 'batch'` jobs (about half price). A student opening the archive sooner
promotes the job to run immediately. Failed, expired or unusable batch results
fall back to interactive jobs, so a lecture never ends up without study material.

**Search**: lectures are chunked, embedded (`gemini-embedding-2`, 1536 dims) into
`lecture_chunks` (pgvector). `POST /archive/sessions/{id}/ask` answers from the
retrieved passages; `GET /students/search?q=` searches every lecture a student
can access. Lectures that predate the index are backfilled automatically.

**Idle behaviour (scale-to-zero databases)**: background workers only query the
database while the service has recent traffic, a batch is pending, or shortly
after a restart; the first request after an idle period wakes them. Health
probes (`/health*`) do not count as traffic. Point load-balancer health checks
at `/health` (no database) and not `/health/ready`, or the database never sleeps.

## Local stack (mirrors production)

```bash
./scripts/gen-dev-certs.sh                 # one-off: CA + cert for TLS Redis
GEMINI_API_KEY=... docker compose up -d --build
python3 scripts/e2e.py                     # full lifecycle against the stack
```

Postgres (pgvector), TLS Redis, S3-compatible storage (standing in for R2) and the
backend. `scripts/e2e.py` drives auth, sessions, live captions, rolling summaries,
polls, audio recording and transcription, post-lecture batch jobs, flashcards,
chapters, concept maps, embeddings, search, access control and cost accounting.
Live Gemini smoke tests: `GEMINI_API_KEY=... cargo test --test gemini_live -- --ignored`.
The compose Postgres ships a stub `pg_cron` so the production migration history
(which creates that extension) applies unchanged.
