---
summary: "GPUI P2 recheck: snap and cursors now pass, but VL-29 still misses (app CPU 5,438 ms against Electron's 4,156) and the Light title bar differs"
---

# GPUI Grace check: phase P2's misses, rechecked on Windows

Date: 2026-11-11
Commit: `develop` at 39d6ed8, GPUI Grace built with `cargo build -p grace --release`. It holds the fixes for the [P2 exit check](2026-11-10-gpui-p2-exit.md)'s misses: #323 (the hand over a row's Run button), #325 (a snapped window is kept), #326 (the 16 px rem), #327 (the stepped marks and the cached panes), and #329 (`gpui-pre` 0.3.12, GPUI Kit 0.9.1, Rust 1.99.1). The CPU baseline is `develop` at 7ae9683, just before #327, built the same way. Electron Grace is the exit check's build of 91f50ae (`npm run package`).
Machine: the reference machine of `bench/limits.json`, as in the exit check: Windows 11 Pro, AMD Ryzen 7 5800X (8 cores, 16 threads), 32 GB, NVIDIA GeForce RTX 4060 Ti, two screens of 2560x1440 at 125% (the second to the right of the main one).
Run by: an agent, through a remote session on the owner's machine with the owner's go-ahead, in separate git worktrees. Every window opened on the second screen.

Part of #281: the P2 rows that missed in the exit check, after their fixes, and the theme rows the exit check left for Windows in dark mode.

## Results

