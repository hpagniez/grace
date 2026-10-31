# Bench baseline: `develop` at e3cfe3b7, the first limits

Date: 2026-10-31
Commit measured: `e3cfe3b7` (merge of #146, `feature/virtual-tree`; `package.json` version 1.0.0-rc.6). The bench code is the one on `feature/perf-ci` at `511e4e5d`.
Machine: Windows 11 Pro 10.0.26300 (x64), AMD Ryzen 7 5800X (8 cores, 16 threads), 32 GB, NVIDIA GeForce RTX 4060 Ti, Node 24.15.0, npm 11.16.0, Electron 45.0.0, bundled Temurin 21.0.12.1 and Karate 2.1.4. Two screens: the main one, `DISPLAY2`, 2560 x 1440 at 120 Hz, and `DISPLAY1`, 2560 x 1440 at 59 Hz, to its right. This is the reference machine of [R5.2](../specs/2026-10-31-light-and-fast.md). See [Machine load](#machine-load).
Spec: [Light and fast](../specs/2026-10-31-light-and-fast.md), R3.2: the first limits are today's numbers.

**Result: the limits are written to `bench/limits.json`**, 59 measures, every one with a value in all 5 rounds. Each limit is the median of the 5 rounds plus a noise margin, as `makeLimits` works it out: the spread of its rounds, at least 5 % of the median and at least 5 ms or 1 unit, except a size, whose margin is its spread and at least 1 MB. One measure misses its aim today, at 10,000 tests ([Aims missed today](#aims-missed-today)).

## Why it was measured again

A first measurement earlier the same day opened the bench's windows at 40, 40, on the main screen, the 120 Hz one, in the way of the person working there. The owner asked that the windows of this work stay off the main screen, so the bench now opens every window centred on the first screen that is not the main one (`DISPLAY1` here; see `windowSpot` in `bench/system.mjs`), and the limits had to be taken there. `bench/limits.json` names that screen in its `machine` (`Windows 11 Pro, Ryzen 7 5800X, 32 GB, second screen 2560x1440 at 59 Hz`): a release benchmark must run with the same screen layout. This record replaces the first one; the first one's numbers and limits are in git history.

The 59 Hz screen did not halve the frame rate: the app drew at 118 fps during Run all, and its longest frames are multiples of 8.3 ms, the period of the 120 Hz main screen. Chromium seems to pace its frames by the main screen on this machine. So the Smooth limits are close to the first measurement's (`smooth.run.fps` at least 112, was 113).

The sizes also got narrower margins: a size never varies, so it no longer takes 5 % of the median (`size.unpacked` at most 539.7 MB, was 565.7).

## What was run

- `e3cfe3b7` exported with `git archive` into a scratch folder (no worktree), then `npm ci`, the worktree's `vendor/` copied in and `npm run vendor` (it verified the pins, which are the same as on `feature/perf-ci`), then `npm run make`. The exe is that folder's `out/Grace-win32-x64/Grace.exe`, the zip `Grace-win32-x64-1.0.0-rc.6.zip`. The scratch folder is deleted.
- From the `feature/perf-ci` worktree: `node --no-warnings bench/bench.mjs --release --b <exe> --b-zip <zip> --label-b "develop e3cfe3b7" --out <folder>`. Release mode, one build, so no comparison: 1 warm-up and 5 rounds, each round 3.1 to 3.4 minutes (the 6 runs took 201, 188, 188, 187, 188 and 195 s; 19 minutes in all, 18:43 to 19:02). A round is the fresh-profile start, the reused-profile start, idle (20 s to settle, 60 s shown, 60 s minimised), then Run all on the 2,000-test project and on the 10,000-test one, each with the Smooth probes idle and during the run.
- The bench logged `windows open at 3140, 246 on DISPLAY1, not the main screen (work area 2560 x 1392 at 2560, 0)`, and a window read mid-run sat at 3140, 246 to 4540, 1146, on `DISPLAY1`.
- The projects are the bench's own: 2,000 tests (5 features per module) and 10,000 tests (25).
- Then `node bench/bench.mjs --limits-from <folder>/bench.json --commit e3cfe3b7`, which wrote `bench/limits.json`.
- Each round waited for the system CPU load to read under 35 % before it started.
- `npm test` and `npm run e2e` were not run by this session while measuring; another session's e2e run overlapped the warm-up, see below.
- The raw result of the run, with every round, is beside this record: [`2026-10-31-bench-baseline.bench.json`](2026-10-31-bench-baseline.bench.json). Its build's `exe` reads `<scratch>/...`: the scratch folder's own path, which named the session, is cut from the copy committed here.
- Nothing was left running: no Grace or Java process of the bench after it exited (exit 0).

## The measures

Medians of the 5 rounds after the warm-up. "Rounds" is the lowest and highest of the 5. Limit is `median + noise` (`median - noise` where higher is better). Measures with an id ending `@10k` are those on the 10,000-test project. Aims are those of R3.3: click to paint under 100 ms, no frame over 50 ms. Sizes and DOM node counts do not vary from round to round; the page heap figures can (`page.heap.run` read 21 to 22 MB in the first measurement), though they did not in this one.

| Measure | Unit | Median | Rounds | Noise | Limit | Aim |
|---|---|---|---|---|---|---|
| `size.unpacked` | MB | 538.7 | 538.7 to 538.7 | 1 | at most 539.7 | – |
| `size.zip` | MB | 218.7 | 218.7 to 218.7 | 1 | at most 219.7 | – |
| `start.paint.fresh` | ms | 529 | 509 to 578 | 69 | at most 598 | – |
| `start.tree.fresh` | ms | 1782 | 1685 to 1866 | 181 | at most 1963 | – |
| `start.paint.reused` | ms | 553 | 514 to 573 | 59 | at most 612 | – |
| `start.tree.reused` | ms | 2050 | 1816 to 2155 | 339 | at most 2389 | – |
| `idle.memory` | MB | 155.3 | 150.8 to 158.3 | 8 | at most 163.3 | – |
| `idle.cpu.shown` | ms | 1562 | 172 to 1953 | 1781 | at most 3343 | – |
| `idle.cpu.minimised` | ms | 62 | 31 to 94 | 63 | at most 125 | – |
| `run.memory.app` | MB | 299.3 | 294 to 316.1 | 23 | at most 322.3 | – |
| `run.memory.java` | MB | 307.4 | 298.7 to 346.9 | 49 | at most 356.4 | – |
| `run.cpu.app` | ms | 4094 | 3969 to 5250 | 1281 | at most 5375 | – |
| `run.cpu.java` | ms | 17438 | 15703 to 20156 | 4453 | at most 21891 | – |
| `run.finishedLag` | ms | 132 | 124 to 151 | 27 | at most 159 | – |
| `smooth.select.idle.paint` | ms | 7 | 7 to 11 | 5 | at most 12 | 100 |
| `smooth.select.run.paint` | ms | 4 | 4 to 5 | 5 | at most 9 | 100 |
| `smooth.expand.idle.paint` | ms | 25 | 22 to 30 | 8 | at most 33 | 100 |
| `smooth.expand.run.paint` | ms | 21 | 20 to 27 | 7 | at most 28 | 100 |
| `smooth.http.idle.paint` | ms | 17 | 13 to 21 | 8 | at most 25 | 100 |
| `smooth.http.run.paint` | ms | 20 | 9 to 21 | 12 | at most 32 | 100 |
| `smooth.select.idle.frame` | ms | 8 | 8 to 8 | 5 | at most 13 | 50 |
| `smooth.select.run.frame` | ms | 4 | 3 to 4 | 5 | at most 9 | 50 |
| `smooth.expand.idle.frame` | ms | 16 | 10 to 25 | 15 | at most 31 | 50 |
| `smooth.expand.run.frame` | ms | 17 | 8 to 17 | 9 | at most 26 | 50 |
| `smooth.http.idle.frame` | ms | 8 | 8 to 8 | 5 | at most 13 | 50 |
| `smooth.http.run.frame` | ms | 17 | 8 to 17 | 9 | at most 26 | 50 |
| `smooth.scroll.idle.frame` | ms | 9 | 9 to 9 | 5 | at most 14 | 50 |
| `smooth.scroll.run.frame` | ms | 17 | 17 to 25 | 8 | at most 25 | 50 |
| `smooth.run.fps` | fps, higher is better | 118 | 118 to 119 | 6 | at least 112 | – |
| `smooth.run.frame` | ms | 25 | 17 to 25 | 8 | at most 33 | 50 |
| `page.nodes.open` | count | 2355 | 2355 to 2355 | 118 | at most 2473 | – |
| `page.nodes.run` | count | 7289 | 7289 to 7289 | 365 | at most 7654 | – |
| `page.heap.open` | MB | 17 | 17 to 17 | 1 | at most 18 | – |
| `page.heap.run` | MB | 22 | 22 to 22 | 2 | at most 24 | – |
| `run.memory.app@10k` | MB | 577.5 | 574.7 to 584.8 | 29 | at most 606.5 | – |
| `run.memory.java@10k` | MB | 495.2 | 475.5 to 509 | 34 | at most 529.2 | – |
| `run.cpu.app@10k` | ms | 14953 | 14438 to 17156 | 2718 | at most 17671 | – |
| `run.cpu.java@10k` | ms | 43516 | 42281 to 47672 | 5391 | at most 48907 | – |
| `run.finishedLag@10k` | ms | 338 | 294 to 412 | 118 | at most 456 | – |
| `smooth.select.idle.paint@10k` | ms | 9 | 8 to 10 | 5 | at most 14 | 100 |
| `smooth.select.run.paint@10k` | ms | 5 | 4 to 9 | 5 | at most 10 | 100 |
| `smooth.expand.idle.paint@10k` | ms | 48 | 47 to 51 | 5 | at most 53 | 100 |
| `smooth.expand.run.paint@10k` | ms | 57 | 51 to 74 | 23 | at most 80 | 100 |
| `smooth.http.idle.paint@10k` | ms | 21 | 11 to 22 | 11 | at most 32 | 100 |
| `smooth.http.run.paint@10k` | ms | 24 | 10 to 25 | 15 | at most 39 | 100 |
| `smooth.select.idle.frame@10k` | ms | 8 | 8 to 9 | 5 | at most 13 | 50 |
| `smooth.select.run.frame@10k` | ms | 5 | 4 to 8 | 5 | at most 10 | 50 |
| `smooth.expand.idle.frame@10k` | ms | 42 | 42 to 45 | 5 | at most 47 | 50 |
| `smooth.expand.run.frame@10k` | ms | 50 | 42 to 67 | 25 | at most 75 | 50 |
| `smooth.http.idle.frame@10k` | ms | 17 | 8 to 17 | 9 | at most 26 | 50 |
| `smooth.http.run.frame@10k` | ms | 17 | 8 to 19 | 11 | at most 28 | 50 |
| `smooth.scroll.idle.frame@10k` | ms | 9 | 9 to 9 | 5 | at most 14 | 50 |
| `smooth.scroll.run.frame@10k` | ms | 25 | 25 to 33 | 8 | at most 33 | 50 |
| `smooth.run.fps@10k` | fps, higher is better | 107 | 98 to 111 | 13 | at least 94 | – |
| `smooth.run.frame@10k` | ms | 81 | 75 to 100 | 25 | at most 106 | 50 |
| `page.nodes.open@10k` | count | 2355 | 2355 to 2355 | 118 | at most 2473 | – |
| `page.nodes.run@10k` | count | 22533 | 22533 to 22533 | 1127 | at most 23660 | – |
| `page.heap.open@10k` | MB | 62 | 62 to 62 | 4 | at most 66 | – |
| `page.heap.run@10k` | MB | 83 | 83 to 83 | 5 | at most 88 | – |

Nothing is `n/a`: no probe came back empty in any round.

A limit above its aim is expected (R3.3: a limit is never capped at its aim). Two limits are above their aims, both at 10,000 tests:

- `smooth.run.frame@10k`: at most 106 ms, aim 50 ms.
- `smooth.expand.run.frame@10k`: at most 75 ms, aim 50 ms.

## Aims missed today

The median is over the aim for one measure:

| Measure | Median | Rounds | Aim | Limit |
|---|---|---|---|---|
| `smooth.run.frame@10k`: longest frame during Run all | 81 ms | 75, 92, 75, 81 and 100 ms | 50 ms | 106 ms |

On the aim, not over it: `smooth.expand.run.frame@10k`, the longest frame opening the top folder during Run all, a median of exactly 50 ms; two of its 5 rounds read 67 and 58 ms.

Every other aim is met by the median and by every round: all click-to-paint measures are under 100 ms (the highest, `smooth.expand.run.paint@10k`, 57 ms, 74 ms in its worst round) and, at 2,000 tests, every frame is under 50 ms (the highest round, 25 ms).

The bench reads lower than the old harness's figures that R3.3 quotes for the longest frame of a Run all (about 100 ms at 2,000 tests, 333 to 350 ms at 10,000): 25 and 81 ms here. The two harnesses are different probes (here the longest gap between frames from the start of Run all to its end, with the Smooth probes running in it), so they are not compared; the limits follow the bench.

## Machine load

The load the bench reads before each round (system CPU, over a short window), in order: warm-up 13.3 %, then 25.2, 11.5, 11.8, 8.1 and 10.0 % for the 5 rounds. Each was under 35 % within 1 s of the wait starting. The bench reads the load only before a round, not during it.

Another session ran the Playwright end-to-end tests of the main checkout from about 18:12, starting runs again at 18:27 and 18:36; I waited for a gap, then started the bench at 18:43, when the load read 17 %. Its last Electron process was gone by about 18:45, during the warm-up, and none was seen from then until 18:47, when that check (every 10 s) stopped; the loads read before the later rounds, 8 to 12 %, show no new run. The warm-up, which no limit uses, may include that run's load; the 5 rounds after it most likely ran without it. Other sessions' idle processes stayed on the machine throughout, among them a Grace exe left open since 14:26 by another session's update test, using no CPU.

The wide margins of the CPU measures are their own spread, not a busy machine: `idle.cpu.shown` read 172 to 1953 ms over the 5 rounds, `run.cpu.java` 15.7 to 20.2 s.

## Not covered / still needs a person

- The margins come from 5 rounds. If a later quiet-machine measurement gives much narrower spreads, lowering a limit is an ordinary commit (R3.4).
- Release-mode figures only: PR mode on CI runs the 2,000-test subset on a different machine with one screen, and judges only the measures that do not depend on the machine (R3.5).
- The `machine: true` limits (times, CPU, memory) hold for this machine, this Windows build and this screen layout only.
- Why the frames follow the 120 Hz main screen and not the 59 Hz one the window is on is not looked into; if the main screen changes rate, the Smooth figures may change with it.
