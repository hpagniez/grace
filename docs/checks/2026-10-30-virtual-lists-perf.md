# App check: virtual list speed at 10,000 tests (#125, #91)

Date: 2026-10-30
Branch: `feature/virtual-tree` at 36484699. The only change since is this record and its harness.
Machine: Windows 11 Pro 10.0.26300 (x64), AMD Ryzen 7 5800X (8 cores, 16 threads), 32 GB, NVIDIA GeForce RTX 4060 Ti, Node 24.15.0, npm 11.16.0, Electron 45.0.0, bundled Temurin 21.0.12.1 and Karate 2.1.4. The machine was not quiet: see [Machine load](#machine-load).
Spec: [Virtual lists and keyboard navigation](../specs/2026-10-30-virtual-lists.md), R6. Method: sections 1 and 8 of the [big-project performance note](../research/2026-10-29-big-project-perf/README.md).

**Result: R6 is not met at 10,000 tests.**
- Fails: moving with Down (R6.3), Run all (R6.4: "Finished" 7.3 s after Karate's last test against a limit of 2 s, and frames at or over the 500 ms limit), and scrolling the tree during Run all (R6.6).
- At the limit: opening the top folder (R6.2, 95 ms in rounds 1 to 3, 101 ms over all six rounds).
- Passes: opening (R6.1), closing the top folder, selecting with the mouse, the HTTP tab (R6.5), and "nothing is slower at 2,000 tests" (R6.7).

## What was run

- `npm run package` at 36484699, then the note's harness, copied to [`2026-10-30-virtual-lists-perf/`](2026-10-30-virtual-lists-perf/) with the selectors of the virtual lists (see [Harness changes](#harness-changes)). Window 1400 × 900, as in the note.
- Projects from the note's `generate.mjs`: 2,000 tests (5 features per module: 600 HTTP calls, 2,449 tree rows) and 10,000 tests (25: 3,000 calls, 12,049 rows).
- Per round, from the repo root: `PERF_PROJECT=<project> PERF_OUT=<file>.jsonl SCROLL_OUT=<file>.jsonl npx playwright test --config docs/checks/2026-10-30-virtual-lists-perf/playwright.config.ts`. One round is `perf.spec.ts` (open, folders, selection, Down, two Run alls, the HTTP tab) and `scroll.spec.ts` (60 wheel notches idle, up, and during Run all).
- 10,000 tests: 6 rounds in a row, the R6 figure being the median of rounds 1 to 3. Rounds 4 to 6 were added because three figures are close to their limits; the median of all six is given beside it. They agree on every verdict but the top folder's, which is at its limit in both.
- 2,000 tests (R6.7): 3 rounds of this build and 3 of `develop` before this change, run in turns (this, develop, develop, this, this, develop). `develop` is `b9a8a5e2` (the merge of the spec), built with `npm run package` in a temporary worktree and measured with the note's own, unchanged harness; the worktree is deleted.
- `npm test` and `npm run e2e` were not run while measuring.
- "Finished after Karate" is the time from the last depth-0 `SCENARIO_EXIT` in Karate's event file to "Finished" shown, on one clock (the note's method).

## R6 at 10,000 tests

Medians. The prototype column is the note's B–E + V build, as bracketed in the spec. "Rounds 1–3" is the R6 figure.

| R6 | Measure | Limit | Prototype | Rounds 1–3 | All 6 | Result |
|---|---|---|---|---|---|---|
| 1 | Open: longest renderer task | under 500 ms | 290 ms | 368 ms | 372 ms | pass |
| 1 | DOM nodes after open | under 10,000 | 3.7k | 3,252 | 3,252 | pass |
| 2 | Close the top folder (12,049 rows) | under 100 ms | 49–52 ms | 65 ms | 66 ms | pass |
| 2 | Open the top folder | under 100 ms | 49–52 ms | 95 ms | 101 ms | at the limit |
| 3 | Select a test with the mouse (median of 20 per round) | under 50 ms | 11 ms | 30 ms | 31 ms | pass |
| 3 | Move with Down (median of 10 per round) | under 50 ms | not measured | 65 ms | 61 ms | **fail** |
| 4 | Console tab: "Finished" after Karate's last test | within 2 s | 0.2–1.2 s | 7.3 s | 12.1 s | **fail** |
| 4 | HTTP tab: "Finished" after Karate's last test | within 2 s | 0.2–1.2 s | 7.6 s | 11.9 s | **fail** |
| 4 | Console tab: longest frame during Run all | no more than 500 ms | 342–383 ms | 500 ms | 542 ms | **fail** |
| 4 | HTTP tab: longest frame during Run all | no more than 500 ms | 342–383 ms | 500 ms | 554 ms | **fail** |
| 5 | Open the HTTP tab (3,000 calls) | under 100 ms | 23 ms | 28 ms | 31 ms | pass |
| 6 | Scroll the tree during Run all: longest frame | under 100 ms | 25–33 ms | 117 ms | 117 ms | **fail** |

Notes on the verdicts:
- A frame is the gap between two `requestAnimationFrame` calls, so 500 ms is 30 frames of 16.7 ms. Rounds 1 to 3 gave 500, 500 and 583 ms (Console) and 525, 492 and 500 ms (HTTP): at the limit, not under it.
- Down: the note's harness only clicks. The probe added here presses ArrowDown on the selected row ten times, from the top folder, and waits for the next row to be selected and painted. Selecting with the mouse is under its limit. R6.3 reads "with the mouse, or move with Down": taken as both, it fails.
- The speed of the machine moves the run figures a lot. Karate's last test came at 21.2 to 26.7 s (Console) here against 18.2–19.0 s in the note, and the delay after it grew with that: 6.6 to 16.1 s over the six Console rounds. A quiet machine may give less, but not a tenth of it: see the loaded rounds below, where the quietest Run all (Karate at 19.0 s) still ended 4.7 s late.

All rounds, in order:

| Round | Karate's last test, Console / HTTP | Close / open top folder | Select / Down | Finished after Karate, Console / HTTP | Longest frame, Console / HTTP | Scroll during run: longest frame | Open: longest task |
|---|---|---|---|---|---|---|---|
| 1 | 21.2 / 20.8 s | 66 / 106 ms | 30 / 58 ms | 7.30 / 7.32 s | 500 / 525 ms | 125 ms | 368 ms |
| 2 | 21.3 / 23.2 s | 62 / 95 ms | 31 / 65 ms | 6.61 / 7.61 s | 500 / 492 ms | 117 ms | 360 ms |
| 3 | 24.1 / 23.5 s | 65 / 95 ms | 30 / 65 ms | 10.55 / 13.96 s | 583 / 500 ms | 117 ms | 375 ms |
| 4 | 25.0 / 23.5 s | 72 / 124 ms | 30 / 61 ms | 15.46 / 11.08 s | 533 / 600 ms | 100 ms | 406 ms |
| 5 | 25.2 / 23.4 s | 54 / 96 ms | 31 / 60 ms | 16.10 / 21.62 s | 550 / 583 ms | 108 ms | 369 ms |
| 6 | 26.7 / 25.5 s | 70 / 111 ms | 33 / 60 ms | 13.68 / 12.80 s | 567 / 633 ms | 142 ms | 463 ms |

Other figures of rounds 1 to 3 (no limit):
- Rows drawn after open: 29. DOM nodes after open 3,252 and at the end of both Run alls 3,201. JS heap 87 MB after open, 124 MB at the end (the note's prototype: 74 MB after open).
- Renderer busy during open: 918 ms (the prototype's 700–900 ms). Script time during the first round's Run all: 20.5 s (Console) and 21.7 s (HTTP) in runs of 28.5 and 28.2 s. The renderer was busy for about three quarters of the run, and the bar fell behind Karate by 7.1 and 7.3 s at the end (the prototype: 0.7–1.2 s and 0.19–0.24 s). This is the one result that explains R6.4 and R6.6: during Run all the renderer has no time to spare for a scroll or a frame.
- Idle scroll of 60 notches: longest frame 9 ms down and 21 ms up, main-thread paint 144 ms, hit tests 14 ms (the prototype: 13 ms hit tests, 9–33 ms frames). During Run all, 91 of about 480 frames took over 50 ms.
- HTTP tab: group chips 33–40 ms, select a call 15 ms, narrow to one feature 37 ms, widen back to the top folder 170 ms.
- Not profiled, a likely cause for whoever takes it: `deriveStatus` (`src/shared/status.ts`) walks the whole subtree of a folder, with a `Set` per node, and the tree calls it twice for every drawn folder row on each status change (in the row's `attrs`, for its accessible name, and in `Row`'s `status` memo). The top folder alone is 12,049 nodes per call. This is the folder roll-up of [#126](https://github.com/hpagniez/grace/issues/126) that the note and the spec name as the next limit, but the delay here is larger than the prototype's.

## At 2,000 tests (R6.7)

Medians of 3 rounds each, in turns. The note's B–E column is the reference named in R6.7; its B–E + V column is the prototype.

| Measure | `develop` before | This build | Note B–E | Note B–E + V |
|---|---|---|---|---|
| Open: longest task / renderer busy | 540 / 941 ms | 111 / 370 ms | 473 / 882 ms | 91 / 311 ms |
| DOM nodes / listeners / JS heap after open | 123.8k / 14,856 / 87 MB | 2.5k / 338 / 18 MB | 123.4k / 14,856 / 87 MB | 2.5k / 337 / 16 MB |
| Open the top folder (2,449 rows) | 561 ms | 35 ms | 450 ms | 23 ms |
| Select a test (median of 20) | 57 ms | 11 ms | 35 ms | 11 ms |
| Console run: click to "Finished" / Karate's last test | 8.77 / 8.12 s | 8.22 / 7.99 s | 7.3 / 7.0 s | 6.7 / 6.4 s |
| … "Finished" after Karate / bar full after Karate | 0.35 / 0.09 s | 0.20 / 0.11 s | 0.3 / 0.11 s | 0.3 / 0.08 s |
| … frame rate / longest frame | 55 fps / 892 ms | 101 fps / 108 ms | 76 fps / 608 ms | 114 fps / 67 ms |
| HTTP run: click to "Finished" / Karate's last test | 8.73 / 8.31 s | 8.17 / 7.72 s | 7.5 s / n/a | 6.6 s / n/a |
| … "Finished" after Karate / bar full after Karate | 0.42 / 0.10 s | 0.41 / 0.10 s | n/a / 0.11 s | n/a / 0.09 s |
| … frame rate / longest frame | 48 fps / 925 ms | 100 fps / 117 ms | 70 fps / 717 ms | 116 fps / 58 ms |
| Open the HTTP tab (600 calls) | 125 ms | 18 ms | 118 ms | 15 ms |
| Idle scroll down: main-thread paint / hit tests | 219 / 189 ms | 136 / 15 ms | 170 / 133 ms | 104 / 10 ms |
| Idle scroll: longest frame (down / up) | 25 / 25 ms | 14 / 13 ms | 25 ms | 17 ms |
| Scroll during Run all: longest frame | 125 ms | 33 ms | 100 ms | 17 ms |

Result: pass. Every row is equal or better than `develop` measured beside it. The one row that is level, not better, is the HTTP run's "Finished" after Karate: 0.41 s here, 0.42 s on `develop`, the rounds spreading from 0.24 to 0.54 s. Against the note's B–E column every row is better too, except the absolute run times (8.2 s here, 7.3 and 7.5 s there): Karate's own last test came at 7.7 to 8.1 s on this machine against 7.0 s, and the time left after Karate is the same or shorter.

## Machine load

Another program used a large part of the machine all through the measuring: a live video in Edge, whose GPU process took about 9 of the 16 threads at times (46 CPU-seconds in 5 s). With no test running, the system CPU load read 60 to 90 % around the first rounds, and 26 to 53 % when read before the 2,000-test rounds and before rounds 4 to 6. Rounds 1 to 6 were started once it had eased. It could not be stopped.

The first three rounds at 10,000 tests were taken before it eased, and are not used for any verdict above:

| Round | Karate's last test, Console / HTTP | Open top folder / Down | Finished after Karate, Console / HTTP | Longest frame, Console / HTTP | Scroll during run: longest frame |
|---|---|---|---|---|---|
| A1 | 19.0 / 46.1 s | 104 ms / 53 ms | 4.7 / 88.8 s | 450 / 1,175 ms | 300 ms |
| A2 | 38.0 / 18.1 s | 181 ms / 104 ms | 37.8 / 7.8 s | 975 / 475 ms | 108 ms |
| A3 | 43.6 / 43.0 s | 200 ms / 120 ms | 55.2 / 47.6 s | 982 / 975 ms | 67 ms |

A1's Console run was on a quiet stretch (Karate at 19.0 s, the note's 18.2–19.0 s) and is the closest here to the note's conditions: "Finished" came 4.7 s after Karate, the longest frame was 450 ms, 30 fps. The renderer is saturated during Run all at this size, so when the machine slows by a factor the delay grows by far more than that factor (HTTP run of A1: 88.8 s). A fourth loaded round (Karate at 37.7 / 39.2 s, 45.7 / 47.8 s late) agreed. These rounds also show how little the other figures hold under load: the top folder opens in 104 to 200 ms, Down takes 53 to 120 ms.

## Not covered / still needs a person

- A rerun with the machine quiet, to see whether Down (61–65 ms against 50), the top folder (95–101 ms against 100) and the scroll during Run all (117 ms against 100) are within the load or real. The Run all delay of 7 s or more is not: it was 4.7 s on the quietest run and 2 s is the limit.
- How scrolling feels with a real wheel or trackpad during a 10,000-test Run all. Playwright's wheel sends steady 100 px notches.
- A decision on what to do with R6.3, R6.4 and R6.6: change the code, move the limits with a spec change, or leave the folder roll-up to #126. No limit was changed here.

## Harness changes

`perf.spec.ts`, `scroll.spec.ts` and `playwright.config.ts` were copied from `docs/research/2026-10-29-big-project-perf/`, which is untouched. The config is identical and `scroll.spec.ts` has only a comment added: the tree's scroller is still its parent, and the tree still carries `data-rows`. In `perf.spec.ts`:
- The HTTP list's row count is read from `data-rows` on `[data-testid=http-list]` (it was `http-calls`, the scroller).
- Rows are picked by their place from the top of the list, not by DOM order, because a virtual list draws its focus row first. The `index`th match is the `index`th from the top.
- A call is selected through a click on the row inside its option, which holds the click handler. Its `done` check reads `aria-selected`; `aria-pressed` is gone.
- A probe whose `done` already held before the click now throws instead of timing nothing.
- New probe: ten ArrowDown presses from the top folder (a keydown on the row, in the page), reported as `down`.

Files: `docs/checks/2026-10-30-virtual-lists-perf.md` and `docs/checks/2026-10-30-virtual-lists-perf/{perf.spec.ts,scroll.spec.ts,playwright.config.ts}`. The raw medians of every round are in the tables above; the JSON lines were kept outside the repository.
