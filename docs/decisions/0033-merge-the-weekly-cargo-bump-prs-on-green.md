---
status: accepted
date: 2026-11-11
decision-makers: Tusquito
---

# Merge the weekly Cargo bump PRs on their own once all tests pass

## Context and Problem Statement

`cargo-updates` opens a PR every Tuesday that moves the GPUI set, Rust and `Cargo.lock` to their newest releases, the Windows patches rebased, and starts `test` on it, Windows e2e and minimised idle included ([ADR 0032](0032-keep-the-gpui-set-rust-and-cargo-lock-current.md)). The PR was a draft. A person had to mark it ready, so that `checks` and `perf` ran, then merge it: a PR opened with the workflow's `GITHUB_TOKEN` starts no `pull_request` workflow, and `merge-when-green` ([ADR 0011](0011-merge-labelled-prs-when-ci-is-green.md)) waits for `pull_request` runs only. The first one, #330, needed both.

The owner's rule, 2026-11-11: the weekly Cargo bump PRs merge on their own once all tests pass.

How does a bump PR that no person opened get every run that a merge needs, and merge?

## Considered Options

1. `merge-when-green` also takes runs started by hand, on a `workflow_run` of `checks`, `test` or `perf`.
2. `cargo-updates` starts the three runs itself, waits for them in a second job, and merges.
3. Open the PR with a personal token, so that its `pull_request` runs start and `merge-when-green` merges it.

## Decision Outcome

Chosen: option 2. Option 1 depends on `workflow_run` firing when a run that `GITHUB_TOKEN` started ends. GitHub documents only that events triggered by `GITHUB_TOKEN` start no new workflow run, `workflow_dispatch` and `repository_dispatch` aside, so the merge might silently never come, the very failure this ADR removes. Option 3 puts a long-lived personal token with write access in the repository's secrets.

- `scripts/bump-cargo.mjs --pr` opens the bump PR ready for review, not as a draft, with `Part of #328` in its body, and dispatches `checks.yml`, `test.yml` and `perf.yml` on its branch. Older bump PRs, draft or not, that hold only the bot's commits are closed as superseded; any other is named in the new PR, as before.
- `checks.yml` takes `workflow_dispatch`: it checks the branch as a PR into `develop` (`check-pr.mjs` gets the branch's name, `develop` and `origin/develop`). Its `checks` check run is on the PR's head commit, so it meets the ruleset's required check.
- A second job of `cargo-updates`, `merge`, runs `scripts/bump-cargo.mjs --merge <pr> <sha>`. It reads the newest `workflow_dispatch` run of each of the three workflows on that commit once a minute. When all three ended in success, it comments the links to the runs on the PR (the runs are not on its checks tab, and `perf`'s table is only in its run's summary), then merges with a merge commit, deleting the branch, with `--match-head-commit`, so a push after the runs stops the merge. A run that ended any other way, a refused merge, or 110 minutes without all three done, leaves the PR open with a comment that says why, and fails the job. The job holds `contents` and `pull-requests: write` and `actions: read`, runs no code of the new releases, and checks out without keeping the token.
- A person then fixes the branch, or starts the failed run again by hand, and merges it as any PR (AGENTS.md). The newest run of each workflow counts, so a run started again replaces a failed or cancelled one.
- `perf` measures Electron Grace, which a bump does not change, so its numbers are noise there; it runs because AGENTS.md merges nothing before `perf` finished with success. GPUI Grace's size, start and minimised wake-ups are in `gpui-idle`'s summary.

This supersedes ADR 0032's "opens a draft PR into `develop`" and "`checks` still starts when a person marks the PR ready for review". The rest of ADR 0032 stands.

### Consequences

- Good: a new GPUI set, Rust or lock reaches `develop` on Tuesday, tested, with nobody's action. A person acts only when something failed, and the comment says what.
- Good: `checks` and `perf` run on every bump, where before they waited for a person.
- Bad: no person reads the new releases' changes before they reach `develop`. Every release still goes through a person: a `release/*` PR into `main`, its check record and the release benchmark.
- Bad: the `merge` job holds a Linux runner while it waits, about 30 to 60 minutes a week (free on a public repository).
- Bad: a weekly comment on the bump PR, which notifies whoever watches the repository.
- Neutral: the workflow runs only from the default branch, so the `merge` job's first real run is after merge.
