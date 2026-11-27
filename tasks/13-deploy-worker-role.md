# 13 - Deploy: run the worker role

**Why.** `KLASYNC_ROLE=api|worker|all` exists, but no deployment uses it.
Heavy generation still competes with the API for CPU in the default `all` mode.

**Build.**
- Add an optional `worker` service to `docker-compose.yml` (same image,
  `KLASYNC_ROLE=worker`, no port) and make the API service `api`.
- Document scaling notes: workers are safe to run in several copies (jobs are
  claimed with `for update skip locked`); the Redis bridge stays with the API.
- Check `deploy/entrypoint.sh` and the Dockerfile pass the variable through.

**Done when.** `docker compose --profile split up` runs one API and one worker,
and a handout generates end to end with them apart.

**Start at.** `backend/src/main.rs`, `docker-compose.yml`, `deploy/entrypoint.sh`.
