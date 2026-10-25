---
status: accepted
date: 2026-10-25
decision-makers: Tusquito
---

# Name release and hotfix branches after their tag

## Context and Problem Statement

[ADR 0003](0003-conventional-commits-and-git-flow.md) names release and hotfix branches `release/X.Y.Z` and `hotfix/X.Y.Z`, while the tags on `main` are `vX.Y.Z`. [ADR 0007](0007-github-actions-ci-cd-on-the-free-private-plan.md)'s PR check only looked at the `release/` and `hotfix/` prefixes. Before cutting 1.0.0, should the branch name match the tag exactly?

## Considered Options

1. `release/vX.Y.Z` and `hotfix/vX.Y.Z`: the branch carries the tag it produces.
2. Keep `release/X.Y.Z` and `hotfix/X.Y.Z`.

## Decision Outcome

Chosen: option 1. The branch is named after the tag it produces, e.g. `release/v1.0.0` for tag `v1.0.0`.

- This amends the branch names of ADR 0003. Everything else in 0003 still holds.
- It also tightens ADR 0007's branch check: `scripts/check-pr.mjs` rejects a `release/*` or `hotfix/*` branch whose name is not `vX.Y.Z` with three numbers.

### Consequences

- Good: the branch, the tag and `package.json` read the same, with one less thing to translate when tagging.
- Good: a typo such as `release/1.0` or `release/latest` fails CI before the release PR is merged.
- Neutral: pre-release suffixes such as `-rc.1` are not accepted. Allow them in the pattern if pre-releases are ever cut.
