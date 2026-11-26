#!/bin/sh
# Fresh database + backend + seed data for the demo recording.
# Needs: Postgres (pgvector), TLS Redis, S3-compatible store and a built backend
# (cargo build). DEMO_BACKEND_ENV points at the env file the backend runs with.
HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(git -C "$HERE" rev-parse --show-toplevel)"
ENV_FILE="${DEMO_BACKEND_ENV:-/tmp/backend.env}"
DB_ADMIN="${DEMO_DB_ADMIN:-postgres://klasync:klasync-dev@127.0.0.1:5432/postgres}"
REDIS="redis-cli --tls --cacert $REPO/deploy/certs/ca.crt -p ${DEMO_REDIS_PORT:-6380}"
pkill klasync-backend; sleep 1
psql "$DB_ADMIN" -qc "drop database if exists klasync with (force)" -c "create database klasync owner klasync"
$REDIS flushall >/dev/null
(cd "$REPO/backend" && set -a && . "$ENV_FILE" && set +a && nohup ./target/debug/klasync-backend > /tmp/backend.log 2>&1 &)
for i in $(seq 1 40); do curl -s -o /dev/null localhost:8787/api/v1/courses && break; sleep 1; done
python3 "$HERE/seed.py"
# Clear the rate-limit counters the seed just used (join is 30/min per IP).
$REDIS flushall >/dev/null
