---
status: accepted
date: 2026-10-24
decision-makers: Tusquito
---

# Run CI/CD on GitHub Actions within the free private plan

## Context and Problem Statement

The repo has no CI. Agents open most PRs, so nothing checks lint, types, tests, commit format or known vulnerabilities before a merge, and releases are built by hand. The repo is private on GitHub Free and stays private for now. That plan gives:

- 2,000 Actions minutes a month. Windows runners use them at 2×, so about 1,000 Windows minutes.
- 500 MB artifact storage, 10 GB cache per repo.
- Dependabot alerts, security updates and version updates.
- No CodeQL, no dependency review, no secret scanning, no branch protection or rulesets.

The tests need Windows: `taskkill`, the PowerShell `java.exe` leak checks and the bundled Windows JRE. The unit suite (109 tests) takes about 1.5 minutes locally; the e2e suite packages the app first. Many PRs are docs only (specs, plans, ADRs). [ADR 0003](0003-conventional-commits-and-git-flow.md) deferred commit-format enforcement to this point.

How does CI test every change before merge and keep Windows minutes low?

## Considered Options

When the Windows e2e suite runs:

1. Unit and e2e tests on every code push to a PR. Simplest; about 25–30 billed minutes per push, so roughly 35 runs a month.
2. Unit tests on PRs; e2e after merge on `develop` and `main`, and on release and hotfix PRs. Cheapest, but an e2e break shows up only after merge.
3. Unit tests on every code push; e2e only once the PR is ready for review (not a draft).

## Decision Outcome

Chosen: option 3. E2E still gates every merge, and agents pushing fix-ups to a draft pay only for the unit run.

### Workflows (`.github/workflows/`)

| File | Trigger | Runner | Steps |
|---|---|---|---|
| `checks.yml` | Every PR to `develop` or `main` | `ubuntu-latest` | `npm ci` (Electron binary download skipped), `npm run lint`, `npm run typecheck`, `npm audit --audit-level=high`, commit check, branch check |
| `test.yml` | PRs to `develop` or `main` that touch more than `docs/**` and `*.md` (opened, synchronize, reopened, ready_for_review); manual | `windows-latest` | `npm ci`, `npm run vendor`, `npm test`; then `npm run e2e` unless the PR is a draft |
| `release.yml` | Push of a `v*.*.*` tag | `windows-latest` | Tag must equal `v` + `package.json` version; `npm run make`; `gh release create` with the zip and generated notes |

- Commit check: every non-merge commit in the PR has a first line matching `<type>(<scope>)<!>: <subject>` with the ADR 0003 types, a lower-case subject, no trailing period, 72 characters max. Dependabot PRs are skipped.
- Branch check: `feature/*` and `dependabot/*` target `develop`; `release/*` and `hotfix/*` target `main` or `develop`. Anything else fails.
- One Windows job runs both suites in turn, so install and downloads happen once and the suites never overlap (AGENTS.md).
- Caches: npm (setup-node), Electron download (key: `package-lock.json`), `vendor/` (key: `scripts/fetch-vendor.mjs`).
- A new push cancels the running workflow of the same PR.
- Release: no test re-run; the release or hotfix PR into `main` ran them. The zip goes to the GitHub Release, not to Actions artifacts (it is about 210 MB of the 500 MB quota). No other artifacts are uploaded.
- Actions are pinned to full commit SHAs with a version comment. Workflows default to `permissions: contents: read`; only `release.yml` gets `contents: write`.

### Dependencies and vulnerabilities

- `.github/dependabot.yml`: npm and github-actions, monthly, target `develop`, commits `build(deps)` / `ci(deps)`. npm groups: `solid` (`solid-js`, `@solidjs/*`, `@tanstack/solid-router`, `@kobalte/core`, all update types, because AGENTS.md upgrades the Solid set together), `electron-forge` (`@electron-forge/*`, `@electron/*`), then one group for all other minor and patch updates. Majors outside those groups get one PR each. Actions updates are one group.
- Repo settings: turn on Dependabot alerts and Dependabot security updates.
- Make `develop` the default branch. Dependabot security updates always target the default branch, and git flow forbids PRs into `main` except from `release/*` and `hotfix/*`. New PRs also target `develop` by default; release and hotfix PRs pick `main` explicitly.
- `npm audit` fails today: `oxfmt` 0.41.0 pulls `tinypool` with two critical advisories (GHSA-5gmw-xhrv-c9v3, GHSA-85c8-ppgw-ccpr). Upgrade `oxfmt` to 0.74.0 (exact) in its own commit, re-format if its output changed.

### Process

- AGENTS.md: open PRs as drafts while working; mark ready for review when done, which runs e2e. Merge only with green checks.

### Consequences

- Good: e2e passes before every merge; draft pushes cost about 12 billed minutes instead of 25–30; docs-only PRs use no Windows minutes.
- Good: commit format and git flow branch targets are checked, as ADR 0003 asked.
- Bad: checks are advisory. Without branch protection nothing blocks merging a red PR; the reviewer must check the status.
- Bad: no CodeQL or secret scanning. When the repo goes public, add CodeQL, dependency review and branch protection in a new ADR, and drop the minute-saving rules (public repos have no minute cap).
- Bad: Dependabot does not update the JRE and Karate pins in `scripts/fetch-vendor.mjs`; they stay manual.
- Bad: the release zip is unsigned (no certificate), so Windows SmartScreen warns on first start.
- Bad: a tag pushed on an untested commit still builds a release.
- Neutral: Dependabot rebases open PRs when `develop` changes, and each rebase re-runs the Windows tests. Set `rebase-strategy: disabled` if that eats too many minutes.
