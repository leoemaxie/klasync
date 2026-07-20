# KLASYNC API

Run the development API from this folder:

```powershell
cargo run
```

It listens at `http://127.0.0.1:8787`. The current store is in memory and resets when the server stops. `migrations/0001_initial.sql` is the PostgreSQL schema to apply when persistence is introduced.

## Current HTTP contract

| Method | Endpoint | Purpose |
| --- | --- | --- |
| `GET` | `/health` | Service health check |
| `POST` | `/api/v1/lecturers/register` | Create a lecturer profile (development placeholder for full auth) |
| `GET, POST` | `/api/v1/courses` | List or create courses |
| `POST` | `/api/v1/courses/:id/roster` | Save parsed roster records |
| `POST` | `/api/v1/sessions` | Create a live session and return link, QR payload, and short code |
| `GET` | `/api/v1/sessions/code/:code` | Resolve a short code |
| `POST` | `/api/v1/sessions/code/:code/join` | Guest join with a matric number |
| `POST` | `/api/v1/participants/:id/heartbeat` | Record active presence |

Roster file parsing, password authentication, student account claims, and WebSocket captions are intentionally next steps. The API contract already preserves the guest-first join path and returns `verified` only when a submitted matric number matches the uploaded course roster.
