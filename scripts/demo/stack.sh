#!/bin/sh
# Starts the supporting services for the demo without Docker: Postgres (pgvector),
# TLS Redis, an S3-compatible store (moto) and the frontend dev server.
# Assumes a one-time setup (see README): postgresql-16 + pgvector installed, a
# data directory initialised at $PGDATA owned by `postgres`, and a `klasync` role.
HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(git -C "$HERE" rev-parse --show-toplevel)"
PGDATA="${PGDATA:-/tmp/pgdata}"
su postgres -c "/usr/lib/postgresql/16/bin/pg_ctl -D $PGDATA -o '-p 5432 -c listen_addresses=127.0.0.1' -l /tmp/pg.log -w start" >/dev/null 2>&1 || true
nohup redis-server --port 0 --tls-port "${DEMO_REDIS_PORT:-6380}" \
  --tls-cert-file "$REPO/deploy/certs/redis.crt" --tls-key-file "$REPO/deploy/certs/redis.key" \
  --tls-ca-cert-file "$REPO/deploy/certs/ca.crt" --tls-auth-clients no > /tmp/redis.log 2>&1 &
nohup moto_server -p 9090 > /tmp/moto.log 2>&1 &
(cd "$REPO" && nohup pnpm dev --host 127.0.0.1 > /tmp/vite.log 2>&1 &)
sleep 4
python3 - <<'PY'
import boto3
s = boto3.client('s3', endpoint_url='http://127.0.0.1:9090', aws_access_key_id='klasync', aws_secret_access_key='klasync-dev-secret', region_name='us-east-1')
try: s.create_bucket(Bucket='klasync-lectures')
except Exception as e: print('bucket:', e)
PY
echo "stack up: postgres :5432, redis :${DEMO_REDIS_PORT:-6380}, s3 :9090, vite :1420"
