---
status: accepted
date: 2026-11-03
decision-makers: Tusquito
---

# Raise the open-project node limit for the one-line filter bar

## Context and Problem Statement

[#127](https://github.com/hpagniez/grace/issues/127) keeps the filter bar to one line ([spec](../specs/2026-11-03-compact-filter-bar.md)). With the bench project's 54 tags the bar took 4 to 6 lines; now it takes one, and the virtual tree under it draws the rows it gets back. On PR #244 the `perf` workflow measured `page.nodes.open` at 2664 against 2292 on its base (+372), over the limit of 2473 set in [ADR 0023](0023-run-lightly-as-a-release-requirement.md).

The cause was measured in the built app at 1184 x 761: the one-line bar leaves the tree 403 px and 27 drawn rows, 2731 nodes; the same build with the bar forced back to wrapping (187 px) leaves 253 px and 21 rows, 2432 nodes. The extra nodes are tree rows on screen, the point of the change.

Should the limit move up, or should the feature give the room back?

## Considered Options

1. Raise `page.nodes.open` and `page.nodes.open@10k` by the measured 372 nodes.
2. Keep the limit, and the tree's height as it was: the bar's room left empty, or given to something else.
3. Keep the limit and draw fewer nodes per tree row.

## Decision Outcome

Chosen: option 1, by the owner.

- `page.nodes.open`: 2473 to 2845 (2473 + 372). Noise stays 118.
- `page.nodes.open@10k`: 2473 to 2845. The 10,000-test project has the same 54 tags and the same window, so the bar gives back the same rows; it was not measured here, only in a release benchmark.
- `page.nodes.run` (7558 against 7654) and `page.nodes.run@10k` stay: under their limits.
- The run memory and CPU measures moved too on the `perf` runner (+51 MB, +1391 ms), from the same rows updating during a run. Their limits are judged on the reference machine only, where `run.cpu.app`'s noise is 1281 ms; the next release benchmark measures them there.

Why not the others:

- Option 2 undoes #127: the issue exists to give the tree its height back.
- Option 3 is work of its own on the tree, unrelated to the filter bar, and would not make these nodes free: they are rows a person sees.

### Consequences

- Good: the limit matches what the app now draws for a person; a later regression in the tree's rows still shows against it.
- Bad: 372 more nodes are allowed with a project open. A lower number from the next release benchmark lowers the limit with an ordinary commit.