| Row | Exit check | Recheck |
|---|---|---|
| VL-29 2,000 tests are not slower | **miss**: app CPU 6,875 ms against 4,625 during Run all (+49%) | **still a miss, smaller**. With no UI Automation client, as most people run it: 5,438 ms against 4,156 (+31%), from 6,578 before #327. Run time, Java's CPU and memory are as good or better. See [VL-29](#vl-29-a-2000-test-run-all) (#320) |
| WB-18 a snapped window | **fail**: a Win+Left snap reopens unsnapped | **pass** on both screens, 3 restarts each, and a normal window does not drift. See [Snap](#wb-18-a-real-snap) (#318) |
| UP-13 / E-cursor-01 cursors | **pass but one**: the arrow over a row's Run button | **pass**: the hand. See [Cursors](#up-13-and-e-cursor-01-cursors) |
| PU-06 / E-look-01 the window follows Windows in dark mode | for a person | **pass**: with Windows in dark mode and no settings, the ground is `#151821` and the title bar dark |
| PU-14 the title bar in the app's theme | for a person | **differs from Electron Grace**: with Grace on Light in a dark Windows, the ground is light but the title bar stays dark. See [Theme](#pu-06-pu-14-and-e-look-01-the-theme) (#337) |
| Text size (#321) | 1 to 2 px smaller | **the text matches; other parts differ**: the owner's list of what still differs. See [Text](#text-size-321) |

## VL-29: a 2,000-test Run all

The exit check measured VL-29 with [vl.mjs](2026-11-10-gpui-p2-exit/vl.mjs), which reads both apps through UI Automation during the run. A UI Automation client changes what both apps do: GPUI Grace builds its accessibility tree only while an assistive technology is connected, and since #327 it reuses its panes' drawing between frames only while none is. Electron Grace's page builds no accessibility tree without one either. Most people run with none, so [vl-noat.mjs](2026-11-10-gpui-p2-recheck/vl-noat.mjs) measures that case. It finds Run all through UI Automation in one calibration launch, kills that process, and then starts each measured round in a fresh process that no UI Automation client touches. It clicks Run all at the calibrated place and samples the app's and Java's CPU until the run's Java exits.

`node --no-warnings docs/checks/2026-11-10-gpui-p2-recheck/vl-noat.mjs <uiprobe.exe> <exe> <2,000-test project> 3`, one app at a time on a quiet machine, the same generated project as the exit check. Medians of 3 rounds:

| Measure | GPUI 7ae9683 (before #327) | GPUI develop 39d6ed8 | Electron Grace |
|---|---|---|---|
| App CPU during Run all (ms) | 6,578 | **5,438** | 4,156 |
| Java CPU during Run all (ms) | 10,984 | 10,469 | 10,344 |
| App peak memory (MB) | 89.1 | 88.0 | 248.1 |
| Click to the run's end (ms) | 7,545 | 7,579 | 7,676 |

With the exit check's harness and its UI Automation reads (`vl.mjs`, 3 rounds, no WM_NULL pings), `develop`'s GPUI Grace uses 6,922 ms of app CPU (Java 10,797 ms, 89.1 MB), the same as the exit check's 6,875 ms. #327's gain shows only with no UI Automation client, as it was built to.

VL-29 still misses on one measure: with no UI Automation client, GPUI Grace's app CPU is 1.3 s more than Electron Grace's over a 7.6 s run (+31%, from +58% before #327). The run ends at the same time, Java's CPU is the same, and the app holds 65% less memory. #320 stays open.

Two changes to the harness during the check:

- The 2,000-test project's dry run lasts about 2 s, shorter than two of the sampler's PowerShell readings, so the harness never saw its Java and stopped with "Java never came and went". It now waits until 10 s after the start, then for two readings in a row with no Java.
- Once, in one `develop` round, the click on Run all started no run, and the round timed out. The cause was not found. The 3 rounds were run again with no such miss. The first batch's data, whose loop overlapped the broken round, was all discarded.

## WB-18: a real snap

On a fresh data folder, Grace was brought to the front and snapped with Win+Left (main screen) or Win+Right (second screen). After 1 s the window's rectangle was read. Grace was closed normally (`taskkill` without `/F`, so it saves), its saved `window` was read, and it was started again on the same data folder. Rectangles are `GetWindowRect` in physical pixels, with Windows' invisible borders. The saved bounds are logical pixels.

| Snap | Snapped window | Saved on close | Reopened, 3 restarts |
|---|---|---|---|
| Win+Left, main screen | x -8, y 0, 1296 x 1388 | x 0.8, y 18.4, 1022.4 x 1072.8 | x -8, y 0, 1296 x 1388 each time: exact |
| Win+Right, second screen | x 3832, y 0, 1296 x 1388 | x 3072.8, y 18.4, 1022.4 x 1072.8 | x 3832, y 0, 1296 x 1388 each time: exact |

The first Win+Right try never snapped (the keys' timing); the second did.

A normal window saved at x 2300, y 150, 1100 x 750 opened at x 2866, y 165, 1393 x 985 and stayed there over 3 restarts. Its saved bounds became y 150.4 and height 750.4 on the first close, then stayed the same: no drift.

## UP-13 and E-cursor-01: cursors

As in the exit check: the clean sample project, the pointer moved to each control's middle by its UI Automation rectangle, the shape read with `GetCursorInfo`, idle and during a Run all.

| Control | Exit check | Recheck |
|---|---|---|
| A tree row's Run button, idle / running | **arrow** | hand / hand |
| A tree row's label | hand | hand |
| Run all, idle / running | hand / not-allowed | hand / not-allowed |
| Cancel, idle / running | not-allowed / hand | not-allowed / hand |
| Report (disabled) | not-allowed | not-allowed |
| Env and Threads boxes | text | text |
| The empty details pane | arrow | arrow |

The splitter above the bottom panel read the arrow this time: the probe looks a fixed distance above the Console tab, which misses the 4 px band now that the rem is 16 px. It was not read again; the exit check read the up-down resize cursor there.

## PU-06, PU-14 and E-look-01: the theme

Windows was in dark mode (`AppsUseLightTheme` 0). Each start used a fresh data folder, with the window in front. The pixels are the window's ground, inside the client area under the status bar, and the title bar.

| Grace's Appearance | Ground | Title bar |
|---|---|---|
| none saved: follows Windows | `#151821` (dark, as expected) | `#1f221a` (dark) |
| Light | `#f3efe6` (light, as expected) | `#1f221a` (**still dark**) |

The window follows Windows in dark mode (PU-06, E-look-01). But the title bar follows Windows, not Grace's own choice: GPUI sets it from Windows' theme when it opens the window and when Windows' theme changes, and has no way to set it from the app. Electron Grace sets `nativeTheme.themeSource` from Appearance, so its title bar follows Grace's choice. PU-14 stays open on #337.

## Text size (#321)

GPUI Grace from `develop` (with #326's 16 px rem) on the left half of the second screen and Electron Grace on the right, both 1250 x 1125 physical, on the clean project.

The owner compared the two windows and listed what still differs. The text itself was not on the list; these were:

| What differs | Cause found in the code | Where it is fixed |
|---|---|---|
| Settings, at the bottom right, does not look the same | GPUI Kit's button draws its label at 14 px, over the size Grace gives it, and its icon at the button's size; Electron Grace's status bar button has 12 px text, a 14 px icon and 6 px each side | the look PR (#321) |
| The tag chips are bigger | the same: the chips' 11 px text came out at 14 px | the look PR (#321) |
| The bottom panel's tabs are bigger | the tabs used GPUI's `text_sm`, 14 px, where Grace's is 13, and 8 px each side where Electron Grace has 12 | the look PR (#321) |
| History's count is another colour and size | its pill used the control colour and 12 px text, where `countPill` has the selected colour, 11 px medium text and a 16 px height | the look PR (#321) |
| HTTP shows no count | the HTTP inspector comes in phase P3 (#282) | P3 |
| No debug buttons | the step debugger comes in phase P4 (#283) | P4 |
| The icons are bigger, the console's find icon too | GPUI Kit's buttons size their icon by the button's size (16 px), over the 14 px Grace gives the tree's, the console's and the find button's | the look PR (#321) |
| Hide non-matching is not centred on its box, and the box is bigger | GPUI Kit's checkbox draws a 16 px box at the top of its line; Electron Grace's is the browser's 13 px box on the middle of its label | the look PR (#321) |
| The progress bar is smaller, and its ends turn square during a run | GPUI clips the fill to the track's box, not to its rounded ends, so a coloured segment's ends are square; the track's own size is the same 6 px as Electron Grace's | the look PR (#321) |
| The console's bottom corners are square | the same clipping: the console's dark ground is drawn over the sheet's rounded corners | the look PR (#321) |

## Not done, and still for a person

- WB-16 with a screen really unplugged.
- UP-18 with Windows itself switched between light and dark while Grace is open.
- UP-14 and E-cursor-02, the busy cursor during a run: the exit check found none in either app. Whether GPUI Grace should show one is the owner's call.
