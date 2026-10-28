---
status: accepted
date: 2026-10-28
decision-makers: Tusquito
---

# Run e2e on CI only for PRs into main

## Context and Problem Statement

[ADR 0007](0007-github-actions-ci-cd-on-the-free-private-plan.md) chose "e2e only once the PR is ready for review", so e2e gates every merge. Since [ADR 0016](0016-windows-tests-in-parallel-jobs.md), a ready PR runs four Windows jobs. On the 1.0.0-rc.3 release PR (#47) they took 6.5 and 4 minutes (unit) and 8 and 6 minutes (e2e): about 27 billed Windows minutes, 15 of them for e2e, out of about 1,000 a month.

Every feature PR into `develop` pays for e2e, and so does the merge of a `release/*` or `hotfix/*` branch back into `develop`. That back-merge runs `checks` and `test` again on the very commit that passed them, e2e included, on its PR into `main` (#48 after #47).

Where should e2e run, and which PRs need CI at all?

## Considered Options

1. Keep e2e on every ready PR (ADR 0007).
2. Unit tests on CI for every PR into `develop`; e2e run locally before a PR into `develop` is marked ready; e2e on CI only for PRs into `main`. No CI on a back-merge into `develop`.
3. e2e after merge, on pushes to `develop` (ADR 0007's option 2). A merge by `merge-when-green` uses `GITHUB_TOKEN`, which starts no workflow (ADR 0011), so this needs a personal token or a GitHub App.

## Decision Outcome

Chosen: option 2.

- `checks` and `test` skip every job on a PR from `release/*` or `hotfix/*` into `develop`. GitHub records those runs as skipped.
- `test`'s `unit` job runs on every other PR into `develop` or `main`, draft or not.
- `test`'s `e2e` job runs only on PRs into `main`, and still skips the step named `e2e` while the PR is a draft. A manual run (`workflow_dispatch`) on any branch runs e2e too.
- Before marking a PR into `develop` ready, agents run `npm run e2e` locally when the PR changes more than `docs/**` and `*.md`, and paste the summary in the PR.
- `merge-when-green` no longer requires an `e2e` step for a PR into `develop`: `checks` and `test` must have passed on its head commit, as before (docs-only PRs still need no `test` run).
- For a back-merge, `merge-when-green` looks up the newest successful `pull_request` run of `checks` and of `test` on the head commit: the runs of its PR into `main`. Every step named `e2e` in that `test` run must have passed.

This supersedes ADR 0007's choice of option 3 (e2e on every ready PR) and ADR 0011's rule that a PR into `develop` merges only after its `e2e` step passed. ADR 0016's four jobs stand; the two `e2e` jobs now run only for PRs into `main`. ADR 0012's bump PRs get `checks` and unit tests when marked ready, not e2e.

### Consequences

- Good: a PR into `develop` costs about 12 billed Windows minutes instead of about 27; a back-merge costs none.
- Good: e2e still gates every release and hotfix, on the commit that is merged into `main` and tagged.
- Bad: CI no longer proves e2e on `develop`. A local run can be skipped or run on other changes, and nothing checks it. An e2e break shows up on the next release PR, possibly several features later.
- Bad: bot PRs have no one to run e2e locally. A Dependabot Electron bump or a `vendor-updates` JRE or Karate bump merges into `develop` with unit tests only, unless someone runs `npm run e2e` on its branch or starts `test` by hand on it (`gh workflow run test.yml --ref <branch>`). The runner unit tests still start the bundled Java and Karate for real.
- Bad: a back-merge whose branch gained a commit after its PR into `main` (a conflict fix with `develop`, say) has no successful run on its head. `merge-when-green` then refuses it, and it is merged by hand after a local `npm test` and `npm run e2e`.
- Neutral: marking a draft PR into `develop` ready still re-runs `checks` and `test` on the same commit. That start is what runs CI on a `vendor-updates` draft, whose bot pushes start no workflow.
