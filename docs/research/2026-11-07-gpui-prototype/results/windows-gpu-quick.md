# Windows, first look: one quick round (preliminary)

Date 2026-11-08. The reference machine: Windows 11 Pro, AMD Ryzen 7 5800X (16 threads), 32 GB, NVIDIA GeForce RTX 4060 Ti (driver 32.0.16.2114), the measuring screen at 120 Hz. Grace at `d486141`, packaged; the prototype built for Windows with no idle patch. `measure/windows/build-and-measure.ps1 -Quick`: **one round** after the warm-up, 5 s to settle and **10 s idle** (the full run uses 20 s and 60 s), and the machine was not fully quiet (16 to 34 % CPU before the rounds). It proves the pipeline works on Windows. It is not the verdict: that is the 5-round run of [windows-runbook.md](../windows-runbook.md), step 2.

| Measure | Unit | Grace (Electron) | GPUI prototype | GPUI / Grace | Grace's limit |
|---|---|---|---|---|---|
| First paint, profile reused | ms | 583 | 367 | 0.63x | 612 |
| Tree shown (2,000 tests), profile reused | ms | 1,990 | 1,733 | 0.87x | 2,389 |
| First paint, fresh profile | ms | 697 | 405 | 0.58x | 598 |
| Tree shown (2,000 tests), fresh profile | ms | 1,997 | 1,858 | 0.93x | 1,963 |
| Idle memory: private working set, whole tree | MB | 172.2 | 27.8 | 0.16x | 163.3 |
| Idle processes |  | 4 | 1 | 0.25x |  |
| Idle CPU, window shown, **10 s** | ms | 343 | 156 | 0.45x | 1,453 per 60 s |
| Idle CPU, window minimised, **10 s** | ms | 141 | 235 | 1.67x | 125 per 60 s |
| Run all: peak memory, app | MB | 263.9 | 33.5 | 0.13x | 322.3 |
| Run all: peak memory, Java | MB | 176.4 | 247.4 | 1.40x | 249.9 |
| Run all: CPU, app | ms | 6,547 | 625 | 0.10x | 5,375 |
| Run all: CPU, Java | ms | 13,063 | 11,734 | 0.90x | 14,875 |
| Run all: wall time | ms | 7,511 | 5,607 | 0.75x |  |
| Run all: memory 3 s after the end, app | MB | 258.9 | 29.8 | 0.12x |  |
| Wheel over the tree, idle: frames drawn |  | 273 | 60 | 0.22x |  |
| Wheel over the tree, idle: longest frame | ms | 8.5 | 48.7 | 5.73x |  |
| Wheel over the tree, idle: 95th percentile frame | ms | 8.4 | 33.5 | 3.99x |  |
| Wheel during Run all: frames drawn |  | 91 | 73 | 0.80x |  |
| Wheel during Run all: longest frame | ms | 11.8 | 124 | 10.51x |  |
| Wheel during Run all: 95th percentile frame | ms | 8.4 | 57 | 6.79x |  |
| Size: the app's exe | MB | 249.8 | 27.3 | 0.11x |  |
| Size: unpacked folder, Java included | MB | 488.5 | 191.6 | 0.39x | 489.1 |
| Size: unpacked, without Java | MB | 324.2 | 27.3 | 0.08x |  |
| Size: zip (GPUI: deflate of the exe, an estimate) | MB | n/a | 10.1 | n/a | 207.2 |

Empty 1400 x 900 window, idle: GPUI 16.8 MB, 109 ms of CPU in 10 s, 1 process. Electron's was not measured: its binary had not been downloaded yet (fixed in the script since).

What it suggests, to be confirmed by the full run:

- **Memory is the big win, much bigger than on Linux.** On Linux most of GPUI's 161 MB was the software renderer (llvmpipe); with a real GPU driver the prototype idles at 28 MB against Grace's 172 MB, and peaks at 34 MB during Run all against 264 MB. The app's own CPU during Run all is a tenth of Grace's.
- **Start is faster:** first paint 367 against 583 ms.
- **The wheel rows below are not valid.** They read slow for GPUI only because of the probe: it timed the gap between wheel notches, not frames, since GPUI draws only when something changes ([windows-scroll.md](windows-scroll.md)). With the probe fixed, GPUI draws every frame at the screen's rate, 8.3 ms at 120 Hz, idle and during Run all, longest 10.4 ms.
- **Minimised, GPUI uses more CPU than Grace** (235 against 141 ms in 10 s), as [zed-industries/zed#66221](https://github.com/zed-industries/zed/issues/66221) predicts: the vsync thread keeps waking. Over 60 s that would be about 1.4 s, far over the 125 ms limit and ADR 0030's rule. Whether the Windows idle patch (#268) fixes it is being checked (`results/windows-idle-patch.md` when done).
- Java's peak memory differs (176 against 247 MB) for the same Karate run; with one round this is likely noise from GC timing, and the full run will tell.
