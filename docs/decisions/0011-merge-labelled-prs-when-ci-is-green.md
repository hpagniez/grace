---
status: accepted
date: 2026-10-27
decision-makers: Tusquito
---

# Merge a labelled PR on its own once CI is green

## Context and Problem Statement

[ADR 0007](0007-github-actions-ci-cd-on-the-free-private-plan.md) runs `checks` and `test` on every PR but cannot enforce them. The repo is private on GitHub Free: no branch protection, and no GitHub auto-merge (`allow_auto_merge` is false). Agents open the PRs and the user merges each one by hand once CI is green. The Claude desktop app's CI monitor wakes a session when CI fails, never when it passes. So "merge when green" has meant waiting for a person to come back.

How can a PR merge itself once CI passes, without ever merging something CI has not passed?

## Considered Options

1. Keep merging by hand.
2. GitHub auto-merge. Not available: it needs branch protection with required checks, which private repos on GitHub Free do not have.
3. An agent session polls CI and merges. The session must stay alive and spends tokens waiting.
4. A third-party auto-merge action. A new dependency that holds a write token.
5. Our own workflow, `merge-when-green.yml`, that merges a PR with the `merge-when-green` label once `checks` and `test` passed on its head commit.

## Decision Outcome

Chosen: option 5. It is opt-in per PR, adds no dependency, and is one short shell step.

### Workflow

- Triggers: `workflow_run` (completed) of `checks` and `test`, and `pull_request_target` (`labeled`, `ready_for_review`) on PRs into `develop`, for a label added or a draft marked ready after CI finished. Both triggers run the file as it is on `develop`, so a PR cannot change the rule it is merged under.
- It merges only when all of these hold. Each is read from the API when the job runs, not from the event:
  - the PR is open, not a draft, targets `develop`, and has the `merge-when-green` label;
  - the newest `pull_request` run of `checks` on the PR's current head commit succeeded;
  - the newest `pull_request` run of `test` on that commit succeeded, and its `e2e` step passed. A run started while the PR was a draft skipped e2e, so it never counts, even if the PR was marked ready a moment ago. If `test` has no run on that commit, every changed file must be under `docs/` or end in `.md`, which is when its `paths-ignore` skips it.
- Because the runs are looked up by the PR's current head, a run on an older commit never counts. `gh pr merge <n> --merge --delete-branch --match-head-commit <sha>` then makes GitHub refuse the merge if a push landed after the checks. It makes a merge commit, as the `--no-ff` rule asks.
- Only PRs into `develop`. A `release/*` or `hotfix/*` PR into `main` is merged by hand: it needs a tag next, and deleting its branch would break its merge back into `develop`.
- Runs on `ubuntu-latest` with no checkout: the job holds a write token and needs only the API.
- Permissions: `actions: read` to find the runs; `contents: write` and `pull-requests: write` to merge and delete the branch. This widens ADR 0007's rule that only `release.yml` gets `contents: write`.
- `test.yml` names its e2e step `e2e`, and the merge check looks for that name. If the step is renamed, PRs stop merging on their own; nothing merges without e2e.
- A per-PR concurrency group: when `checks` and `test` finish close together, the two jobs run one after the other, and the second sees the PR merged.
- Agents add the label only when the user asks for the merge.

### Consequences

- Good: "merge when green" no longer waits for a person. The label is the approval.
- Good: the rule is stricter than reading the PR page: e2e must have run on the exact head commit.
- Bad: a merge made with `GITHUB_TOKEN` starts no workflow. Nothing runs on a push to `develop` or `main` today (`release` runs on tag pushes), so nothing is lost now. A future workflow on push to `develop` would not run after these merges unless this one merges with a GitHub App or personal token.
- Bad: the merge is made by `github-actions[bot]`, not by the user.
- Bad: about one Linux minute for each successful `checks` or `test` run on any PR, labelled or not, because the `workflow_run` event does not carry the PR's labels. Roughly two minutes per code push and one per docs push, out of the 2,000-minute pool.
- Bad: CI tested the PR merged with `develop` as it was then. If `develop` moved since, the merge is not re-tested. A merge by hand has the same gap today.
- Bad: `workflow_run` and `pull_request_target` only use the file on the default branch. The workflow cannot be tried before it is on `develop`, and a change to it takes effect only once merged.
- Neutral: the `merge-when-green` label must exist in the repo.
- Neutral: a merge conflict or refused merge shows as a failed `merge-when-green` run, and the PR stays open.
