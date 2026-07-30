-- Migration: 0007_pg_cron_session_pruning.sql
-- Configure pg_cron to prune expired and old revoked auth sessions every 30 days if pg_cron is enabled for this database

DO $$
BEGIN
    -- Attempt to create pg_cron extension if supported on this database
    CREATE EXTENSION IF NOT EXISTS pg_cron;
    
    -- Schedule cleanup job if pg_cron extension is available
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pg_cron') THEN
        PERFORM cron.schedule(
            'prune_expired_auth_sessions',
            '0 0 1 * *',
            $$ DELETE FROM auth_sessions WHERE expires_at <= NOW() OR (revoked_at IS NOT NULL AND revoked_at < NOW() - INTERVAL '30 days') $$
        );
    END IF;
EXCEPTION
    WHEN OTHERS THEN
        RAISE NOTICE 'pg_cron extension could not be installed on database %: %', current_database(), SQLERRM;
END;
$$;
