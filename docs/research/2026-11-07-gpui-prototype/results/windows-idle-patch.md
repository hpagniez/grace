# Windows: GPUI idle patch (#268)

2026-11-08, Ryzen 7 5800X, RTX 4060 Ti, both screens 120 Hz, French Windows 11 Pro. Prototype at b3dfddd, release builds. Patched: `patches/gpui-pre-windows-0.3.11-idle.patch`, target `out/gpui-prototype-target-idle`, the build printed `Adding gpui-pre-windows v0.3.11 (…patches/vendor/gpui-pre-windows)`; `Cargo.lock` restored afterwards.

**The patch fixes minimised CPU: 16 and 94 ms a minute against 1,000 and 1,031, under the 125 ms gate. It does not meet the wake-up gate: 69 context switches a second minimised, against the gate's 5.** The other switches are not GPUI's vsync loop (which the patch stops), and I did not trace which threads they come from.

## Method

Each build on `C:\grace-2k` (2,000 tests), one window on the second screen, one launch per round: tree shown, then 20 s, then 60 s shown; minimise, 2 s, then 60 s minimised. Order: unpatched, patched, patched, unpatched. `GRACE_PROBE` was on, which adds one thread blocked on stdin.

CPU: the runbook's `Measure-Idle` (sum of `TotalProcessorTime` for `grace-gpui`). Context switches: on this French Windows `Get-Counter`'s English path finds nothing, so they come from `Win32_PerfRawData_PerfProc_Thread.ContextSwitchesPersec` (cumulative, summed over the process's threads, read before and after), divided by 60. The machine was not guaranteed quiet: another session and the owner may have been using it.

## Numbers

| Build | Round | Shown: CPU ms in 60 s | Shown: switches/s | Minimised: CPU ms in 60 s | Minimised: switches/s |
|---|---|---|---|---|---|
| unpatched | 1 | 609 | 430.4 | 1,000 | 467.3 |
| unpatched | 2 | 672 | 358.6 | 1,031 | 371.6 |
| patched | 1 | 62 | 66.7 | 16 | 68.7 |
| patched | 2 | 31 | 66.8 | 94 | 68.6 |

Gate (runbook step 3, ADR 0030): minimised under 125 ms of CPU in 60 s and under 5 wake-ups a second. Patched: CPU passes in both rounds, wake-ups fail in both. Unpatched fails both. Shown CPU drops about tenfold too, which the gate does not ask for.

## Nothing breaks (runbook step 4, items 1 to 3)

One launch of the patched build, driven by real mouse and window calls, checked from screenshots:

| Check | Result |
|---|---|
| 1. Restore from minimised after 8 s minimised | Pass: full window drawn, no black or stale frame, in a screenshot 100 ms after the restore and another 1 s after |
| 2. Resize by dragging the right edge (200 px narrower) | Pass: layout follows the new width after the drag |
| 2. Drag by the title bar to the main screen and back (both 120 Hz) | Pass: window drawn correctly on each screen after the drag |
| 3. Run all, wheel during the run | Pass: progress moved (380 of 2,000 at 2.2 s), the tree scrolled, the run finished (1,960 passed, 40 failed, the project's own failures). Frame gaps with the wheel probe: 116 frames, p50 8.4, p95 9, longest 9.6 ms |

Not covered: frames during the drags themselves (screenshots came after each drag); item 4 (GPU reset with `dxcap`); a window covered or on another desktop; a second screen at another refresh rate, since both are 120 Hz here.
