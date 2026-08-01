-- Migration: 0009_safe_pg_cron_session_pruning.sql
-- Safely ensure pg_cron session pruning job is scheduled if extension permissions permit

DO '
BEGIN
    CREATE EXTENSION IF NOT EXISTS pg_cron;
    
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = ''pg_cron'') THEN
        PERFORM cron.schedule(
            ''prune_expired_auth_sessions'',
            ''0 0 1 * *'',
            ''DELETE FROM auth_sessions WHERE expires_at <= NOW() OR (revoked_at IS NOT NULL AND revoked_at < NOW() - INTERVAL ''''30 days'''')''
        );
    END IF;
EXCEPTION
    WHEN OTHERS THEN
        RAISE NOTICE ''pg_cron extension could not be installed on database %: %'', current_database(), SQLERRM;
END;
';
