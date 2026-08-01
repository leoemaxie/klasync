-- Migration: 0007_pg_cron_session_pruning.sql
-- Configure pg_cron to prune expired and old revoked auth sessions every 30 days

CREATE EXTENSION IF NOT EXISTS pg_cron;

SELECT cron.schedule(
    'prune_expired_auth_sessions',
    '0 0 1 * *',
    $$ DELETE FROM auth_sessions WHERE expires_at <= NOW() OR (revoked_at IS NOT NULL AND revoked_at < NOW() - INTERVAL '30 days') $$
);
