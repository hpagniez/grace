<!--
Title: Conventional Commits, e.g. `feat(runner): run a single outline example`.
Target `develop` (`release/*` and `hotfix/*` also target `main`). Rules: AGENTS.md.
Tick what applies. Strike out (~~text~~) or write "N/A: <reason>" for items that do not apply. Never delete an item silently.
-->

## Summary

<!-- What changes and why, in 1-3 sentences. -->

## Type

- [ ] `feat`: new user-facing capability
- [ ] `fix`: bug fix
- [ ] `docs`: documentation only
- [ ] `refactor` / `perf`: no behavior change / faster
- [ ] `test`: tests only
- [ ] `build` / `ci` / `chore`: tooling, dependencies, pipeline
- [ ] Breaking change (`!` in the title and a `BREAKING CHANGE:` footer)

## Links

- Feature: <!-- F01 ... F19 from docs/features.md, or "none" -->
- Spec: <!-- docs/specs/YYYY-MM-DD-<topic>.md, or "none" -->
- Decision record: <!-- docs/decisions/NNNN-<title>.md, or "none" -->
- Issue: <!-- Closes #N, or "none" -->

## Changes

<!-- Bullet the notable changes. Name files only where it helps a reviewer. -->

-

## Tests

- [ ] Unit tests (Vitest) added or updated: `*.test.ts` for Node code, `*.test.tsx` for components
- [ ] End-to-end tests (Playwright `_electron`) added or updated for user-visible behavior
- [ ] Each spec acceptance criterion (Gherkin) maps to at least one test
- [ ] `npx vitest run` passes
- [ ] `npm run package`, then `npx playwright test` passes
- [ ] `npm run typecheck` passes
- [ ] `npm run lint` passes (oxlint + oxfmt)
- [ ] CI green: `checks` and `test` (e2e ran after ready for review)
- [ ] Ran the app and checked the change by hand (`npm start`)

<!-- Paste the test summary lines. If a test was skipped or is flaky, say which and why. -->

```text

```

Tested on: <!-- OS + version, Java version, Karate version (e.g. Windows 11, Java 21, Karate 2.1.4) -->

## Screenshots

<!-- UI change: before/after screenshots or a short recording. Otherwise "N/A". -->

## Documentation

- [ ] `docs/features.md` row updated (status, spec link) in this PR
- [ ] Spec written or updated in `docs/specs/` (requirements, then Gherkin acceptance criteria)
- [ ] Hard-to-reverse choice (dependency, architecture, file format) has a new `docs/decisions/NNNN-*.md`; no accepted record edited
- [ ] `AGENTS.md` updated if a rule, command or stack fact changed
- [ ] `docs/inventory/` or `docs/research/` updated if this changes what we know about Karate
- [ ] `README.md` updated if setup or usage changed

## Checks

### Git

- [ ] Branch is `feature/<topic>` (or `release/vX.Y.Z` / `hotfix/vX.Y.Z`) and is up to date with its base
- [ ] Every commit follows Conventional Commits; one logical change per commit
- [ ] No merge commits from the base branch mixed with unrelated work; no force-push to `main` or `develop`

### Dependencies

- [ ] New or upgraded packages are pinned to exact versions (no `^` or `~`)
- [ ] Solid set (`solid-js`, `@solidjs/web`, `@solidjs/vite-plugin`, `@tanstack/solid-router`, `@kobalte/core`) upgraded together, if touched
- [ ] Lockfile committed; no unrelated lockfile churn

### Electron security

- [ ] `contextIsolation` on, `nodeIntegration` off, `sandbox` on
- [ ] Renderer reaches Node only through the typed preload API; new IPC channels validate their input in the main process
- [ ] `EnableNodeCliInspectArguments` fuse left as is
- [ ] Main-process code that runs per page load is idempotent (Forge dev reloads the renderer)

### Karate integration

<!-- Only if this PR spawns or parses Karate. Otherwise "N/A". -->

- [ ] Karate spawned from the main process without a shell, with `cwd` = project root and absolute `-o`
- [ ] `KARATE_OPTIONS`, `JAVA_TOOL_OPTIONS`, `JDK_JAVA_OPTIONS`, `_JAVA_OPTIONS` removed from the child environment
- [ ] CLI flags checked against `RunCommand.java`; none invented
- [ ] Exit codes 0/1/2/3 handled; 0 scenarios after a dry-run preflight shows a warning
- [ ] Cancel uses `taskkill /PID <pid> /T /F` on Windows and records "cancelled"
- [ ] Event parsing tested against the real fixtures in `docs/research/2026-10-24-karate-cli-spike/samples/`

### Code

- [ ] Solid 2 APIs only (no `solid-js/web`, `createResource`, `onMount`, `Suspense`, `ErrorBoundary`, `classList`, `use:`)
- [ ] UI is keyboard-usable and labelled for screen readers
- [ ] No secrets, tokens or personal paths committed
- [ ] No code, text or assets copied from Karate Labs' paid products

## Notes for reviewers

<!-- Risks, open questions, follow-ups, what was not tested. -->
