# KLASYNC API

Run the complete desktop/API development host from the repository root:

```powershell
pnpm tauri dev
```

The Tauri host starts the API at `http://127.0.0.1:8787`. For dev, the current store is in memory and resets when the host exits. `migrations` is the PostgreSQL schema to apply in prod.

## Module layout

```text
src/
  api/
    handlers/       # small HTTP handlers grouped by domain
    error.rs        # consistent JSON error responses
    mod.rs          # router and local server startup
  models.rs         # API/domain payloads
  state.rs          # in-memory development store
  utils.rs          # small shared helpers
```

## Production configuration

Copy `.env.example` to `.env` and set `DATABASE_URL` plus a high-entropy `JWT_SECRET`. When `DATABASE_URL` is present, the server applies the SQL migrations at startup. The `/api/v1/auth/*` routes require both values; the guest-session development endpoints can still run without them.

`PASSWORD_RESET_OUTBOX_DIR` writes reset messages to local JSON files for development. Replace that adapter with a transactional email provider before public deployment. `OBJECT_STORAGE_DIR` is the local-development object-store root; a cloud adapter can later implement the same storage boundary.

`GET/POST /api/v1/courses` and `POST /api/v1/courses/:id/roster` now require `Authorization: Bearer <access-token>`. In PostgreSQL mode they persist data and enforce lecturer ownership; the client-supplied lecturer ID is ignored.

Session creation/end, participant lists, attendance summaries, and caption publishing also require a lecturer bearer token. Guest joins, caption reads, and attendance heartbeats remain account-free by design.

## Current HTTP contract

| Method | Endpoint | Purpose |
| --- | --- | --- |
| `GET` | `/health` | Service health check |
| `POST` | `/api/v1/lecturers/register` | Create a lecturer profile (development placeholder for full auth) |
| `POST` | `/api/v1/auth/lecturers/register` | Register a password-protected lecturer and issue tokens |
| `POST` | `/api/v1/auth/lecturers/login` | Authenticate a lecturer and issue tokens |
| `POST` | `/api/v1/auth/students/register` | Create a persistent student account and issue tokens |
| `POST` | `/api/v1/auth/students/login` | Authenticate a student account and issue tokens |
| `POST` | `/api/v1/auth/refresh` | Rotate an opaque refresh session and issue a new JWT |
| `POST` | `/api/v1/auth/logout` | Revoke a refresh session |
| `POST` | `/api/v1/auth/password-reset/request` | Write a one-time password reset message to the configured delivery outbox |
| `POST` | `/api/v1/auth/password-reset/complete` | Consume a reset token, change password, and revoke sessions |
| `POST` | `/api/v1/students/claims` | Claim a matching guest participation record |
| `GET` | `/api/v1/students/archive` | List persistent resources granted to the authenticated student |
| `GET, POST` | `/api/v1/courses` | List or create courses |
| `POST` | `/api/v1/courses/:id/roster` | Save parsed roster records |
| `POST` | `/api/v1/courses/:id/roster/import` | Import and validate a CSV/XLSX roster file |
| `POST` | `/api/v1/sessions` | Create a live session and return link, QR payload, and short code |
| `GET` | `/api/v1/sessions/code/:code` | Resolve a short code |
| `POST` | `/api/v1/sessions/code/:code/join` | Guest join with a matric number |
| `GET` | `/api/v1/sessions/code/:code/participants` | List attendance records for a session |
| `GET` | `/api/v1/sessions/code/:code/attendance` | Return participant, verification, and heartbeat totals |
| `POST` | `/api/v1/sessions/code/:code/end` | End a session and reject later joins/heartbeats |
| `GET, POST` | `/api/v1/sessions/code/:code/captions` | Retrieve or publish live caption chunks |
| `GET` | `/api/v1/sessions/code/:code/captions/ws` | Receive new caption chunks through WebSocket |
| `POST` | `/api/v1/sessions/code/:code/resources` | Create a lecture archive resource as the session lecturer |
| `GET` | `/api/v1/sessions/code/:code/invite/qr.svg` | Generate an SVG QR invite for the session lecturer |
| `POST` | `/api/v1/participants/:id/heartbeat` | Record active presence |

Duplicate joins for one matric number are idempotent, and roster matches are returned as `verified`; unmatched guests are `provisional`.

## Production work still required

Password-reset delivery, student email verification, object storage, audio/transcription services, AI study jobs, and deployment operations are still pending. Caption polling remains supported as a compatibility fallback alongside the WebSocket stream.
