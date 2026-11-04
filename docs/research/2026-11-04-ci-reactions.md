# Reacting to green CI and merged PRs

Checked on 2026-11-04. Question ([#173](https://github.com/hpagniez/grace/issues/173)): how can the agents that build Grace react when CI passes or a PR merges, not only when CI fails, and which of the steps done by hand after a merge can leave the people and sessions out of the loop?

**Bottom line:** put the post-merge work in `merge-when-green` itself and stop relying on a session to do it. Build two small things, no agent, no new action, no secret, no setting:

1. Make the tail of `merge-when-green.yml` retry its GitHub calls, safe to run again on an already-merged PR, and let it set the `Spec:` line of the issues the PR names.
2. Put a lock file in front of `npm test`, `npm run e2e` and `npm run bench`, so parallel sessions queue for the one machine without a parent session brokering it.

A session that has its own follow-up waits for green or merge itself, in a background `gh pr checks --watch`. None of the cloud options fits: a routine cannot see CI results or reach the owner's machine, and `claude-code-action` and a repo webhook add a secret, a third-party action or a public endpoint to do three steps that need no judgment. Release benchmarks and local e2e stay with the owner on the reference machine.

## Sources

Claude Code docs (all read on 2026-11-04; routines are now called "scheduled tasks", cloud ones "research preview"):

- Routines / cloud scheduled tasks: <https://code.claude.com/docs/en/routines> (triggers, supported GitHub events, filters, GitHub App, `claude/` branches, usage and limits)
- Fire a routine over HTTP: <https://platform.claude.com/docs/en/api/claude-code/routines-fire>
- Cloud sessions and Auto-fix: <https://code.claude.com/docs/en/claude-code-on-the-web>; cloud environments and the GitHub proxy: <https://code.claude.com/docs/en/cloud-environments>
- Desktop app (PR monitor, auto-archive, worktrees, cross-session messaging): <https://code.claude.com/docs/en/desktop>; local scheduled tasks: <https://code.claude.com/docs/en/desktop-scheduled-tasks>; `/loop` and the comparison table: <https://code.claude.com/docs/en/scheduled-tasks>
- Channels: <https://code.claude.com/docs/en/channels> and <https://code.claude.com/docs/en/channels-reference>
- `anthropics/claude-code-action`: <https://github.com/anthropics/claude-code-action/blob/main/docs/usage.md>, <https://github.com/anthropics/claude-code-action/blob/main/docs/security.md>
- Managed Agents webhooks: <https://platform.claude.com/docs/en/managed-agents/webhooks>

GitHub docs: events that trigger workflows (<https://docs.github.com/en/actions/writing-workflows/choosing-when-your-workflow-runs/events-that-trigger-workflows>), creating webhooks (<https://docs.github.com/en/webhooks/using-webhooks/creating-webhooks>), Actions billing (<https://docs.github.com/en/billing/managing-billing-for-your-products/managing-billing-for-github-actions/about-billing-for-github-actions>).

This repository, read only: `.github/workflows/merge-when-green.yml`, [ADR 0011](../decisions/0011-merge-labelled-prs-when-ci-is-green.md), [ADR 0024](../decisions/0024-sign-grace-exe-with-signpath-foundation.md), `gh api repos/hpagniez/grace` (`delete_branch_on_merge: true`, `allow_auto_merge: false`, public), `gh api repos/hpagniez/grace/hooks` (no webhook), `gh run view 41809376274 --log-failed`, `gh pr list --state merged`.

Nothing was created or changed for this note: no routine, trigger, webhook, workflow, secret or setting. The behaviours below that rest on the docs alone are listed under [Not verified](#not-verified).

## What is done by hand after a merge

| Step | Who does it now | Needs judgment? | Where it can run |
|---|---|---|---|
| Point the issue's `Spec:` line at the merged spec ([AGENTS.md](../../AGENTS.md), Doc rules) | The parent session, with `gh issue edit` | No. The rule is mechanical: a spec file the PR added, an issue the PR names | CI, after the merge |
| Close issues and delete the branch when `merge-when-green` failed after the merge | A person | No | CI, if the tail can be run again |
| One machine for local `npm test`, `e2e` and `bench` across parallel sessions | A parent session keeps a queue and sessions ask it | No | A lock on the machine, not CI |
| Release benchmark, local e2e before a release, the `Decision:` line | The owner | Yes, and it needs the reference machine | Nowhere else |
| Start the next task | The user or the parent | Yes | Nowhere else |

The failure the second row is about is run `41809376274` on 2026-11-04: `merge-when-green` merged PR #240 at 09:42:45, the job died four seconds later on a `502 Bad Gateway` from GitHub, and issue #224 was closed at 09:43:22 with the text `Done in #240.` by hand. Re-running that job would not have helped: the script ends with `skip "it is not open"` for any PR that is no longer open, so a re-run after a merge never reaches the lines that close the issues. That makes the tail a single point of failure today. (`delete_branch_on_merge` is on, and `gh pr merge` is called with `--delete-branch`, so the remote branch is deleted twice over and was not the problem.)

## The options

| Option | Reacts to | Runs where | Reaches the owner's machine or a session | Verdict |
|---|---|---|---|---|
| Cloud routine, GitHub trigger | Pull request (opened, closed, labeled, synchronized and so on; filters include `Is merged`, base and head branch, labels, draft), release, issue | Anthropic's cloud, fresh clone | No | No |
| Plain GitHub Actions | `pull_request: closed`, `workflow_run: completed`, anything | GitHub's runner | No | Yes, as steps in the existing workflow |
| Actions plus `claude-code-action` | Any workflow event | GitHub's runner | No | Not now |
| Repo webhook | Any repo event | Your endpoint | Only through a tunnel | No |
| Managed Agents webhooks | Managed Agents sessions, vaults, agents, deployments, environments, memory stores | Your endpoint | No | Does not apply |
| Desktop PR monitor and auto-archive | Failing checks, new review comments | The owner's machine | Yes, it is the session | Keep, do not extend |
| Channels | What a channel server pushes | The owner's machine, in an open session | Yes | Revisit after the preview |
| A session waits itself | `gh pr checks --watch`, `/loop`, the Monitor tool | The owner's machine, in that session | Yes | Use |

### Claude Code routines (cloud scheduled tasks)

- **Events.** The GitHub trigger lists three event kinds: pull request, release, issue. `workflow_run`, `check_suite`, `check_run` and `push` are not among them, so a routine cannot fire on "CI went green". A PR trigger filtered on `Is merged` can fire on a merge. Each event starts a new, independent session. The docs state per-task and per-account hourly caps on GitHub events and drop events over the cap, but give no number.
- **Setup.** The Claude GitHub App has to be installed on the repository (an owner action), and the routine runs under the owner's GitHub identity. The feature is a research preview, on Pro, Max, Ultra, Team and Enterprise.
- **Reach.** The run is a cloud session with a fresh clone. It cannot run the bench or e2e on the reference machine, and no documented call posts into a local or desktop session. The GitHub proxy rejects branch deletions, and rejects `gh` subcommands that use GraphQL, which includes `gh pr` and `gh issue`; REST through `gh api` works. A routine could therefore do the issue and `Spec:` edits through REST, but gains nothing over a workflow doing the same.
- **API.** A routine can also start from an HTTP `POST` (`/v1/claude_code/routines/{id}/fire`, per-routine bearer token, experimental). The docs' own example is a GitHub Actions step on `failure()`, so Actions could fire a routine on green. That still ends in a cloud session with the limits above. The `create_webhook_trigger` call named in the issue is not in the public docs I read.
- **Cost.** Every run draws on the same subscription as the interactive sessions that build Grace. 125 PRs merged into `develop` between 2026-10-24 and 2026-11-04 (about ten a day), so a merged-PR trigger would start about ten full sessions a day.

### GitHub Actions on `pull_request: closed` or `workflow_run`

- `pull_request: closed` also fires for a close without merge, so a job-level `if: github.event.pull_request.merged == true` is needed. `workflow_run` types are `completed`, `requested` and `in_progress`; it runs only if the workflow file is on the default branch, and chains at most three levels.
- **The catch.** Events created by a workflow's `GITHUB_TOKEN` start no workflow run, except `workflow_dispatch` and `repository_dispatch`. `merge-when-green` merges with that token, so a new `pull_request: closed` workflow would run for merges by hand and not for the merges that matter. The workflow's own comment records the same rule for closing keywords (`Closes #N`) in token merges. The post-merge steps therefore belong in `merge-when-green` itself, which needs no extra trigger.
- **Cost.** Standard runners are free on public repos (GitHub billing docs; [ADR 0018](../decisions/0018-make-the-repository-public.md)). The tail added to a job that already runs about six seconds does not add a billed minute, and a separate workflow would add one run per trigger (the [October note](2026-10-28-linux-actions-minutes.md) counted 81 `merge-when-green` runs for 13 merges).

### `anthropics/claude-code-action`

- It is not a GitHub-owned action, so it needs an allowlist entry in the repository's Actions settings (an owner action, agents never change them) and a decision record that widens [ADR 0007](../decisions/0007-github-actions-ci-cd-on-the-free-private-plan.md) and the SignPath exception of [ADR 0024](../decisions/0024-sign-grace-exe-with-signpath-foundation.md).
- It authenticates with `ANTHROPIC_API_KEY`, a `CLAUDE_CODE_OAUTH_TOKEN` (the owner's subscription) or workload identity federation. A secret on a public repo is withheld from fork PRs, but `workflow_run` and `pull_request_target` run with the base repo's secrets, and the action's own security page warns about hidden-markdown prompt injection and about checking out untrusted PR code into the workspace root.
- Cost: Actions minutes plus tokens per run. No token figure was measured.
- The three post-merge steps need no model. Revisit it for a task that does, such as writing a release summary.

### A repo webhook

Creating one takes the repository owner or an admin. GitHub POSTs to a URL that someone must host and keep up; the docs page I read does not say whether a failed delivery is redelivered. It cannot wake a desktop session. The localhost listener in the channel example binds `127.0.0.1`, so GitHub could not reach it without a tunnel. Nothing here pays for the extra moving parts.

### Managed Agents webhooks

They point the other way, as the issue says: Anthropic POSTs to your HTTPS endpoint when a Managed Agents resource changes (session, vault, agent, deployment, deployment run, environment, memory store). The payload carries only a type and an id. They are in beta, make up to three delivery attempts, and take no GitHub input. Grace runs no Managed Agents, so they do not apply.

### The desktop app

- **PR monitor.** The docs name two things Auto-fix does: fix failing checks, and (in a local session) address new review comments from the owner, org members, collaborators or GitHub Apps. They list no wake event for CI success, merge or close, and no setting to be notified of one. For merge conflicts the cloud-session page says GitHub emits no webhook when a base branch advances, so Auto-fix cannot react to a conflict there; the desktop page is silent. "Merge when ready" switches on GitHub's auto-merge (a squash merge unless the base uses a merge queue); `allow_auto_merge` is false in this repo, and `merge-when-green` stays the merge path.
- **Auto-archive.** "Auto-archive after PR merge or close" archives a session when its PR merges or closes; sessions still running do not archive themselves. With uncommitted changes the worktree stays on disk for about 30 days; the docs do not say what happens to a local branch. The consequence for this repo: any step that waits for the merge must not depend on the session that opened the PR still being there. That is the reason to move the post-merge steps into CI.
- **Scheduled tasks on the desktop** start a new session at a time, only while the app is open and the computer awake; the forms allow an interval of one hour at the shortest. They cannot wake an existing session and are not event driven.

### Channels (not in the issue)

A channel is an MCP server that pushes events into a running local session, so the event reaches the session that has the context and the files. Research preview; started with `--channels`; a custom channel needs `--dangerously-load-development-channels` and an interactive confirmation; on Team and Enterprise an owner must enable them. A channel can poll an API instead of listening, as the chat plugins do, which needs no public URL. Events arrive only while the session is open, and I found no statement that the desktop app supports channels. Not now; look again when the preview ends.

### A session waits for its own follow-up

This works today and needs nothing new. `gh pr checks <n> --watch --fail-fast` ends when the checks finish (`gh` documents exit code 8 for "pending"), and a background command re-invokes the session when it exits. `/loop` and the Monitor tool do the same on a timer or a stream. It covers "green" and, with `gh pr view <n> --json state,mergedAt`, "merged". It costs a command that polls every ten seconds, but only while that session waits, and it runs on the owner's machine.

## The recommended setup

1. **Harden and extend the tail of `merge-when-green.yml`** (a follow-up PR, not part of this note):
   - Retry each `gh` call that follows the merge a few times with a short pause, so a single 502 does not stop the issue closing.
   - For a PR that is already merged, run the tail instead of ending at "it is not open", so "Re-run failed jobs" repairs a failure. Closing an already-closed issue is skipped today; keep that.
   - Add the `Spec:` step: for each issue the PR names with `Closes #N` or `Part of #N` (the second is not in `closingIssuesReferences`, so parse the PR body), if the PR added a file under `docs/specs/`, set the issue's `Spec:` line to that file on `develop`. The workflow already holds `issues: write`.
   - Cost: no extra run, a few seconds more in a job that is billed one minute either way.
2. **A machine lock** (a follow-up PR): a short `scripts/machine.mjs` that takes an exclusive lock file in the OS temp folder, waits while another process holds it, treats a lock whose process no longer exists as stale, and runs the command it wraps. `npm test`, `npm run e2e` and `npm run bench` go through it, so [AGENTS.md](../../AGENTS.md)'s "never run any two at the same time" becomes a rule the machine enforces. About fifty lines, no dependency, one unit test. Single test files stay outside the lock, as today.
3. **Leave the rest as it is.** Auto-fix and auto-archive stay on. A session with a follow-up waits for it in the background. `merge-when-green` is still added only when the user asks.

## What still needs a person

- **The owner**, on the reference machine: the release benchmark, local e2e before a release and the `Decision:` line. No cloud option can do these, and a lock only orders local runs.
- **Merges by hand**, which run no `merge-when-green` tail: PRs into `main`, PRs from forks, and unlabelled PRs into `develop`. Their `Spec:` line stays a manual step.
- **Every change to the repository's Actions settings, the Claude GitHub App, a routine or a secret.** None is recommended here.
- **The user's go** for the machine and for the `merge-when-green` label, as today.

## Not verified

- That a merge made by `merge-when-green` starts no `pull_request: closed` run: from GitHub's documented `GITHUB_TOKEN` rule, not tested. Adding a probe workflow was out of scope.
- Whether the desktop app honours `--channels`; whether a session waiting on a background command counts as "running" for auto-archive; what auto-archive does to a local branch; whether local Auto-fix reacts to merge conflicts.
- The GitHub-event hourly caps of routines (the docs give no number), whether a routine's GitHub trigger works on a public repo with the app installed, and whether a routine may close issues or edit bodies through the proxy's REST path.
- Token cost of `claude-code-action` runs.
- The `create_webhook_trigger` call from the issue.
