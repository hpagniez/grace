---
paths:
  - "docs/**"
---

# Docs

## Docs map

| Path | What it holds |
|---|---|
| [Issues](https://github.com/hpagniez/grace/issues) and the [Grace project](https://github.com/users/hpagniez/projects/2) | Backlog: one issue per feature, priority as label must (first release), should or could; status idea → building → done on the board |
| [docs/features.md](../../docs/features.md) | Frozen map of the old backlog IDs F01–F45 to their issues |
| [docs/inventory/karate-core.md](../../docs/inventory/karate-core.md) | Every open-source Karate v2 feature, tagged by GUI use; §19 = how the app drives Karate |
| [docs/inventory/commercial.md](../../docs/inventory/commercial.md) | What Karate Labs' paid tools offer, and the gaps |
| [docs/research/](../../docs/research/) | Dated research notes with sources |
| [docs/decisions/](../../docs/decisions/) | Decision records (MADR 4.0 minimal) |
| `docs/specs/` | Design specs, one per feature, written just before building it |
| `docs/checks/` | Dated check records of the packaged app: what was run, on which machine, the result, and what still needs a person |
| `docs/plans/` | Implementation plans, one per spec, task by task (overrides the superpowers default `docs/superpowers/plans/`) |
| `docs/process/` | Steps that are followed, not designed: releases and hotfixes |

## Frontmatter and indexes

Spec: [docs/specs/2026-11-13-docs-index.md](../../docs/specs/2026-11-13-docs-index.md).

- `docs/specs/`, `docs/plans/`, `docs/checks/`, `docs/research/` and `docs/decisions/` each hold a generated `README.md`, the folder's index: one line per doc with its title, issue and summary. Read the index first, and open only the docs whose line matches.
- What the index says of a doc: a `built` spec is how the feature behaves; a `built` plan is history; anything `superseded` is history. A check or a research note is a record of its date.
- Every spec and plan starts with frontmatter: `status`, `issue` and `summary`. A check or a research note (a file, or the `README.md` of a folder under `docs/research/`) has `summary` only. Decision records keep theirs as it is.
- `status` is `draft` (written, not approved), `approved` (approved by the owner, not fully built), `built` (the PR that finishes it is merged) or `superseded` (replaced or given up; the text says by what). A new spec or plan starts as `draft`; the session that gets the owner's approval sets `approved`; the PR that says `Closes #N` sets its spec and plan to `built`, and `cargo xtask check-pr` fails it otherwise; a doc with an `issue` the PR leaves open stays as it is. The status has no other home: no `Status:` line in the text.
- `issue` is the number, `373`, or a list, `[64, 231]`, never with `#`. Leave it out only when the work has no issue.
- `summary` is one line in double quotes, at most 160 characters, with no `"` and no `\` inside. It says what the doc decides or found, not what it is about: a spec's behaviour, a plan's outcome, a check's result with its number, a research note's verdict.
- After adding a doc or changing its frontmatter or its title, run `cargo xtask docs` and commit the indexes with it. `cargo xtask check-rules` fails on a missing field or an index that is out of date. On a merge conflict in an index, run `cargo xtask docs` and commit the result.

## Doc rules

- Design specs go in `docs/specs/YYYY-MM-DD-<topic>.md` (this overrides the superpowers default `docs/superpowers/specs/`). Write requirements, then acceptance checks: in Gherkin for behaviour someone using the app sees, as a plain list otherwise. End with a `## Cost` section: what the feature adds to memory, CPU, start time or size, idle and during a run, and how it keeps to the idle rule ([Performance](../../AGENTS.md#performance)); one line, "Cost: none", unless it adds a dependency, a process, a timer or work on the run path. A PR that adds a dependency states its size.
- Every feature has an issue with a priority label. Its body holds the feature, a `Karate surface:` line and a `Spec:` line. New features get only an issue number, never a new F-ID; cite them as `#N`. When a spec is written, point the issue's `Spec:` line at the file on `develop`. How: `.claude/rules/ci.md`. A PR that builds a feature says `Closes #N`; a spec-only PR, or one of several, says `Part of #N`. The project's workflows move the status. Which PRs need an issue is in [AGENTS.md](../../AGENTS.md#pull-requests).
- A choice that is hard to reverse (a dependency, an architecture, a file format) gets a new `docs/decisions/NNNN-title.md`. Never edit an accepted record; supersede it. A change to how agents work is a rule change in the rules files, not a decision record.
- A Project's plan has a `## Slices` section: a table of its slices in build order, one PR each, with what the slice does, the parity rows or checks that end it, and the model that builds it (`sonnet`; `opus` for a slice the plan marks hard). `cargo xtask check-pr` fails a Project PR whose plan has none.
- Building a Project: the session that wrote the plan does not build it. Once the owner approves the plan, which is his word to start, it starts one fresh session for the first slice, on that slice's model, with a brief that names the plan, the slice, its rows and what done looks like. That session builds the slice as a Feature and opens its PR. When the PR is green, the planning session tells the owner and waits for his word to merge, then starts the next slice from the merged `develop`. It keeps the tracking issue's State current. Where no tool starts a session (a cloud session, a plain terminal), it builds the slices itself, one PR at a time.
- The state of long work lives in its tracking issue's body, edited in place: where each phase stands, what is next, the questions for the owner. No hand-over note as a comment.
