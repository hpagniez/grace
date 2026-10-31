# Bench baseline: `develop` at e3cfe3b7, the first limits

Date: 2026-10-31
Commit measured: `e3cfe3b7` (merge of #146, `feature/virtual-tree`; `package.json` version 1.0.0-rc.6). The bench code is the one on `feature/bench` at `c280efeb`; the only later change is the `--limits-from` mode that wrote the limits.
Machine: Windows 11 Pro 10.0.26300 (x64), AMD Ryzen 7 5800X (8 cores, 16 threads), 32 GB, NVIDIA GeForce RTX 4060 Ti, Node 24.15.0, npm 11.16.0, Electron 45.0.0, bundled Temurin 21.0.12.1 and Karate 2.1.4. This is the reference machine of [R5.2](../specs/2026-10-31-light-and-fast.md). See [Machine load](#machine-load).
Spec: [Light and fast](../specs/2026-10-31-light-and-fast.md), R3.2: the first limits are today's numbers.

**Result: the limits are written to `bench/limits.json`**, 59 measures, every one with a value in all 5 rounds. Each limit is the median of the 5 rounds plus a noise margin, the spread of its rounds (at least 5 % of the median, at least 5 ms or 1 unit), as `makeLimits` works it out. Two measures miss their aims today, both at 10,000 tests ([Aims missed today](#aims-missed-today)).

## What was run

- `e3cfe3b7` exported with `git archive` into a scratch folder (no worktree), then `npm ci`, the worktree's `vendor/` copied in and `npm run vendor` (it verified the pins, which are the same as on `feature/bench`), then `npm run make`: 30 s. The exe is that folder's `out/Grace-win32-x64/Grace.exe`, the zip `Grace-win32-x64-1.0.0-rc.6.zip`. The scratch folder is deleted.
- From the `feature/bench` worktree: `node --no-warnings bench/bench.mjs --release --b <exe> --b-zip <zip> --label-b "develop e3cfe3b7" --out <folder>`. Release mode, one build, so no comparison: 1 warm-up and 5 rounds, each round 3.2 to 3.4 minutes (the 6 runs took 202, 195, 195, 192, 197 and 197 s; 20 minutes in all, 15:46 to 16:06). A round is the fresh-profile start, the reused-profile start, idle (20 s to settle, 60 s shown, 60 s minimised), then Run all on the 2,000-test project and on the 10,000-test one, each with the Smooth probes idle and during the run.
- The projects are the bench's own: 2,000 tests (5 features per module) and 10,000 tests (25).
- Then `node bench/bench.mjs --limits-from <folder>/bench.json --commit e3cfe3b7`, which wrote `bench/limits.json` with `machine` set to `Windows 11 Pro, Ryzen 7 5800X, 32 GB`.
- Each round waited for the system CPU load to read under 35 % before it started.
- `npm test` and `npm run e2e` were not run by this session while measuring; other sessions used the machine, see below.
- The raw result of the run, with every round, is beside this record: [`2026-10-31-bench-baseline.bench.json`](2026-10-31-bench-baseline.bench.json).

## The measures

Medians of the 5 rounds after the warm-up. "Rounds" is the lowest and highest of the 5. Limit is `median + noise` (`median - noise` where higher is better). Measures with an id ending `@10k` are those on the 10,000-test project. Aims are those of R3.3: click to paint under 100 ms, no frame over 50 ms. Sizes and page figures do not vary from round to round.

| Measure | Unit | Median | Rounds | Noise | Limit | Aim |
|---|---|---|---|---|---|---|
| `size.unpacked` | MB | 538.7 | 538.7 to 538.7 | 27 | at most 565.7 | – |
| `size.zip` | MB | 218.7 | 218.7 to 218.7 | 11 | at most 229.7 | – |
| `start.paint.fresh` | ms | 548 | 536 to 634 | 98 | at most 646 | – |
| `start.tree.fresh` | ms | 2086 | 1929 to 2094 | 165 | at most 2251 | – |
| `start.paint.reused` | ms | 618 | 498 to 628 | 130 | at most 748 | – |
| `start.tree.reused` | ms | 2223 | 2084 to 2439 | 355 | at most 2578 | – |
| `idle.memory` | MB | 154.7 | 151.3 to 157.4 | 8 | at most 162.7 | – |
| `idle.cpu.shown` | ms | 2829 | 2000 to 3890 | 1890 | at most 4719 | – |
| `idle.cpu.minimised` | ms | 312 | 250 to 422 | 172 | at most 484 | – |
| `run.memory.app` | MB | 302.4 | 287.1 to 311.6 | 25 | at most 327.4 | – |
| `run.memory.java` | MB | 315.8 | 302.5 to 384.3 | 82 | at most 397.8 | – |
| `run.cpu.app` | ms | 5563 | 4531 to 6297 | 1766 | at most 7329 | – |
| `run.cpu.java` | ms | 20875 | 18406 to 22625 | 4219 | at most 25094 | – |
| `run.finishedLag` | ms | 178 | 154 to 185 | 31 | at most 209 | – |
| `smooth.select.idle.paint` | ms | 8 | 7 to 11 | 5 | at most 13 | 100 |
| `smooth.select.run.paint` | ms | 6 | 4 to 8 | 5 | at most 11 | 100 |
| `smooth.expand.idle.paint` | ms | 27 | 22 to 31 | 9 | at most 36 | 100 |
| `smooth.expand.run.paint` | ms | 27 | 22 to 32 | 10 | at most 37 | 100 |
| `smooth.http.idle.paint` | ms | 14 | 8 to 23 | 15 | at most 29 | 100 |
| `smooth.http.run.paint` | ms | 15 | 13 to 23 | 10 | at most 25 | 100 |
| `smooth.select.idle.frame` | ms | 8 | 7 to 22 | 15 | at most 23 | 50 |
| `smooth.select.run.frame` | ms | 7 | 3 to 8 | 5 | at most 12 | 50 |
| `smooth.expand.idle.frame` | ms | 24 | 17 to 25 | 8 | at most 32 | 50 |
| `smooth.expand.run.frame` | ms | 17 | 17 to 25 | 8 | at most 25 | 50 |
| `smooth.http.idle.frame` | ms | 17 | 8 to 17 | 9 | at most 26 | 50 |
| `smooth.http.run.frame` | ms | 9 | 8 to 17 | 9 | at most 18 | 50 |
| `smooth.scroll.idle.frame` | ms | 9 | 9 to 9 | 5 | at most 14 | 50 |
| `smooth.scroll.run.frame` | ms | 17 | 17 to 25 | 8 | at most 25 | 50 |
| `smooth.run.fps` | fps, higher is better | 119 | 118 to 119 | 6 | at least 113 | – |
| `smooth.run.frame` | ms | 25 | 17 to 33 | 16 | at most 41 | 50 |
| `page.nodes.open` | count | 2355 | 2355 to 2355 | 118 | at most 2473 | – |
| `page.nodes.run` | count | 7289 | 7289 to 7289 | 365 | at most 7654 | – |
| `page.heap.open` | MB | 17 | 17 to 17 | 1 | at most 18 | – |
| `page.heap.run` | MB | 22 | 21 to 22 | 2 | at most 24 | – |
| `run.memory.app@10k` | MB | 579 | 577.7 to 597.4 | 29 | at most 608 | – |
| `run.memory.java@10k` | MB | 491.4 | 478.1 to 493.2 | 25 | at most 516.4 | – |
| `run.cpu.app@10k` | ms | 18875 | 18219 to 22500 | 4281 | at most 23156 | – |
| `run.cpu.java@10k` | ms | 51109 | 49156 to 52578 | 3422 | at most 54531 | – |
| `run.finishedLag@10k` | ms | 398 | 381 to 478 | 97 | at most 495 | – |
| `smooth.select.idle.paint@10k` | ms | 10 | 9 to 12 | 5 | at most 15 | 100 |
| `smooth.select.run.paint@10k` | ms | 8 | 5 to 9 | 5 | at most 13 | 100 |
| `smooth.expand.idle.paint@10k` | ms | 56 | 52 to 71 | 19 | at most 75 | 100 |
| `smooth.expand.run.paint@10k` | ms | 83 | 62 to 94 | 32 | at most 115 | 100 |
| `smooth.http.idle.paint@10k` | ms | 26 | 8 to 28 | 20 | at most 46 | 100 |
| `smooth.http.run.paint@10k` | ms | 24 | 14 to 59 | 45 | at most 69 | 100 |
| `smooth.select.idle.frame@10k` | ms | 8 | 8 to 9 | 5 | at most 13 | 50 |
| `smooth.select.run.frame@10k` | ms | 7 | 6 to 8 | 5 | at most 12 | 50 |
| `smooth.expand.idle.frame@10k` | ms | 50 | 42 to 67 | 25 | at most 75 | 50 |
| `smooth.expand.run.frame@10k` | ms | 75 | 50 to 92 | 42 | at most 117 | 50 |
| `smooth.http.idle.frame@10k` | ms | 17 | 8 to 17 | 9 | at most 26 | 50 |
| `smooth.http.run.frame@10k` | ms | 25 | 8 to 33 | 25 | at most 50 | 50 |
| `smooth.scroll.idle.frame@10k` | ms | 9 | 9 to 9 | 5 | at most 14 | 50 |
| `smooth.scroll.run.frame@10k` | ms | 33 | 25 to 42 | 17 | at most 50 | 50 |
| `smooth.run.fps@10k` | fps, higher is better | 98 | 97 to 100 | 5 | at least 93 | – |
| `smooth.run.frame@10k` | ms | 108 | 100 to 125 | 25 | at most 133 | 50 |
| `page.nodes.open@10k` | count | 2355 | 2355 to 2355 | 118 | at most 2473 | – |
| `page.nodes.run@10k` | count | 22533 | 22533 to 22533 | 1127 | at most 23660 | – |
| `page.heap.open@10k` | MB | 62 | 62 to 62 | 4 | at most 66 | – |
| `page.heap.run@10k` | MB | 83 | 83 to 83 | 5 | at most 88 | – |

Nothing is `n/a`: no probe came back empty in any round. A limit above its aim is expected (R3.3: a limit is never capped at its aim); `smooth.expand.run.paint@10k` has one, 115 ms against 100 ms.

## Aims missed today

The median is over the aim for two measures, both at 10,000 tests:

| Measure | Median | Rounds | Aim | Limit |
|---|---|---|---|---|
| `smooth.run.frame@10k`: longest frame during Run all | 108 ms | 100 to 125 ms | 50 ms | 133 ms |
| `smooth.expand.run.frame@10k`: longest frame opening the top folder during Run all | 75 ms | 50 to 92 ms | 50 ms | 117 ms |

On the aim, not over it: `smooth.expand.idle.frame@10k`, the longest frame opening the top folder when idle, a median of exactly 50 ms; two of its 5 rounds read 58 and 67 ms.

Every other aim is met by the median and by every round: all click-to-paint measures are under 100 ms (the highest, `smooth.expand.run.paint@10k`, 83 ms, 94 ms in its worst round) and, at 2,000 tests, every frame is under 50 ms (the highest round, `smooth.run.frame`, 33 ms).

The bench reads lower than the old harness's figures that R3.3 quotes for the longest frame of a Run all (about 100 ms at 2,000 tests, 333 to 350 ms at 10,000): 25 and 108 ms here. The two harnesses are different probes (here the longest gap between frames from the start of Run all to its end, with the Smooth probes running in it), so they are not compared; the limits follow the bench.

## Machine load

The load the bench reads before each round (system CPU, over a short window), in order: warm-up 21.2 %, then 21.0, 16.3, 9.9, 22.8 and 29.2 % for the 5 rounds. Each was under 35 % within 4 s of the wait starting. The bench reads the load only before a round, not during it.

Other sessions used the machine around the bench: I saw a Playwright end-to-end run of the main checkout (Electron and Karate processes) start at 15:40 and processes of other sessions at other times. Instantaneous readings were 11 to 71 % while I waited for the machine to settle before the run (15:40 to 15:46), and 65 % during the last round (16:05, about 160 s into it). The rounds are therefore not all measured on a quiet machine, and the spread of a round's CPU and time measures includes that. It shows in the widest margins: `idle.cpu.shown` (2000 to 3890 ms, noise 1890 ms), `run.cpu.app` and `run.cpu.java`, and the 10,000-test paint and frame measures. The medians are less sensitive to it than the margins.

A first attempt, started at 15:29, ended in its third round at 15:38 without an error message: its process was gone, its exit code read 127 and its temporary folder was left behind, which the bench's own cleanup would have removed, so it was ended from outside. The cause is not known. Nothing of it is used; its leftover folder was removed and the run above is the second and only one that counts.

## Not covered / still needs a person

- The margins come from 5 rounds on a machine shared with other sessions. If a quiet-machine measurement gives much narrower spreads, lowering a limit is an ordinary commit (R3.4).
- Release-mode figures only: PR mode on CI runs the 2,000-test subset on a different machine, and judges only the measures that do not depend on the machine (R3.5).
- The `machine: true` limits (times, CPU, memory) hold for this machine and Windows build only.
