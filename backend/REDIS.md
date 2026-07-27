# Managed Redis

KLASYNC uses managed Redis only; there is no local Redis service. Configure a
TLS `rediss://` URL from the cloud provider.

Redis provides shared rate limits, participant presence TTLs, caption Pub/Sub,
AI job Streams, distributed locks, and short-lived idempotency reservations.
PostgreSQL remains the durable source of truth for accounts, sessions,
attendance, captions, resources, and AI job state.

AI jobs are published to a Redis Stream and consumed by a worker group. The
worker falls back to the PostgreSQL queue when Redis is degraded. A per-job
lock prevents manual dispatch and background workers from executing the same
job concurrently.

Set `REDIS_REQUIRED=true` after the managed service is provisioned and tested.
