---
status: accepted
date: 2026-10-31
decision-makers: Tusquito
---

# Run lightly as a release requirement

## Context and Problem Statement

Grace stays open all day beside IDEs, browsers, containers and chat, so it must never be the reason the machine feels slow. Nothing guards that. The [big-project note](../research/2026-10-29-big-project-perf/README.md) and the [virtual-lists check](../checks/2026-10-30-virtual-lists-perf.md) measured Grace by hand, once. There is no limit, and no CI job measures anything, so a PR can double Grace's memory or start time and still pass. The [spec](../specs/2026-10-31-light-and-fast.md) has the detail.

How does Grace keep running lightly as it changes, without holding a merge back because a number is worse?

## Considered Options

1. A. Rules and a release check only: this record, rules in AGENTS.md, `npm run bench`, a limits file and the release gate. On normal PRs, agents paste numbers from their own machines.
2. B. A, plus a `perf` workflow on every code PR: it builds the PR's base and head on one runner, benchmarks both in turns and posts the numbers.
3. C. B, plus a history chart of every `develop` build.

## Decision Outcome

Chosen: option B.

- Grace's priorities, in order: run lightly (memory, CPU and a smooth screen, idle and during a run, Java included), then be fast (start and response), then be small (download and install size). A lower priority never wins over a higher one. A change of stack (shell, UI framework, runtime) is judged first on running lightly, measured with `npm run bench` against the current build.
- `bench/limits.json` holds a limit for each measure: today's numbers (`develop` at `e3cfe3b7`, see the [baseline](../checks/2026-10-31-bench-baseline.md)) plus a noise margin. Times, CPU and memory are judged against limits only on the reference machine named in that file.
- Every code PR shows what it costs. `perf` posts one comment, updated on each push. A merge waits for those numbers, never for them to be good: `perf` fails only when the bench cannot measure. The PR body explains a worse number.
- A release stops on a miss. Before a `release/*` or `hotfix/*` PR into `main`, the release benchmark runs on the reference machine, the release build beside the previous release's zip. A miss is a measure over its limit, or worse than the previous release beyond its noise margin. The check record's `## Performance` section then needs an `### Analysis`, and the owner decides with `Decision: go` or `Decision: no-go`. An agent writes the analysis, never the decision. `scripts/check-pr.mjs` works out pass or miss from the committed bench JSON and the limits file, and fails the PR when the record says otherwise.
- A limit moves down by an ordinary commit and up only through a new decision record. `scripts/check-pr.mjs` fails a PR that raises a limit without adding a file under `docs/decisions/`.

Why not the others:

- Option A: the numbers come from busy machines, and only when someone remembers.
- Option C: it needs more infrastructure. It can come later.

This narrows ADR 0011: a labelled PR into `develop` also waits until `perf` has finished with success on its head. Raising a limit in `bench/limits.json` needs a new decision record.

### Consequences

- Good: the numbers show up on every code PR without anyone remembering, and the base and the PR are measured on one VM, in turns, which cancels most of the noise.
- Good: no release ships a regression that nobody explained and the owner did not accept, and limits cannot drift up quietly.
- Bad: each code PR costs about 20 minutes of a Windows runner, free on a public repo. A labelled merge takes about that long after a push instead of about 6.
- Bad: a release adds about an hour on the reference machine: two builds, 5 rounds each, at 2,000 and 10,000 tests.
- Bad: a `perf` run that could not measure holds the merge until it is re-run or the bench is fixed, because no numbers were shown.
- Bad: nothing can check who wrote `Decision: go`. Agents commit under the owner's git identity and use the owner's `gh` token, so that part rests on the AGENTS.md rule.
- Bad: the bench drives the app through test ids, data attributes and settings keys. A PR that renames one the bench uses updates the bench in the same PR.
