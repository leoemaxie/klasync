<!--
Title: use Conventional Commits, `type(scope): short summary`
e.g. `fix(vercel): move off deprecated Node 20 runtime`
Types: feat, fix, refactor, perf, docs, test, build, ci, chore
-->

## Summary
<!-- What changed and why (1-3 bullets). The diff shows the what; explain the why. -->

## Test plan
<!-- Tick what you ran; CI runs these too. -->
- [ ] Frontend: `pnpm format:check`, `pnpm check`, `pnpm test`, `pnpm build`
- [ ] Backend (if touched): `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test`
- [ ] Manually verified: <!-- what you ran or clicked -->

## Notes for reviewers
<!-- Risks, follow-ups, screenshots for UI changes. Link issues with `Closes #123`. -->
