---
status: proposed
date: 2026-10-28
decision-makers: Tusquito
---

# Track the backlog in GitHub Issues and a project board

## Context and Problem Statement

The backlog is a table in `docs/features.md`: 45 rows (23 done, 22 idea), with notes for 7 of them. Every spec or feature PR edits it, so parallel branches collide on one file (the F44/F45 research PR and the rename PR both edited it). Agents move a row's status by hand, and forget. Once the repo is public, outside users will report bugs and ideas as GitHub issues (F35) anyway.

The repo is `hpagniez/karate-kid`, on the maintainer's personal account on GitHub Free. The account was renamed from `Tusquito` to `hpagniez`; `gh` still labels its saved login `Tusquito`. What GitHub offers, checked on 2026-10-28:

- A project (Projects v2) belongs to a user or an organisation, not to a repo. Its board columns must be a single-select or iteration field. Labels cannot be columns, and a table view cannot group by labels.
- Built-in project workflows set the `Status` field when an item is added, when a PR is linked to an issue (added November 2025), and when an item is closed. They are set up in the web UI only. GitHub Free allows one auto-add workflow. It adds only items created or updated after it is turned on.
- Moving a card from the CLI takes two calls: `gh project item-add` returns the issue's item ID, then `gh project item-edit` sets the field with the project, field and option IDs. Both need a token with the `project` scope. `gh` keeps one token per user account of the computer, not per worktree, so granting the scope once (`gh auth refresh -h github.com -s project`) covers every agent on that computer.
- A closing keyword (`Closes #N`) in a PR body links the PR to the issue. Merging the PR into the default branch (`develop`) closes the issue.

## Considered Options

1. Keep `docs/features.md`.
2. Issues with labels only, no project.
3. Issues, with priority as labels, plus a project whose `Status` field the built-in workflows move, and agents only where no workflow fires.
4. Issues plus a project in which agents edit every field (priority, status, Karate surface) on every change.

## Decision Outcome

Chosen: option 3.

- One issue per feature. Its body holds the feature, a `Karate surface:` line, a `Spec:` line (the file on `develop`, or "none") and any notes.
- Priority: labels `must`, `should` and `could`.
- One project, `Grace`, owned by `hpagniez`. Its `Status` field has the options `idea`, `building` and `done`:
  - Auto-add to project: issues of `hpagniez/karate-kid` (filter `is:issue`).
  - Item added to project: `idea`.
  - Pull request linked to issue: `building`.
  - Item closed: `done`.
- `specced` is no longer a status. The issue's `Spec:` line shows it.
- A PR that builds a feature says `Closes #N`. A spec-only PR, or one of several PRs for a feature, says `Part of #N`.
- Agents leave the status to the workflows. They move a card by hand only where no workflow fires: a feature built over several PRs goes to `building` when its first `Part of #N` PR opens. `AGENTS.md` holds the project, field and option IDs for that.
- `docs/features.md` becomes a frozen map from F01–F45 to their issues, because specs, plans, research notes, decision records and commits name features by those IDs. New features have only an issue number.
- Specs, plans, decision records, research notes and check records stay Markdown in the repo.
- This supersedes the `features.md` line of ADR 0002's layout and its mitigation ("update `docs/features.md` ... in the same change"). The rest of ADR 0002 stands.

Why not the others:

- Option 1 keeps the conflicts and the manual status, and outside users cannot see or add to it.
- Option 2 has no board: labels cannot be board columns.
- Option 4 puts two `gh project` calls with three IDs into every status change, and the board drifts whenever an agent forgets one. The workflows make the same moves from PR events, with nothing to forget. Priority and Karate surface gain nothing as project fields: labels filter as well, and the body holds the text.

### Consequences

- Good: no more conflicts on the backlog. The status follows the PRs without anyone editing it.
- Good: outside users can file and follow issues, and F35's feedback can land in the same place.
- Bad: the backlog is no longer in git. Agents need the network and `gh` to read it, and a status change is not reviewed in a diff.
- Bad: a feature built over several PRs needs its card moved by hand, because only a closing keyword links a PR to an issue. If an agent forgets, it stays `idea` until the PR with `Closes #N` opens.
- Bad: an issue closed as "not planned" also shows as `done`. Its close reason tells them apart.
- Neutral: the workflows are set up by hand in the web UI, so a new project means repeating that setup.
- Neutral: issues and PRs share one number sequence, so the first issue is #41 or later.
