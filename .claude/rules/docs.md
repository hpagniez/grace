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

## Doc rules

- Design specs go in `docs/specs/YYYY-MM-DD-<topic>.md` (this overrides the superpowers default `docs/superpowers/specs/`). Write requirements, then acceptance checks: in Gherkin for behaviour someone using the app sees, as a plain list otherwise. End with a `## Cost` section: what the feature adds to memory, CPU, start time or size, idle and during a run, and how it keeps to the idle rule ([Performance](../../AGENTS.md#performance)); one line, "Cost: none", unless it adds a dependency, a process, a timer or work on the run path. A PR that adds a dependency states its size.
- Every feature has an issue with a priority label. Its body holds the feature, a `Karate surface:` line and a `Spec:` line. New features get only an issue number, never a new F-ID; cite them as `#N`. When a spec is written, point the issue's `Spec:` line at the file on `develop`. How: `.claude/rules/ci.md`. A PR that builds a feature says `Closes #N`; a spec-only PR, or one of several, says `Part of #N`. The project's workflows move the status. Which PRs need an issue is in [AGENTS.md](../../AGENTS.md#pull-requests).
- A choice that is hard to reverse (a dependency, an architecture, a file format) gets a new `docs/decisions/NNNN-title.md`. Never edit an accepted record; supersede it. A change to how agents work is a rule change in the rules files, not a decision record.
- The state of long work lives in its tracking issue's body, edited in place: where each phase stands, what is next, the questions for the owner. No hand-over note as a comment.
