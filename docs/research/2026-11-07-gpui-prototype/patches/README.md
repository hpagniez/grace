# Idle at zero: patches to GPUI's refresh loop (#268)

Date 2026-11-07. The same cloud Linux machine as the [prototype](../README.md): Intel Xeon at 2.10 GHz, 4 vCPU, no GPU, Ubuntu 24.04, Xvfb at 1920 x 1080, Mesa llvmpipe. `gpui-pre` 0.3.11, Rust 1.97.

The prototype found that an idle GPUI window never stops working: on X11 a timer wakes the main thread at the screen's refresh rate for as long as the window is visible, and on Windows a `VSyncProvider` thread invalidates every visible window on every vsync. That broke the rule of the time that idle costs nothing ([ADR 0023](../../../decisions/0023-run-lightly-as-a-release-requirement.md)). The X11 patch stops the loop while nothing changes and restarts it on the next notify, input, animation frame or throttled retry. The Windows patch only stops it while every window is minimised or hidden.

Since [ADR 0030](../../../decisions/0030-no-wake-ups-while-hidden-and-an-idle-cpu-budget.md), a shown window may spend a small CPU budget, and only wake-ups while it is minimised or hidden are ruled out. The patches are therefore optional: the X11 one only lowers a cost that is within budget, and only the Windows part that stops the vsync thread while every window is minimised or hidden is still needed. The plan is to get the fix upstream rather than keep a fork; see [Upstream](#upstream).

## Upstream

Searched in zed-industries/zed on 2026-11-07. Both halves are already reported or proposed there, so Grace adds nothing new by opening its own:

- **X11:** draft PR [zed#66449](https://github.com/zed-industries/zed/pull/66449) (2026-11-03, no review yet) does what the Linux patch here does: it implements `frame_waker` and `schedule_frame` for X11, drops the refresh timer after idle refreshes and restarts it through a calloop ping.
- **Windows, minimised:** issue [zed#66221](https://github.com/zed-industries/zed/issues/66221) (2026-10-27, open, needs triage) reports that a minimised GPUI window keeps redrawing on every vsync, about 3 % of a core, and proposes skipping hidden windows in `VSyncProvider`. That is the part ADR 0030 still needs. Related: [zed#58048](https://github.com/zed-industries/zed/issues/58048) (laptop fans while the screen is off) and draft PR [zed#63438](https://github.com/zed-industries/zed/pull/63438) (park the vsync thread while the display is off).
- **Windows, shown:** PR [zed#63182](https://github.com/zed-industries/zed/pull/63182) made Windows frame scheduling demand-driven, as the first Windows patch here did. A maintainer closed it on 2026-09-17, unmerged, saying they plan to do it differently in-house.
- **Wayland:** already fixed upstream by [zed#60690](https://github.com/zed-industries/zed/pull/60690), merged 2026-08-21.

So the useful upstream steps are a comment with these measurements on zed#66449 and zed#66221, not a new issue or PR. Nothing has been posted there.

## Result

On X11 the patch removes the refresh loop's cost. The main thread stops waking while the window is idle, and the remaining CPU is within the measure's 10 ms resolution of zero.

| Idle, shown, 60 s after 20 s to settle | Before | After |
|---|---|---|
| Empty window (`examples/floor.rs`), CPU, 3 rounds | 190 ms [170–220] | **0 ms** [0–20] |
| Empty window, thread wake-ups | 3,990 [3,871–3,990] | **240** |
| Prototype with a project open, CPU, 2 rounds | 170–180 ms | **10 ms** |
| Prototype with a project open, thread wake-ups | 3,990 | **240** |
| Prototype after Run all, CPU, 2 rounds | 170–190 ms | **10–20 ms** |

Wake-ups are the voluntary context switches of all the process's threads. The 240 left after the patch are not GPUI's frame loop: over 20 s, the main thread switched 0 times, a `blocking-1` thread (GPUI's executor) 40 times, and Mesa's `WSI` swapchain threads 26 times. The empty Electron window read 0 to 10 ms a minute in the prototype's [floor](../results/floor.log), so GPUI idles at Electron's floor with the patch.

What still works after the loop has paused (checked by hand under Xvfb, screenshots not kept): the tree appears when the dry run ends (a background task's notify), Run all shows live progress and the final 8 passed, 1 failed, a click opens a feature and selects a scenario, and hover highlights a row. The prototype's 7 tests do not use a real window, so they do not exercise the patch.

Raw numbers: [../results/idle-patch.log](../results/idle-patch.log). Scripts: [../measure/idle-patch.sh](../measure/idle-patch.sh).

## How it works

For X11 (and the first Windows version), GPUI already has the hooks. Its macOS backend pauses the Metal display link after a frame in which GPUI "neither drew, presented, nor asked for another frame" (`MetalFrames` in `gpui_macos/src/display_link.rs`), and GPUI core calls the platform's `frame_waker()` whenever a window becomes dirty, when `on_next_frame` is called (animations), and when a throttled frame must be retried. X11 and Windows return no waker and never pause. The patches port the macOS model:

- A per-window `FrameDemand` records, during each frame request, whether GPUI presented (`PlatformWindow::draw`) or woke the window (`frame_waker`, `schedule_frame`).
- After a frame request that did neither, the window pauses.
- The frame waker unpauses it.

GPUI decides when frames are needed, including presenting for a while after high-rate input; the patches only stop the loop between those decisions.

### X11 ([gpui-pre-linux-0.3.11-idle.patch](gpui-pre-linux-0.3.11-idle.patch))

- `RefreshState` gains `Paused`: visible, but the timer is removed. A tick that GPUI did not use returns `TimeoutAction::Drop` and moves the window to `Paused`.
- The waker only sets flags and pings a `calloop` ping source that the client owns; the ping's handler restarts the timers of paused windows that were woken. Restarting from the event loop, not from the waker, keeps it out of the client's `RefCell` borrows, since GPUI calls the waker from inside event handling and frames.
- Hiding a paused window moves it to `Hidden`, as before; showing it restarts the loop, as before.
- Expose events still present directly, as before.
- One unit test of `FrameDemand`.

### Windows ([gpui-pre-windows-0.3.11-idle.patch](gpui-pre-windows-0.3.11-idle.patch)): hidden windows only

Since ADR 0030 only the minimised and hidden case is required, so on 2026-11-08 the Windows patch was cut down to the approach [zed#66221](https://github.com/zed-industries/zed/issues/66221) proposes: +74 lines in 3 files, of which about 40 are code and the rest a unit test and comments. Shown windows behave exactly as upstream: the vsync thread still invalidates them on every vsync.

- The vsync thread no longer calls `RedrawWindow` on a minimised or hidden window.
- When every window is minimised or hidden, the thread sleeps on a condition variable (`VSYNC_GATE`) instead of looping on `DwmFlush`.
- `report_visibility` (called on `WM_SIZE` and `WM_SHOWWINDOW`, so on minimise, restore, show and hide) opens the gate, and so do opening a window and stopping the thread (`detach_gpu`, `Drop`). Restoring a window paints it, as before.
- One unit test of the gate.

Type-checked for `x86_64-pc-windows-msvc` with its test; the gate's test also passes on Linux. Not run on Windows yet.

The first, demand-driven version (the macOS model above, also pausing shown windows) is in this branch's history at `9852684`; [../results/windows-idle-patch.md](../results/windows-idle-patch.md) measured it. It brought minimised CPU from about 1,000 to 16 to 94 ms a minute, but left about 69 thread wake-ups a second minimised, which come from threads other than the vsync loop. The smaller patch should give the same minimised numbers, since in both versions the vsync thread sleeps while every window is minimised; that is still to be measured. It does not touch the 69 wake-ups either.

To check on Windows:

1. Minimised and hidden: CPU and wake-ups, against `results/windows-idle-patch.md`.
2. Restore from minimised, show after hide, a second window opened while the first is minimised: drawn at once, no black or stale frame.
3. Device loss while every window is minimised: the thread checks for a lost GPU device only when it wakes, so check that the first frame after restoring still recovers.

## For Grace

Until upstream takes it, Grace would carry these patches on the pinned `gpui-pre`: [apply.sh](apply.sh) copies the two crates from Cargo's registry into `patches/vendor/` and applies them, and `Cargo.toml` points `[patch.crates-io]` at them. The prototype's `Cargo.toml` does not do that yet, so its build stays as measured in the prototype.

```toml
[patch.crates-io]
gpui-pre-linux = { path = "patches/vendor/gpui-pre-linux" }
gpui-pre-windows = { path = "patches/vendor/gpui-pre-windows" }
```

Each `gpui-pre` upgrade would mean rebasing them: about 250 added lines (with comments and tests) across 5 files, 177 of them the optional X11 patch.

## Upstream proposal

Draft for an issue or PR on `zed-industries/zed`, written before the search in [Upstream](#upstream). Not to be posted: [zed#66449](https://github.com/zed-industries/zed/pull/66449) and [zed#66221](https://github.com/zed-industries/zed/issues/66221) already cover it, and Grace follows them from #268 and #267.

> **Pause the X11 refresh timer and the Windows vsync thread while a window is idle**
>
> On macOS, GPUI pauses the Metal display link after a frame in which nothing was drawn, presented or requested, and resumes it through `PlatformWindow::frame_waker`. X11 and Windows have no frame waker, so an idle window keeps waking at the display's rate: on X11 a calloop timer per visible window, on Windows the `VSyncProvider` thread invalidating every visible window on every vsync. Measured on X11 (Xvfb, llvmpipe), an idle empty window uses about 0.2 s of CPU a minute and 66 wake-ups a second; with the change, 0 s and none on the main thread.
>
> The change implements `frame_waker` and `schedule_frame` on both platforms with the same rule as `MetalFrames`: pause after a frame request in which GPUI neither presented nor woke the window. On X11, a paused window's timer is dropped and a calloop ping restarts it. On Windows, the vsync thread skips paused windows and sleeps on a condition variable when every window is paused, hidden or minimised.

Wayland is not patched: it is already paced by the compositor's frame callbacks.
