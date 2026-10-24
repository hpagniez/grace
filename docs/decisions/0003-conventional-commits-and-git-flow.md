---
status: accepted
date: 2026-10-24
decision-makers: Tusquito
---

# Use Conventional Commits and git flow

## Context and Problem Statement

Most of this repo is written by AI agents, often in parallel sessions. Without a fixed commit format and branch model, history is hard to read, changelogs and version bumps are manual, and agents can push half-done work to the release branch. Which conventions do humans and agents follow?

## Considered Options

1. Conventional Commits 1.0.0 + git flow (`main`, `develop`, `feature/*`, `release/*`, `hotfix/*`).
2. Conventional Commits + trunk-based development (short branches off `main`).
3. Free-form commits, ad hoc branches.

## Decision Outcome

Chosen: option 1. The commit format gives machine-readable history (changelog, SemVer bump). Git flow keeps `main` as released code only and gives agents a safe integration branch (`develop`). Option 2 is lighter, but the user asked for git flow.

Rules are in [AGENTS.md](../../AGENTS.md#git-workflow). Summary:

- Commit: `<type>(<scope>)<!>: <subject>`. Types: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`. Breaking change: `!` after the scope, or a `BREAKING CHANGE:` footer.
- `main` = released, tagged `vX.Y.Z` (SemVer). `develop` = integration. Work happens on `feature/<topic>`, cut from and merged into `develop` through a PR. Releases go through `release/X.Y.Z`; urgent fixes through `hotfix/X.Y.Z` cut from `main`. Both merge into `main` and back into `develop`.
- Merge with `--no-ff` so each branch stays visible in history. Never force-push `main` or `develop`.

### Consequences

- Good: changelog and version bump can be derived from commits later.
- Good: `main` is always releasable; agents never touch it directly.
- Bad: more branches to manage than trunk-based. Acceptable for a small team; revisit if release/hotfix branches stay unused.
- Migration: the repo currently has a single `master` branch. Rename it to `main` and create `develop` from it before the first feature branch. No enforcement tooling (commitlint, hooks) yet; add it with the first app code and record it in a new ADR.
