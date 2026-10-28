# Linux Actions minutes in October 2026

Checked on 2026-10-28. Question: which workflows in `hpagniez/grace` used the 775 Linux minutes billed on the account in October, and which of them waste minutes?

**Bottom line:** `grace` used only 171 of the 775 Linux minutes. The other 604 came from the archived repo `codebase-indexer-mcp`. Inside `grace`, `merge-when-green` wastes about 67 of its 81 billed minutes on no-op runs, but a job-level `if:` can cut only 4 of them, so no change was made. The quota is driven by Windows: `grace` alone is 981 Windows minutes (1,962 at the 2x multiplier) plus 171 Linux minutes, which is already over 2,000 without the other repo.

## Sources

- Account billing: `gh api "users/hpagniez/settings/billing/usage?year=2026&month=10"` (per repo, per SKU, per day). `GET /users/{user}/settings/billing/actions` returns 410.
- Run list: `gh api --paginate "repos/hpagniez/grace/actions/runs?created=2026-10-01..2026-10-31"` (290 runs, the first on 2026-10-24).
- Jobs: `repos/hpagniez/grace/actions/runs/<id>/jobs?filter=all` for each run (332 job attempts). The run `/timing` endpoint reports `total_ms: 0` for Ubuntu jobs, so billed minutes are computed as `ceil(completed_at - started_at)` per job, at least 1, and 0 for jobs that never got a runner.
- Outcomes of `merge-when-green` jobs: the check-run annotations (`::notice::not merging ...`) of each billed job.

My job sums differ a little from the billing API: Linux 176 against 171, Windows 1,012 against 981. Treat per-workflow numbers as accurate to about 3%.

## Billing by repository

| Repository | SKU | Minutes |
|---|---|---|
| `codebase-indexer-mcp` | Actions Linux | 604 |
| `grace` | Actions Linux | 171 |
| `grace` | Actions Windows | 981 |

Gross $14.49, discount $14.49, net $0. Prices are $0.006 per Linux minute and $0.010 per Windows minute.

`codebase-indexer-mcp` minutes by day: 10-01 11, 10-05 128, 10-12 21, 10-13 100, 10-14 27, 10-19 216, 10-26 101. The big days are Mondays (10-05, 10-12, 10-19, 10-26): Dependabot update runs (nuget, docker, github_actions; 49 runs in October) and the `CI` workflow on the PRs they open (12 failed `pull_request` runs, 45-minute job timeout), plus a weekly scheduled `CI` run (cancelled each time). The repo is now archived (`archived: true`, updated 2026-10-28 13:04 UTC), which should end that usage. This note did not change anything in that repo.

## Linux minutes in `grace`

| Workflow | Job | Jobs billed | Billed minutes | Real minutes |
|---|---|---|---|---|
| `checks` | `checks` | 89 | 89 | 30.6 |
| `merge-when-green` | `merge` | 81 | 81 | 8.1 |
| Dependabot (npm, github_actions) | update | 2 | 3 | 2.3 |
| `vendor-updates` | `jre`, `karate-check`, `karate-pr` | 3 | 3 | 0.5 |
| `release` | none on Linux | 0 | 0 | 0 |
| **Total** | | | **176** | 41.5 |

27 more `merge-when-green` jobs were skipped by the existing job-level `if:` and cost 0.

The rounding rule is the main cost: 175 billed Linux jobs that ran 41.5 real minutes in total were billed 176.

### `checks`

90 runs, one per push to a PR (86 success, 2 failure, 2 cancelled by `concurrency`). Each takes about 20 s and is billed 1 minute. Nothing to cut: it is the lint, typecheck, audit and commit-header gate, and the `if:` already skips release and hotfix back-merges (ADR 0017).

### `merge-when-green`

81 billed jobs, about 6 s each. What each one did:

| Trigger | Outcome | Jobs |
|---|---|---|
| `workflow_run` | PR has no `merge-when-green` label | 34 |
| `workflow_run` | PR is a draft | 13 |
| `workflow_run` | merged the PR | 10 |
| `workflow_run` | `test` or `e2e` not green yet on that head | 4 |
| `workflow_run` | PR does not target `develop` | 3 |
| `workflow_run` | no PR in the event | 1 |
| `pull_request_target` | `checks` or `test` not finished or not green yet (label added early) | 12 |
| `pull_request_target` | merged the PR | 3 |
| `pull_request_target` | failed (exit 1) | 1 |

13 of 81 minutes merged a PR. 67 were no-ops, 1 failed.

What a job-level `if:` can reach: the `workflow_run` payload carries the PR number and its `head` and `base` refs (GitHub webhook docs for `workflow_run`, not re-checked in this session), but no labels and no draft flag. So `github.event.workflow_run.pull_requests[0].base.ref == 'develop'` can skip the 3 "does not target develop" and the 1 "no PR" jobs, which is 4 minutes. The 34 no-label and 13 draft jobs, 47 minutes, need the API, so they cannot be skipped before a runner starts. The `pull_request_target` side already checks the label at job level.

Decision: no PR. Saving 4 minutes a month (2% of the `grace` Linux minutes, 0.2% of the 2,000-minute allowance) does not pay off.

Options that would cut more, each a design change to ADR 0011 and not done here:

- Have `checks` or `test` start the merge only for labelled, ready PRs (for example a last step that dispatches `merge-when-green`). Saves up to about 47 minutes.
- Merge from the last step of the `test` job instead of a separate workflow. Removes the 1-minute rounding of the separate job entirely, but runs on Windows and needs the docs-only PR path handled.

### `vendor-updates`, `release`, Dependabot

`vendor-updates` ran once, by hand (3 Linux jobs, 1 Windows job, 4 billed minutes). `release` ran 6 times on Windows only (3 failures). Dependabot used 3 minutes. None is a Linux problem.

## Windows, for scale

Windows is 981 billed minutes, or 1,962 of the 2,000 at the 2x multiplier. Billed minutes by outcome (my job sums, 1,012 in total):

| Job | Success | Failure | Cancelled |
|---|---|---|---|
| `test` (single-job layout) | 589 (45 jobs) | 116 (8) | 63 (13) |
| `unit` | 109 (21) | 5 (1) | 4 (2) |
| `e2e` | 110 (18) | 0 | 4 (2) |
| `release` | 8 (3) | 3 (3) | 0 |
| `vendor-updates` `karate` | 1 (1) | 0 | 0 |

Failed and cancelled Windows jobs burned about 195 billed minutes (19%). Dropping the whole `codebase-indexer-mcp` Linux usage would not have kept `grace` under the allowance on its own: 1,962 + 171 = 2,133.

## Follow-ups

- Windows `test` minutes are the lever. This note does not cover them beyond the table above; see `2026-10-27-ci-test-speed.md`.
- If the quota is the constraint after 2026-11-01, the cheapest Linux change is the first option under `merge-when-green`, worth about 47 minutes a month.
