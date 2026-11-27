# 09 - Commit the browser and real-stack test suites

**Why.** The materials features were verified with throwaway Playwright and
Python scripts that live outside the repo. Nothing in CI exercises the UI or the
real API end to end.

**Build.**
- Add the real-stack steps to `scripts/e2e.py` (generate, edit, publish, six
  exports, signed URL, student access, book, compare, webhook, slides) with the
  existing `E2E_SKIP_AI` switch for CI.
- Add a Playwright suite for the Materials tab, the student handout and the
  course-book modal, run against the compose stack.
- Fix the dev-database pitfall: tests share a database with a running API and
  can fail when its workers pick up test jobs (document it or isolate it).

**Done when.** CI runs the new steps green on a fresh stack and a failing UI
flow fails the build.

**Start at.** `scripts/e2e.py`, `.github/workflows/`, `docker-compose.yml`.
