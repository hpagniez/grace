# Idle CPU while shown: the console cursor's blink

Checked on 2026-10-31 on the reference desktop (Windows 11 Pro, Ryzen 7 5800X, 32 GB; main screen 2560x1440 at 120 Hz, the window on the second screen at 59 Hz), with the packaged app built from `develop` at 6bdd936a and from this fix. Bench of [#147](https://github.com/hpagniez/grace/issues/147): the baseline's `idle.cpu.shown` read 1.5 to 2.8 s of CPU per 60 s idle on this machine, against 157 to 266 ms on the GitHub Windows runner.

**Bottom line:** the console's block cursor after "Ready." blinked forever (`animate-blink`, `steps(1)`, 1.1 s, infinite). It was the only animation on the idle page. Each step made Chromium restyle and redraw the window, so the GPU process and the renderer worked all minute. The cursor now blinks only while Grace finds tests or starts Karate. Idle CPU fell from 1.3 to 2.1 s per minute to about 0.23 s, and the GPU process from about 1 s to 31 ms.

## Method

The app was started as the bench starts it: the 2,000-test project (`writeBigProject(dir, 5)`), updates off, a 1400 x 900 window on the second screen, no CDP. After 20 s, the CPU time (kernel + user) of every process in its tree was read from `Win32_Process`, again 60 s later, and grouped by Chromium's `--type`. A second start with CDP listed `document.getAnimations()` and took the page's `Performance.getMetrics` over 10 s. The floor is Grace's own Electron with a blank `data:` page in the same window. The old and fixed builds ran alternately, two rounds each, on a quiet machine.

## Numbers

CPU ms over 60 s, shown and idle:

| Build | Browser | GPU process | Renderer | Total |
|---|---|---|---|---|
| Old, round 0 | 234 | 1,000 | 891 | 2,141 |
| Old, round 1 | 125 | 1,109 | 422 | 1,656 |
| Old, round 2 | 31 | 734 | 500 | 1,265 |
| Fixed, round 1 | 0 | 31 | 219 | 250 |
| Fixed, round 2 | 0 | 31 | 188 | 219 |
| Electron, blank page | 0 | 63 | 0 | 63 |
| Electron, blank page + the same blinking cursor | 141 | 953 | 422 | 1,516 |

The blinking cursor alone, on a blank page, costs what the old idle Grace cost. Nothing else in Grace adds to it.

The page over 10 s idle:

| Build | Running animations | Style recalcs | Layouts | Page task time |
|---|---|---|---|---|
| Old | `blink` on `console-cursor` | 54 | 0 | 19 ms |
| Fixed | none | 0 | 0 | 0.1 ms |

The home screen with no project open (the cursor after "Open a Karate project folder") is idle the same way once fixed: no animation and no page work.

## What is left

The fixed build's renderer still uses about 0.2 s per minute while the page runs no task, restyle or layout. This is Chromium's own housekeeping for a page of 2,255 nodes: the blank page has 1 node and reads 0. Nothing in Grace's code runs.

## Why CI read so much less

Not checked. The runner's desktop has no GPU and nobody looks at it, so Chromium probably draws few or none of the blink's frames there. This is also why `perf` on a PR would barely show this fix: the drop shows on a real screen, in the release benchmark.

## Follow-up

- Once this fix and the bench ([#167](https://github.com/hpagniez/grace/pull/167)) are both on `develop`, lower `idle.cpu.shown` in `bench/limits.json` from a release benchmark run. Lowering a limit is an ordinary commit.
