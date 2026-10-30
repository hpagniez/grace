# App check: virtual list speed at 10,000 tests (#125, #91)

Date: 2026-10-30
Branch: `feature/virtual-tree`. First measured at 36484699: R6 was not met. Measured again at 9576d373, after three renderer fixes ([What was fixed](#what-was-fixed-and-why)). The only change since is this record and its harness.
Machine: Windows 11 Pro 10.0.26300 (x64), AMD Ryzen 7 5800X (8 cores, 16 threads), 32 GB, NVIDIA GeForce RTX 4060 Ti, Node 24.15.0, npm 11.16.0, Electron 45.0.0, bundled Temurin 21.0.12.1 and Karate 2.1.4. See [Machine load](#machine-load).
Spec: [Virtual lists and keyboard navigation](../specs/2026-10-30-virtual-lists.md), R6. Method: sections 1 and 8 of the [big-project performance note](../research/2026-10-29-big-project-perf/README.md).

**Result: R6 is met at 10,000 tests**, every figure under its limit. The closest are the longest frame during Run all with the HTTP tab (350 ms against 500 ms) and opening the top folder (68 ms against 100 ms). "Finished" now comes 0.55 s (Console tab) and 0.65 s (HTTP tab) after Karate's last test, against 2 s; it came 7.3 and 7.6 s after before the fixes. R6.7 at 2,000 tests passes: every figure is better than on `develop` measured beside it.

The progress bar's green part that stayed at zero during a big run is explained and fixed: [The progress bar](#the-progress-bar).

## What was run

- `npm run package` at 9576d373, then the harness in [`2026-10-30-virtual-lists-perf/`](2026-10-30-virtual-lists-perf/), unchanged since the first measurement but for the new, opt-in `bar.spec.ts` ([Harness changes](#harness-changes)). Window 1400 × 900, as in the note.
- Projects from the note's `generate.mjs`: 2,000 tests (5 features per module: 600 HTTP calls, 2,449 tree rows) and 10,000 tests (25: 3,000 calls, 12,049 rows).
- Per round, from the repo root: `PERF_PROJECT=<project> PERF_OUT=<file>.jsonl SCROLL_OUT=<file>.jsonl npx playwright test --config docs/checks/2026-10-30-virtual-lists-perf/playwright.config.ts perf.spec.ts scroll.spec.ts`. One round is `perf.spec.ts` (open, folders, selection, Down, two Run alls, the HTTP tab) and `scroll.spec.ts` (60 wheel notches idle, up, and during Run all).
- 10,000 tests: 3 rounds in a row; the R6 figure is their median.
- 2,000 tests (R6.7): 3 rounds of this build and 3 of `develop` before the change, in turns (this, develop, develop, this, this, develop). `develop` is `b9a8a5e2` (the merge of the spec), built with `npm run package` in a temporary worktree and measured with the note's own, unchanged harness; the worktree is deleted.
- Each round waited for the system CPU load to read under 35 %.
- `npm test` and `npm run e2e` were not run while measuring.
- "Finished after Karate" is the time from the last depth-0 `SCENARIO_EXIT` in Karate's event file to "Finished" shown, on one clock (the note's method).

## R6 at 10,000 tests

Medians of 3 rounds. The prototype column is the note's B–E + V build, as bracketed in the spec.

| R6 | Measure | Limit | Prototype | 9576d373 | Result |
|---|---|---|---|---|---|
| 1 | Open: longest renderer task | under 500 ms | 290 ms | 282 ms | pass |
| 1 | DOM nodes after open | under 10,000 | 3.7k | 3,252 | pass |
| 2 | Close the top folder (12,049 rows) | under 100 ms | 49–52 ms | 28 ms | pass |
| 2 | Open the top folder | under 100 ms | 49–52 ms | 68 ms | pass |
| 3 | Select a test with the mouse (median of 20 per round) | under 50 ms | 11 ms | 6 ms | pass |
| 3 | Move with Down (median of 10 per round) | under 50 ms | not measured | 9 ms | pass |
| 4 | Console tab: "Finished" after Karate's last test | within 2 s | 0.2–1.2 s | 0.55 s | pass |
| 4 | HTTP tab: "Finished" after Karate's last test | within 2 s | 0.2–1.2 s | 0.65 s | pass |
| 4 | Console tab: longest frame during Run all | no more than 500 ms | 342–383 ms | 333 ms | pass |
| 4 | HTTP tab: longest frame during Run all | no more than 500 ms | 342–383 ms | 350 ms | pass |
| 5 | Open the HTTP tab (3,000 calls) | under 100 ms | 23 ms | 27 ms | pass |
| 6 | Scroll the tree during Run all: longest frame | under 100 ms | 25–33 ms | 50 ms | pass |

All rounds, in order:

| Round | Karate's last test, Console / HTTP | Close / open top folder | Select / Down | Finished after Karate, Console / HTTP | Longest frame, Console / HTTP | Scroll during run: longest frame | Open: longest task |
|---|---|---|---|---|---|---|---|
| 1 | 16.7 / 17.9 s | 28 / 54 ms | 5 / 9 ms | 0.55 / 0.33 s | 317 / 350 ms | 50 ms | 269 ms |
| 2 | 19.3 / 19.2 s | 28 / 69 ms | 8 / 15 ms | 0.79 / 0.65 s | 333 / 350 ms | 42 ms | 282 ms |
| 3 | 20.0 / 21.1 s | 44 / 68 ms | 6 / 9 ms | 0.53 / 0.70 s | 333 / 383 ms | 58 ms | 339 ms |

Other figures (medians, no limit):
- Rows drawn after open: 29. DOM nodes after open 3,252 and at the end of both Run alls 3,201. JS heap 91 MB after open, 108 MB at the end. Renderer busy during open: 693 ms.
- Run all: 82 fps (Console) and 76 fps (HTTP); the bar full 0.12 and 0.09 s after Karate's last test. Script time during round 1's runs: 5.8 s of a 17.2 s run (Console) and 6.4 s of 18.3 s (HTTP), against 20.5 s of 28.5 s before.
- In the profile (see below), the longest burst of work in a Run all is at its start, when main sends the tree of the discovery it runs before every run: a new tree object, so every drawn row, the details pane and the filter chips work out again.
- Scroll during Run all: 1, 0 and 1 frames over 50 ms in the three rounds (before: 91 of about 480).
- Idle scroll of 60 notches: longest frame 31 ms down and 24 ms up (9, 39 and 31 ms; 9, 36 and 24 ms), main-thread paint 116 ms, hit tests 12 ms. Before: 9 and 21 ms. Not a limit at this size; at 2,000 tests see R6.7.
- HTTP tab: group chips 29–35 ms, select a call 8 ms, narrow to one feature 12 ms, widen back to the top folder 64 ms (before: 33–40, 15, 37 and 170 ms).

### Before the fix

The first measurement, at 36484699 (rounds 1 to 3, medians). The machine was loaded then: a live video in Edge took up to 9 of the 16 threads, and Karate's last test came at 21.2 to 24.1 s against 16.7 to 21.1 s now. Even its quietest Run all (Karate at 19.0 s) ended 4.7 s late.

| R6 | Measure | Limit | 36484699 | Result then |
|---|---|---|---|---|
| 1 | Open: longest renderer task / DOM nodes | under 500 ms / 10,000 | 368 ms / 3,252 | pass |
| 2 | Close / open the top folder | under 100 ms | 65 / 95 ms | pass / at the limit |
| 3 | Select with the mouse / move with Down | under 50 ms | 30 / 65 ms | pass / **fail** |
| 4 | "Finished" after Karate, Console / HTTP | within 2 s | 7.3 / 7.6 s | **fail** |
| 4 | Longest frame during Run all, Console / HTTP | no more than 500 ms | 500 / 500 ms | **fail** |
| 5 | Open the HTTP tab | under 100 ms | 28 ms | pass |
| 6 | Scroll the tree during Run all: longest frame | under 100 ms | 117 ms | **fail** |

## What was fixed, and why

Measured first, on an unminified build of `dcd5357c` (`build.minify: false` for the profile's function names, reverted after; same code as 36484699 plus the first record): CPU profiles of a 10,000-test Run all with the harness's `PERF_PROFILE`, Console then HTTP tab, the top folder selected as the harness runs it, and the call counts of one Console run from V8's precise coverage. These are figures of that build, not R6 figures.

Before, Console run: Karate's last test at 21.7 s, "Finished" at 34.9 s. Until Karate's last test the renderer was busy 16.4 s of 21.7 s; after it, 13.4 s more with no idle time (15.9 s in the HTTP run), working off queued updates.

| Cost, until Karate's last test | Inclusive time | Calls in the run |
|---|---|---|
| `deriveStatus`, every caller | 10.4 s | 15.0 M nodes visited: about 8 walks of the 12,050-node top folder per update, about 155 updates |
| … the details pane's status badge for the selected top folder | 7.7 s | 6 walks per update: each read of the badge's `status` prop worked it out again |
| … a tree row's `attrs`, for its accessible name | 2.6 s, 1.3 s of it the flaky chip | `attrs` 312, the flaky chip 370: the status again, and a walk of the row's tests |
| … a tree row's status memo | 1.4 s | one walk per drawn folder per update |
| The details pane's counts per status | 1.3 s | 155 |
| The HTTP tab's calls in scope (`callsOf`) | 0.9 s | 152 |

Self time went mostly to the store's proxy reads (5.2 s, 104 M calls) and signal reads (1.8 s, 115 M). A row's `attrs` also sets `aria-selected` and `tabindex`, so it ran again for every drawn row at each selection and each move of the focus, walking each subtree and its tests: most of Down's 65 ms and of the top folder's 95 ms.

The fixes, in the renderer only:
1. **Details pane** (f8187603): the group's status is a memo. One walk per update instead of six.
2. **Tree rows** (a6a5d7db): `VirtualList` takes a `shared` part, made once per drawn row in the row's scope and handed to both `attrs` and the row's content. The tree puts the row's status and flaky chip there as two memos: the row's mark and its accessible name read the same walk, and a selection or a focus move walks nothing. Names (R5.2), levels and positions are as before.
3. **Progress bar** (9576d373): [The progress bar](#the-progress-bar).

After, same build mode and flow: Karate's last test at 22.8 s, "Finished" 0.78 s later (HTTP run: 0.76 s), longest frame 425 and 433 ms in that slower build. Until Karate's last test: busy 14.4 s, idle 8.2 s; after it, 0.8 s. `deriveStatus` visits 4.0 M nodes (−73 %), the proxy reads fall to 41 M, `attrs` runs 94 times and the flaky chip 58. The top costs left: the tree rows' status memos (2.3 s) and the details pane's status (2.4 s) and counts (2.4 s), each walking the top folder once per update, and `callsOf` (1.9 s). Rolling folder statuses up as each test changes ([#126](https://github.com/hpagniez/grace/issues/126)) would remove the first two; it was not needed for R6.

## The progress bar

Reported during the first measurement: in a 10,000-test Run all the bar showed only its red part, 1.8 % wide at the far left, while the words read "8970 / 10000, 8790 passed, 180 failed". The green part stayed at zero width while the red part grew.

Observed with a new probe, [`bar.spec.ts`](2026-10-30-virtual-lists-perf/bar.spec.ts): the harness's set-up (top folder selected), one Run all with the Console tab and one with the HTTP tab, and every 500 ms the drawn width of the green and red segments (`getBoundingClientRect`) against `progress-passed` and `progress-failed`, with each segment's CSS transitions and whether they have a start time yet. CDP's Animation domain lists every animation that starts in the page. Unminified build of `dcd5357c`, then of 9576d373. The track is 1,358 px wide.

| Run | Added to every update | Console tab | HTTP tab |
|---|---|---|---|
| As the harness runs (two runs) | nothing | green grows: 1,054 px for 78.6 % at 25.0 s | green grows: 416 px for 32.8 % at 21.0 s |
| `BAR_FORCE=read` | a forced style read | green grows | green grows |
| `BAR_FORCE=spin` | a new spinning element | **green 0 px for 50.8 % (5,076 passed), red 8.3 px for 1.0 %, at 21.0 s** | **green 0 px for 37.2 %, red 7.9 px for 0.8 %, at 21.7 s** |
| `BAR_FORCE=spin`, after the fix | a new spinning element | green 1,208 px for 90.4 % at 21.5 s | green 1,178 px for 87.9 % (8,789 passed) at 21.5 s |

In the `spin` runs before the fix, the green transition is running with no start time at every sample until Karate is done.

**Cause.** A spinning status mark (`animate-spin`, a `transform` animation) runs on the compositor, and one starts each time a drawn row's status turns to running. Chromium starts a main-thread transition, such as a segment's `width`, that begins in the same frame as a compositor animation only once the compositor has started that animation. A transition replaced while it waits starts again from where it stood. Green gets a new width at nearly every update of a big run, so when spinners keep starting, each of its transitions is replaced before it starts, and it never leaves zero. Red changes every few updates, gets its start times in between, and grows. The probe shows this in the real app, with one added spinner per update. The harness's own runs do not show it: with the top of the tree in view, those rows finish in the first second, and only 8 spinning marks start in a whole run (the outcome's at 0.4 s, 7 tree rows at 6.9 s), so the bar grew in every run measured. The view the report came from was not recorded; one in which rows keep turning to running would do it. A forced style read in every update, tried first, does not.

**Fix** (9576d373). The segments take new widths only once none of their transitions is waiting for its start time; until then they try again at each frame, with the newest counts. The words, the bar's `aria-value*` and its stripe do not wait. On a small project a transition has long started when the next result comes, so each result moves the bar at once with the same 300 ms ease-out (R2.1 of the [loading spec](../specs/2026-10-28-loading-always-visible.md)); with reduced motion there is no transition and nothing waits. Not taken: dropping the transition, which breaks R2.1; taking new widths at most every 300 ms, which a frame of over 300 ms in a big run defeats.

## At 2,000 tests (R6.7)

Medians of 3 rounds each, in turns. The note's B–E column is the reference named in R6.7; its B–E + V column is the prototype.

| Measure | `develop` before | 9576d373 | Note B–E | Note B–E + V |
|---|---|---|---|---|
| Open: longest task / renderer busy | 504 / 895 ms | 103 / 343 ms | 473 / 882 ms | 91 / 311 ms |
| DOM nodes / listeners / JS heap after open | 123.7k / 14,856 / 87 MB | 2.5k / 338 / 17 MB | 123.4k / 14,856 / 87 MB | 2.5k / 337 / 16 MB |
| Open the top folder (2,449 rows) | 535 ms | 27 ms | 450 ms | 23 ms |
| Select a test (median of 20) | 54 ms | 8 ms | 35 ms | 11 ms |
| Console run: click to "Finished" / Karate's last test | 8.48 / 7.96 s | 8.14 / 7.80 s | 7.3 / 7.0 s | 6.7 / 6.4 s |
| … "Finished" after Karate / bar full after Karate | 0.50 / 0.08 s | 0.33 / 0.07 s | 0.3 / 0.11 s | 0.3 / 0.08 s |
| … frame rate / longest frame | 54 fps / 783 ms | 113 fps / 108 ms | 76 fps / 608 ms | 114 fps / 67 ms |
| HTTP run: click to "Finished" / Karate's last test | 8.54 / 7.97 s | 8.08 / 7.79 s | 7.5 s / n/a | 6.6 s / n/a |
| … "Finished" after Karate / bar full after Karate | 0.54 / 0.14 s | 0.29 / 0.10 s | n/a / 0.11 s | n/a / 0.09 s |
| … frame rate / longest frame | 51 fps / 783 ms | 114 fps / 92 ms | 70 fps / 717 ms | 116 fps / 58 ms |
| Open the HTTP tab (600 calls) | 108 ms | 18 ms | 118 ms | 15 ms |
| Idle scroll down: main-thread paint / hit tests | 211 / 170 ms | 140 / 14 ms | 170 / 133 ms | 104 / 10 ms |
| Idle scroll: longest frame (down / up) | 25 / 25 ms | 20 / 9 ms | 25 ms | 17 ms |
| Scroll during Run all: longest frame | 125 ms | 25 ms | 100 ms | 17 ms |

Result: pass. Every row is better than `develop` measured beside it; the HTTP run's "Finished" after Karate, level in the first measurement (0.41 against 0.42 s), is now 0.29 against 0.54 s. Against the note's B–E column every row is better too, except the absolute run times: Karate's own last test came at 7.0 to 8.0 s here against 7.0 s, and the time left after it is the same or shorter. One round of this build counted 3,510 DOM nodes after open against 2,498 in the other two, with the same 29 rows drawn: nodes not yet collected.

## Machine load

Second measurement: quieter. The system CPU load read 15 to 27 % before each round and 20 to 40 % after. Edge's GPU process still ran, at 3.8 CPU-seconds in 5 s against 46 in the first measurement. Karate's own last test came at 16.7 to 21.1 s at 10,000 tests, against the note's 18.2–19.0 s.

First measurement: a live video in Edge took up to 9 of the 16 threads (46 CPU-seconds in 5 s), and the load read 26 to 90 % with no test running. Its figures are the "before the fix" table above.

## Not covered / still needs a person

- How scrolling feels with a real wheel or trackpad during a 10,000-test Run all. Playwright's wheel sends steady 100 px notches.
- The progress bar in the view the report came from: the probe adds the spinners; a person can watch a big run with the running rows in view.

## Harness changes

`perf.spec.ts`, `scroll.spec.ts` and `playwright.config.ts` were copied from `docs/research/2026-10-29-big-project-perf/`, which is untouched. The config is identical and `scroll.spec.ts` has only a comment added: the tree's scroller is still its parent, and the tree still carries `data-rows`. In `perf.spec.ts`:
- The HTTP list's row count is read from `data-rows` on `[data-testid=http-list]` (it was `http-calls`, the scroller).
- Rows are picked by their place from the top of the list, not by DOM order, because a virtual list draws its focus row first. The `index`th match is the `index`th from the top.
- A call is selected through a click on the row inside its option, which holds the click handler. Its `done` check reads `aria-selected`; `aria-pressed` is gone.
- A probe whose `done` already held before the click now throws instead of timing nothing.
- New probe: ten ArrowDown presses from the top folder (a keydown on the row, in the page), reported as `down`.

New: `bar.spec.ts`, the progress bar probe above, skipped unless `BAR_OUT` is set. `BAR_FORCE=read` or `spin` adds work to every update; `BAR_SHOT=<prefix>` saves a picture of the bar.

Files: `docs/checks/2026-10-30-virtual-lists-perf.md` and `docs/checks/2026-10-30-virtual-lists-perf/{perf.spec.ts,scroll.spec.ts,bar.spec.ts,playwright.config.ts}`. The JSON lines and profiles were kept outside the repository.
