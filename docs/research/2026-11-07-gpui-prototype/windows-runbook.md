# Runbook: what is left to run on Windows (#267, #268, #269)

Everything the GPUI evaluation could not do on the Linux cloud machine, for the owner to run on the reference machine (`bench/limits.json`: Windows 11 Pro, Ryzen 7 5800X, 32 GB, a GPU, a second screen; both screens run at 120 Hz on the machine measured 2026-11-08). Each step has the command, what it should print or write, and what counts as pass or fail. A go for #152 needs every **gate** below to pass; the other checks are recorded but decide nothing alone.

Nothing here has been run on Windows. Step 1 proves the scripts work before the long runs.

| Step | What | Time | Gate |
|---|---|---|---|
| [0](#0-setup) | Setup | 20 to 40 min, once | |
| [1](#1-smoke-test) | Smoke test of the scripts | 15 to 25 min | The scripts run |
| [2](#2-the-measure-with-the-gpu) | Grace against GPUI, with the GPU, 5 rounds | about 90 min | Memory, start, idle shown, run |
| [3](#3-idle-while-minimised-and-hidden-adr-0030) | Idle minimised and hidden (ADR 0030) | about 25 min | No wake-ups while minimised or hidden |
| [4](#4-the-windows-idle-patch-268) | The same with the Windows idle patch | 30 to 60 min | Only if step 3 fails |
| [5](#5-smoothness-without-a-gpu) | Smoothness with the GPU off | about 90 min | Recorded, no gate |
| [6](#6-accessibility-with-narrator-and-accessibility-insights-269) | Narrator and Accessibility Insights | about 30 min | A usable tree |
| [7](#7-hand-back) | Commit the results and hand back | 5 min | |

Leave the machine alone during steps 2, 3, 4 and 5. Do not run `npm test`, `npm run e2e` or `npm run bench` at the same time (AGENTS.md).

## 0. Setup

```powershell
git clone https://github.com/hpagniez/grace.git; cd grace   # or in an existing checkout:
git fetch origin; git checkout feature/gpui-research         # until #275 is merged; then develop
cd docs\research\2026-11-07-gpui-prototype\measure\windows
powershell -ExecutionPolicy Bypass -File .\build-and-measure.ps1 -CheckOnly
```

Expected: `==> Checking the prerequisites`, then `==> All prerequisites are there.` Otherwise it prints the `winget` command for each missing one: Node 24, Rust on `stable-x86_64-pc-windows-msvc`, the Visual Studio 2022 Build Tools with the C++ workload and the Windows SDK. Install, open a new PowerShell, and run `-CheckOnly` again. Step 4 also needs Git for Windows (for Git Bash and `patch`), which comes with any Git install.

## 1. Smoke test

```powershell
powershell -ExecutionPolicy Bypass -File .\build-and-measure.ps1 -Quick
```

Expected: `npm install`, `npm run vendor` and `npm run package` when not done yet, then `cargo build --release` (5 to 15 minutes the first time), then one round of every measure. It ends with the table and writes `results\windows-gpu-quick.json`, `.md` and `.log` in the prototype's folder.

- **Pass:** the `.md` has a number in both columns for every row, and the exit code is 0.
- **If the build fails:** the first `error:` line says which tool or crate; `measure\windows\README.md`, "What is untested", lists the likely ones (`fxc.exe`, `rc.exe`, the linker).
- **If a row is empty:** the `.log` says which phase failed. The likeliest ones are the wheel (`scroll.*` reads 1 frame for GPUI) and the minimise (`idle.cpu.minimised` null for GPUI): both come from `window-tools.ps1`.

Delete the `-quick` files afterwards; they only prove the pipeline.

## 2. The measure with the GPU

```powershell
powershell -ExecutionPolicy Bypass -File .\build-and-measure.ps1            # label "gpu", 5 rounds, about 90 minutes
```

Writes `results\windows-gpu.json`, `.md` and `.log` in the prototype's folder. `node report.mjs ..\..\results\windows-gpu.json ..\..\..\..\..\bench\limits.json`, from `measure\windows`, prints the table again. Read it against these gates. "Beyond noise" uses the noise margin of the same measure in `bench/limits.json`.

| Measure | Gate for GPUI | Why |
|---|---|---|
| `idle.memory` | Lower than Grace by more than 8 MB, and under 163.3 MB | Priority 1; Linux read 161 against 199 MB |
| `idle.cpu.shown` | Under 1,453 ms (the limit, ADR 0030's budget) | ADR 0030: a budget while shown, not zero |
| `idle.cpu.minimised` | Under 125 ms | ADR 0030: no wake-ups while minimised. Expected to **fail** without the patch (zed-industries/zed#66221 reads about 3 % of a core, about 1,800 ms a minute); step 3 checks it properly |
| `run.memory.app`, `run.memory.app@10k` | Lower than Grace by more than 23 MB | Priority 1; Linux read 170 against 351 MB |
| `run.cpu.app`, `run.cpu.app@10k` | Not higher than Grace by more than 1,281 ms | Priority 1 |
| `start.paint.*`, `start.tree.*` | Not slower than Grace by more than the noise (69 / 181 ms fresh) | Priority 2 |
| `scroll.idle.maxFrame`, `scroll.run.maxFrame` | No frame over 50 ms (the [light-and-fast spec](../../specs/2026-10-31-light-and-fast.md)'s aim), and not worse than Grace by more than 8 ms | Priority 1, smooth screen. On Linux without a GPU it failed (47 / 126 ms); with a GPU it should pass |
| `size.zip` | Smaller than Grace | Priority 3; certain (40 against 279 MB on Linux) |

The Java columns (`run.memory.java`, `run.cpu.java`) should match within noise: both apps run the same Karate. If they do not, the run is suspect; run it again.

## 3. Idle while minimised and hidden (ADR 0030)

`build-and-measure.ps1` measures minimised CPU, but not wake-ups, nor a window that is shown but covered or on another desktop. This step does it by hand, for Grace and for GPUI. Paste this into a PowerShell window once:

```powershell
# CPU time of every process with this name over $Seconds, and the context switches of their threads (wake-ups).
function Measure-Idle([string]$Name, [int]$Seconds = 60) {
  $cpu = { (Get-Process -Name $Name | ForEach-Object { $_.TotalProcessorTime.TotalMilliseconds } | Measure-Object -Sum).Sum }
  $c0 = & $cpu
  $cs = Get-Counter "\Thread($Name*)\Context Switches/sec" -SampleInterval 1 -MaxSamples $Seconds -ErrorAction SilentlyContinue
  if (-not $cs) { Start-Sleep $Seconds }
  $c1 = & $cpu
  $perSec = if ($cs) { [math]::Round(($cs | ForEach-Object { ($_.CounterSamples | Measure-Object CookedValue -Sum).Sum } | Measure-Object -Average).Average, 1) } else { 'n/a' }
  "{0}: cpu_ms={1:N0} wakeups_per_s={2}" -f $Name, ($c1 - $c0), $perSec
}
```

`Get-Counter` uses English counter names. On a Windows in another display language (French on the reference machine) it finds nothing and the line shows `wakeups_per_s=n/a`; then read the wake-ups with WMI (`Win32_PerfRawData_PerfProc_Thread`, `ContextSwitchesPersec`, summed over the app's threads, before and after the span), as on 2026-11-08, or from [Process Explorer](https://learn.microsoft.com/sysinternals/downloads/process-explorer) instead (the "Context Switch Delta" column, per second, summed over the app's processes).

Start each app on the 2,000-test project, wait until the tree shows, then 20 s more, and measure each state for 60 s. The project from step 2 was in a temporary folder; make it again from the Grace checkout's root:

```powershell
cd <the Grace checkout>
node --input-type=module -e "const { writeBigProject } = await import('./e2e/big-project.ts'); console.log(writeBigProject('C:/grace-2k', 5))"   # prints { tests: 2000, ... }
# Grace (the packaged build from step 1): open C:\grace-2k from its home screen.
& .\out\Grace-win32-x64\Grace.exe
# GPUI, unpatched:
$env:GRACE_JAVA = "$PWD\vendor\jre\bin\java.exe"; $env:GRACE_KARATE_JAR = "$PWD\vendor\karate.jar"
& .\out\gpui-prototype-target\release\grace-gpui.exe C:\grace-2k
```

| State | How | Then |
|---|---|---|
| shown | The window on top, nothing over it | `Measure-Idle Grace` or `Measure-Idle grace-gpui` |
| minimised | Click its taskbar button, or Win+Down twice | the same |
| covered | Maximise another window (Notepad) over it | the same |
| other desktop | Win+Tab, drag the window to a new desktop, switch back to the first | the same |
| screen off | Settings, Power, screen off after 1 minute; leave it, then wake the screen and run the line within 60 s of the screen going off. Optional | the same |

Record the lines in `results\windows-hidden-idle.md`, one table per app, the same shape as [results/hidden-idle-linux.md](results/hidden-idle-linux.md).

- **Gate (ADR 0030):** minimised and on another desktop, GPUI uses under 125 ms of CPU in 60 s (the `idle.cpu.minimised` limit) and under 5 wake-ups a second, summed over its threads.
- **Expected:** GPUI unpatched fails while minimised. Its vsync thread keeps calling `DwmFlush` for as long as the app lives (zed-industries/zed#66221). Grace passes, as the bench's `idle.cpu.minimised` already shows.
- **Covered and screen off** decide nothing on their own: no desktop tells an app reliably that it is covered. Record them. On Linux, Grace dropped to 10 ms a minute when covered and unpatched GPUI kept drawing ([results/hidden-idle-linux.md](results/hidden-idle-linux.md)); note whether Windows does the same.

## 4. The Windows idle patch (#268)

Only if GPUI fails step 3's gate. The patch makes the vsync thread sleep when every window is paused, minimised or hidden. It was type-checked from Linux and has never run.

```bash
# In Git Bash, from the prototype's folder (docs/research/2026-11-07-gpui-prototype); out/ is the Grace checkout's:
sh patches/apply.sh                       # prints "patched .../patches/vendor/gpui-pre-windows"
CARGO_TARGET_DIR=../../../out/gpui-prototype-target-idle cargo build --release --example floor \
  --config 'patch.crates-io.gpui-pre-windows.path="patches/vendor/gpui-pre-windows"'
CARGO_TARGET_DIR=../../../out/gpui-prototype-target-idle cargo build --release \
  --config 'patch.crates-io.gpui-pre-windows.path="patches/vendor/gpui-pre-windows"'
```

Expected: the build prints `Adding gpui-pre-windows v0.3.11 (…patches/vendor/gpui-pre-windows)` near its start; without that line the patch is not in. The build rewrites `Cargo.lock` to point at the patched crate: run `git checkout Cargo.lock` before committing anything.

Then, in PowerShell, from `measure\windows`:

```powershell
powershell -ExecutionPolicy Bypass -File .\build-and-measure.ps1 -Label gpu-idle-patch -SkipBuild -SkipPackage -TargetDir ..\..\..\..\..\out\gpui-prototype-target-idle
```

…and step 3 again with `.\out\gpui-prototype-target-idle\release\grace-gpui.exe C:\grace-2k`.

- **Gate:** step 3's gate passes for the patched GPUI.
- **And nothing breaks**, checked by hand on the patched GPUI:
  1. Restore from minimised: the tree shows at once, with no stale or black frame.
  2. Resize by dragging an edge, and drag the window between the two screens: no frozen frame.
  3. Run all, then scroll the tree during the run: the progress moves and the wheel scrolls; `results\windows-gpu-idle-patch.md` shows no worse `scroll.*` than `windows-gpu.md` beyond 8 ms.
  4. GPU reset while idle: `dxcap -forcetdr` from an admin prompt (`dxcap` comes with the Graphics Tools optional feature), then click a row: it redraws within a second.
- **If it fails:** record what, and stop there. Since ADR 0030, the patch is optional for shown windows, and upstream is the place for this fix (zed-industries/zed#66221, tracked on #267).

## 5. Smoothness without a GPU

What a VM or Remote Desktop gives. Recorded only: it decides nothing, but a bad number is a risk to state in the go or no-go.

1. Device Manager, Display adapters, right-click the GPU, **Disable device**. The screen falls back to the Microsoft Basic Display Adapter. If it drops to 1024 x 768, set at least 1600 x 1000.
2. `powershell -ExecutionPolicy Bypass -File .\build-and-measure.ps1 -Label no-gpu -SkipBuild -SkipPackage`
3. **Enable the GPU again.**

Expected: `results\windows-no-gpu.*`. On Linux without a GPU, GPUI's longest frame was 47 ms idle and 126 ms during a run, against Grace's 17 and 33 ms. Note whether Windows' software renderer (WARP) does better.

## 6. Accessibility with Narrator and Accessibility Insights (#269)

Start GPUI on a copy of `test\fixtures\projects\clean` (9 tests), as in step 3: `& .\out\gpui-prototype-target\release\grace-gpui.exe <the copy>`.

**Accessibility Insights for Windows** (`winget install Microsoft.AccessibilityInsights`): Live Inspect, hover the window, then FastPass on it. Save the FastPass result as `results\windows-a11y-fastpass.a11ytest`.

**Narrator** (Ctrl+Win+Enter to start and stop). With Tab, Shift+Tab and the arrows, check each line:

| Check | Expected (from the Linux run through the same AccessKit tree) |
|---|---|
| Toolbar | Filter, Environment, Threads, Properties, Open…, Cancel, Run all are read with a name and a role. Known gaps: Threads and Properties may read without a name, Cancel may read as enabled when it is not |
| Tree rows | Each row reads its name and status ("Not run", "Passed", "Failed"). Known gap: no level or expanded state |
| Run all | Enter on Run all starts the run; Narrator says "Running 9 tests", then "Run failed: 8 passed, 1 failed" |
| Progress | The progress bar reads its value; the summary reads "9 of 9" when done |

- **Gate:** Run all can be reached and pressed from the keyboard, and the end of the run is announced. Each known gap that also shows on Windows goes into `results\windows-a11y.md`, with the ones that do not.
- **Not done yet:** a UI Automation port of `a11y-linux/e2e_run_all.py` (the Linux e2e test through the accessibility tree). Writing and debugging it needs a Windows machine; a Claude session on this PC through Remote Control could do it.

## 7. Hand back

```powershell
cd ..\..   # the prototype's folder
git checkout -b feature/gpui-windows-results
git add results\windows-gpu.* results\windows-no-gpu.* results\windows-gpu-idle-patch.* results\windows-hidden-idle.md results\windows-a11y*
git commit -m "docs(research): add the gpui prototype's windows results"
git push -u origin feature/gpui-windows-results
```

Then ask for the write-up: the tables go into the prototype's README, and the go or no-go on #152 follows the gates above. A go then needs a decision record that supersedes ADR 0001, ADR 0004 and ADR 0005.
