---
status: accepted
date: 2026-10-28
decision-makers: Tusquito
---

# Make the repository public

## Context and Problem Statement

Grace is a free, open-source runner, but `hpagniez/grace` is private on GitHub Free. [ADR 0007](0007-github-actions-ci-cd-on-the-free-private-plan.md) fitted CI into that plan's 2,000 Actions minutes (about 1,000 on Windows), and asked for a new record when the repo goes public: add CodeQL, dependency review and branch protection, and drop the minute-saving rules. The October minutes ran out on 2026-10-28: CI could not run on #99, which was merged by hand.

A public repo on GitHub Free adds:

- Standard GitHub-hosted runners with no minute cap. `windows-latest` gets 4 CPUs and 16 GB RAM instead of 2 and 8 ([CI test speed note](../research/2026-10-27-ci-test-speed.md)).
- Rulesets and branch protection, and GitHub auto-merge.
- Secret scanning with push protection, private vulnerability reporting, CodeQL default setup and the dependency review action.

It also exposes everything already pushed: code, docs, issues, pull requests and the commit history with its author emails. Anyone can fork the repo and open pull requests. `merge-when-green` ([ADR 0011](0011-merge-labelled-prs-when-ci-is-green.md)) holds a write token and merges a labelled PR once CI is green on its head. The label stays when new commits come in, and the author of a fork PR can push, then mark the PR ready for review, which starts the merge.

How does the repo go public safely, and what changes in CI?

## Considered Options

1. Stay private. The minute cap stays, and "open source" stays untrue.
2. Go public. Keep the CI workflows as they are, never auto-merge a fork PR, and turn on GitHub's free security features.
3. Go public, and at the same time move e2e back onto every PR into `develop` and replace `merge-when-green` with GitHub auto-merge.

## Decision Outcome

Chosen: option 2. Option 3's changes each need their own look: e2e makes every PR into `develop` wait longer, and a required `test` check would block the docs-only PRs and back-merges that skip `test` ([ADR 0017](0017-e2e-on-ci-only-for-prs-into-main.md)). They wait for their own record.

### In the repo, before the switch

- `merge-when-green` merges only PRs whose head branch is in this repo. A fork PR is reviewed and merged by hand.
- `SECURITY.md`, `CONTRIBUTING.md` and `CODE_OF_CONDUCT.md` (Contributor Covenant 2.1). Conduct and security reports both go through GitHub's private reporting form, so no email address is published.
- Issue forms for bugs and features in `.github/ISSUE_TEMPLATE/`. Blank issues are off on the web; issues created through the API, as agents do for the backlog, are not affected.
- AGENTS.md says the repo is public.

### The switch, by hand, in this order

1. Decide whether the author email of the existing commits is rewritten. A rewrite changes every commit ID and the `-rc` tags, so it happens before step 3 or never. Either way, commit with the GitHub `noreply` address from now on and turn on "Block command line pushes that expose my email".
2. Merge the PR that adds this record into `develop`.
3. Settings → General → Change visibility → Public.
4. Settings → Advanced Security: turn on secret scanning with push protection, private vulnerability reporting, Dependabot alerts and CodeQL default setup.
5. Settings → Actions → General: require approval for all external contributors before fork PR workflows run.
6. Settings → Rules → Rulesets: one ruleset on `main` and `develop` that restricts deletions and blocks force pushes. No required checks.
7. The [Grace project](https://github.com/users/hpagniez/projects/2) board: Settings → Visibility → Public.
8. Add topics and a website to the repo's About box.

### What stands

ADR 0007's workflows, as changed by ADRs 0011, 0016 and 0017, stand. The minute-saving rules (no `test` run on docs-only PRs, e2e on CI only for PRs into `main`) now save waiting time, not money. Moving e2e back onto PRs into `develop` is left to a new record. The dependency review action is not added: `checks` already fails a PR on a high `npm audit` advisory.

This supersedes ADR 0007's premise that the repo stays private on GitHub Free, and its consequence that the repo has no CodeQL or secret scanning. It narrows ADR 0011: a PR from a fork never merges itself.

### Consequences

- Good: CI never stops for lack of minutes, and Windows jobs run on 4 CPUs. ADR 0016's split was sized for 2 and still works.
- Good: push protection stops a pushed token, vulnerabilities can be reported privately, and no one can force-push or delete `main` or `develop`, as AGENTS.md asks.
- Good: outside contributors have a guide, issue forms and a code of conduct.
- Bad: what is pushed once is public for good. Making the repo private again does not remove forks and clones.
- Bad: checks still do not block merging a red PR; the reviewer must check the status.
- Bad: a fork PR needs the maintainer twice: to approve its CI runs and to merge it.
- Neutral: the zip is still unsigned, so SmartScreen still warns on first start.
