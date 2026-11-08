# Windows: GPUI idle patch (#268)

2026-11-08, Ryzen 7 5800X, RTX 4060 Ti, both screens 120 Hz, French Windows 11 Pro. Prototype at b3dfddd, release builds. Patched: `patches/gpui-pre-windows-0.3.11-idle.patch` in its first, demand-driven version (commit `9852684`; since cut down to hidden windows only, see [../patches/README.md](../patches/README.md)), target `out/gpui-prototype-target-idle`, the build printed `Adding gpui-pre-windows v0.3.11 (…patches/vendor/gpui-pre-windows)`; `Cargo.lock` restored afterwards.

**The patch fixes minimised CPU: 16 and 94 ms a minute against 1,000 and 1,031, under the 125 ms gate. It does not meet the wake-up gate: 69 context switches a second minimised, against the gate's 5.** The other switches are not GPUI's vsync loop (which the patch stops). They come from one NVIDIA driver thread that any D3D11 device gets, Grace's GPU process included: see [Where the wake-ups come from](#where-the-wake-ups-come-from).

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

## Where the wake-ups come from

2026-11-08, same machine, NVIDIA driver loaded as `nvwgf2umx.dll`. Each thread's context switches over 20 to 60 s, from [thread-wakeups.ps1](../measure/windows/thread-wakeups.ps1), which reads the same `Win32_PerfRawData_PerfProc_Thread` counter as above, plus each thread's start module. No admin rights here, so no WPR trace and no call stack.

**One thread started in the NVIDIA D3D11 driver makes nearly all of them, and it is there before GPUI does anything.** It spends no measurable CPU (0 ms in 30 s). The rate is close to Windows' default 64 Hz tick, so it looks like a driver timer; that is a guess, without a stack.

| What | Wake-ups a second | From the driver thread |
|---|---|---|
| GPUI, first patch, minimised (63 threads) | 65.8 | 65.4 |
| Bare D3D11 device, no window, flags 0 / BGRA | 64.6 / 61.9 | same thread, same start address |
| Same, `PREVENT_INTERNAL_THREADING_OPTIMIZATIONS` | 64.9 | same |
| Same, `SINGLETHREADED` | 61.1 | same |
| Packaged Grace, minimised, on `C:\grace-2k`: all four processes | 109.0 | 91.7, in the GPU process (92.5 in all) |

The control is [d3d-control.ps1](../measure/windows/d3d-control.ps1): a PowerShell process that creates a device and sleeps. No device-creation flag removes the thread. Releasing the device does: 21 driver threads before, none after. So GPUI could stop it only by dropping its D3D11 device while every window is hidden and recreating it on show, which is a much larger change than the idle patch and which I did not try.

**What this means for the gate.** Runbook step 3 asks for under 5 wake-ups a second minimised. Nothing using a D3D11 device on this driver can pass that, Grace included. [ADR 0030](../../../decisions/0030-no-wake-ups-while-hidden-and-an-idle-cpu-budget.md) already counts what the runtime does by itself against the CPU limit, not as a wake-up of the app. Excluding the driver thread, the prototype has 0.0 to 0.1 wake-ups a second minimised. Only NVIDIA was tested: this machine has no other GPU.

## The smaller patch (hidden-window skip)

`patches/gpui-pre-windows-0.3.11-idle.patch` as of b2e4235, target `out/gpui-prototype-target-idle`, one launch on `C:\grace-2k`, one window on the second screen, minimised, then two successive 60 s measures. The build printed `Adding gpui-pre-windows v0.3.11 (…patches/vendor/gpui-pre-windows)`; `Cargo.lock` restored. The machine was not quiet.

| Minimised 60 s | CPU ms | Wake-ups a second | Driver thread | Everything else |
|---|---|---|---|---|
| 1 | 0 | 60.9 | 60.8 | 0.1 |
| 2 | 47 | 61.1 | 61.0 | 0.0 |

Minimised CPU passes the 125 ms gate, as the first version did (16 and 94 ms). Shown CPU was not measured here.

| Check, from screenshots 100 ms and 1 s after | Result |
|---|---|
| Restore from minimised (`ShowWindow` restore) after 2 minutes | Pass: full window drawn at 100 ms |
| Open a second window while the first is minimised (probe command `open-window`, added for this) | Pass: drawn at 100 ms; the first stayed minimised |
| Show after hide (`SW_HIDE`, then `SW_SHOW`, from another process) | Not checked: `SW_SHOW` left the window invisible (`IsWindowVisible` false) in the unpatched build too, so this call cannot test it. The prototype has no hide path of its own. |
