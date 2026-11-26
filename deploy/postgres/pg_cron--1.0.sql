-- Minimal stand-in for pg_cron: records schedules, runs nothing. Production
-- (managed Postgres) provides the real extension; this keeps migration 0007
-- applicable on the plain pgvector image used by docker compose.
create table job (
  jobid bigserial primary key,
  jobname text unique,
  schedule text not null,
  command text not null
);

create function schedule(job_name text, schedule text, command text)
returns bigint language sql as $$
  insert into cron.job (jobname, schedule, command) values (job_name, schedule, command)
  on conflict (jobname) do update set schedule = excluded.schedule, command = excluded.command
  returning jobid;
$$;
