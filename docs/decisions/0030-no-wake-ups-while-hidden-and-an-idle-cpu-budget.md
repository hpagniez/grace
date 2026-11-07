---
status: accepted
date: 2026-11-07
decision-makers: Tusquito
---

# No wake-ups while hidden, and an idle CPU budget while shown

## Context and Problem Statement

[ADR 0023](0023-run-lightly-as-a-release-requirement.md) made running lightly Grace's first priority. The rules that came with it, in AGENTS.md and the [light-and-fast spec](../specs/2026-10-31-light-and-fast.md) (R6.2, R6.3), say "idle costs nothing" and ask every spec to show "how idle stays at zero CPU".

Grace idles at about 230 ms of CPU a minute with the window shown, about 0.4 % of one core ([idle CPU note](../research/2026-10-31-idle-cpu-shown.md)). Most of it is Chromium's own housekeeping, which Grace cannot remove. That much CPU does not make the machine feel slower: the scheduler gives it to other apps without anyone noticing. What does cost something is a process that keeps waking up while nobody can see its window: each wake-up keeps the CPU out of its deep sleep states, and on a laptop where Grace stays open all day that drains the battery well beyond what the CPU percentage suggests.

Read strictly, "zero CPU" also pushes toward custom code that is costly to keep. The GPUI prototype ([#275](https://github.com/hpagniez/grace/pull/275)) reached 10 ms a minute only by patching GPUI's X11 refresh loop ([#268](https://github.com/hpagniez/grace/issues/268)), a fork of a pre-1.0 framework that every update would have to rebase.

What should the idle rule ask for?

## Considered Options

1. Keep "idle costs nothing", shown and hidden.
2. No wake-ups while the window is minimised or hidden, and a small CPU budget while it is shown.
3. A CPU budget only, the same shown and hidden.

## Decision Outcome

Chosen: option 2, by the owner ([#276](https://github.com/hpagniez/grace/issues/276)).

- While no run is going and the window is minimised or hidden, Grace does not wake: no timer, poll, animation or redraw. The bench measures it as `idle.cpu.minimised`.
- While no run is going and the window is shown, Grace stays within a small CPU budget: the `idle.cpu.shown` limit in `bench/limits.json`. Code Grace writes still adds no timer, poll or animation of its own while idle; the budget is room for the engine's housekeeping, not for features.
- Every design spec's `## Cost` section says how the feature keeps to both.
- No limit in `bench/limits.json` changes. A later change to either limit follows ADR 0023: down by an ordinary commit, up only through a new decision record.

This replaces the "idle costs nothing" rule of the light-and-fast spec (R6.2, R6.3) and AGENTS.md. ADR 0023 stays as it is.

Why not the others:

- Option 1 costs custom code, such as a patched UI framework, for a gain nobody feels on the machine.
- Option 3 lets a hidden Grace wake all day, which is the case that drains a battery.

### Consequences

- Good: the rule targets what people feel (battery, a machine that sleeps) and that Grace can control.
- Good: a stack change is no longer held to an engine that never wakes while shown; a framework's own refresh loop can stay unpatched if it fits the budget.
- Bad: the shown budget is a number, not zero: a feature can spend some of it. The `perf` comment shows that on its PR, and a worse number still needs its explanation in the PR body.
