# Idle at zero: patches to GPUI's refresh loop (#268)

Date 2026-11-07. The same cloud Linux machine as the [prototype](../README.md): Intel Xeon at 2.10 GHz, 4 vCPU, no GPU, Ubuntu 24.04, Xvfb at 1920 x 1080, Mesa llvmpipe. `gpui-pre` 0.3.11, Rust 1.97.

The prototype found that an idle GPUI window never stops working: on X11 a timer wakes the main thread at the screen's refresh rate for as long as the window is visible, and on Windows a `VSyncProvider` thread invalidates every visible window on every vsync. That broke the rule of the time that idle costs nothing ([ADR 0023](../../../decisions/0023-run-lightly-as-a-release-requirement.md)). These two patches stop the loop while nothing changes and restart it on the next notify, input, animation frame or throttled retry.

Since [ADR 0030](../../../decisions/0030-no-wake-ups-while-hidden-and-an-idle-cpu-budget.md), a shown window may spend a small CPU budget, and only wake-ups while it is minimised or hidden are ruled out. The patches are therefore optional: the X11 one only lowers a cost that is within budget, and only the Windows part that stops the vsync thread while every window is minimised or hidden is still needed. The plan is to offer them upstream to Zed, whose macOS backend already pauses this way, instead of keeping a fork.

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

GPUI already has the hooks. Its macOS backend pauses the Metal display link after a frame in which GPUI "neither drew, presented, nor asked for another frame" (`MetalFrames` in `gpui_macos/src/display_link.rs`), and GPUI core calls the platform's `frame_waker()` whenever a window becomes dirty, when `on_next_frame` is called (animations), and when a throttled frame must be retried. X11 and Windows return no waker and never pause. The patches port the macOS model:

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

### Windows ([gpui-pre-windows-0.3.11-idle.patch](gpui-pre-windows-0.3.11-idle.patch)): written, not run

Type-checked with `cargo check --target x86_64-pc-windows-msvc`, including its new unit test, but never run: there is no Windows machine here. It needs #267's machine.

- `draw_window` (the `WM_PAINT` handler) wraps GPUI's frame request in `begin_tick` and `end_tick`. A paint that GPUI did not use pauses the window.
- The vsync thread skips paused windows instead of calling `RedrawWindow` on them.
- When every window is paused, hidden or minimised, the vsync thread sleeps on a condition variable instead of looping on `DwmFlush`. That is the part that brings the thread's 60 to 120 wake-ups a second to zero.
- The waker unpauses its window and opens the gate. Opening a window and stopping the thread (`detach_gpu`, `Drop`) open it too.
- A re-entrant paint, which GPUI defers, wakes its window, so the deferred frame is still drawn at the next vsync, as before.

To check on Windows, against #267's numbers:

1. Idle CPU and wake-ups of the empty window and of the prototype, shown, minimised and hidden behind another window.
2. Resize, restore from minimised, move between monitors with different refresh rates, and drag the window: no frozen or stale frame.
3. Device loss: the thread checks for a lost GPU device only when it wakes, so check that a driver reset (`dxcap -forcetdr`) while idle still recovers on the next frame.
4. Scrolling and Run all: the same frame times as without the patch.

## For Grace

Until upstream takes it, Grace would carry these patches on the pinned `gpui-pre`: [apply.sh](apply.sh) copies the two crates from Cargo's registry into `patches/vendor/` and applies them, and `Cargo.toml` points `[patch.crates-io]` at them. The prototype's `Cargo.toml` does not do that yet, so its build stays as measured in the prototype.

```toml
[patch.crates-io]
gpui-pre-linux = { path = "patches/vendor/gpui-pre-linux" }
gpui-pre-windows = { path = "patches/vendor/gpui-pre-windows" }
```

Each `gpui-pre` upgrade would mean rebasing them: about 320 added lines (with comments and tests) across 6 files.

## Upstream proposal

Draft for an issue or PR on `zed-industries/zed`; not posted.

> **Pause the X11 refresh timer and the Windows vsync thread while a window is idle**
>
> On macOS, GPUI pauses the Metal display link after a frame in which nothing was drawn, presented or requested, and resumes it through `PlatformWindow::frame_waker`. X11 and Windows have no frame waker, so an idle window keeps waking at the display's rate: on X11 a calloop timer per visible window, on Windows the `VSyncProvider` thread invalidating every visible window on every vsync. Measured on X11 (Xvfb, llvmpipe), an idle empty window uses about 0.2 s of CPU a minute and 66 wake-ups a second; with the change, 0 s and none on the main thread.
>
> The change implements `frame_waker` and `schedule_frame` on both platforms with the same rule as `MetalFrames`: pause after a frame request in which GPUI neither presented nor woke the window. On X11, a paused window's timer is dropped and a calloop ping restarts it. On Windows, the vsync thread skips paused windows and sleeps on a condition variable when every window is paused, hidden or minimised.

Wayland is not patched: it is already paced by the compositor's frame callbacks.
