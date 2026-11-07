# Measure the GPUI prototype on Windows (#267)

One command builds the [prototype](../../README.md) for Windows and measures it against the packaged Grace on the reference machine. Written on Linux and **never run on Windows**: [what is untested](#what-is-untested) comes first in the checklist below.

```powershell
# From a Grace checkout on the branch that has this folder, in a normal (not admin) PowerShell:
cd docs\research\2026-11-07-gpui-prototype\measure\windows
powershell -ExecutionPolicy Bypass -File .\build-and-measure.ps1 -Quick     # 10 to 15 minutes after the build: proves every step works
powershell -ExecutionPolicy Bypass -File .\build-and-measure.ps1            # the real run, about 90 minutes, label "gpu"
# then turn the GPU off (below) and:
powershell -ExecutionPolicy Bypass -File .\build-and-measure.ps1 -Label no-gpu -SkipBuild -SkipPackage
```

`-ExecutionPolicy Bypass` is needed because Windows 11 refuses to run `.ps1` files by default. Do not run it as administrator, do not run `npm test`, `npm run e2e` or `npm run bench` at the same time (AGENTS.md), and leave the machine alone while it measures: the bench wants a quiet machine, and the script waits for under 35 % CPU before each round as the bench does.

## What the script does

1. **Checks the prerequisites** and prints what to install for each one that is missing, then stops (`-CheckOnly` does only this):
   - Node 24 and npm (Grace's own need).
   - Rust 1.88 or newer on the `x86_64-pc-windows-msvc` target: `winget install Rustlang.Rustup`, then `rustup default stable-x86_64-pc-windows-msvc`. The prototype was built with 1.97.
   - The MSVC build tools and the Windows SDK: `winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"`. The SDK matters beyond the linker: GPUI's build script compiles its HLSL shaders with `fxc.exe` (the script finds it under `Windows Kits\10\bin\<newest>\x64` and sets `GPUI_FXC_PATH`), and the app's manifest needs `rc.exe`.
2. **Grace's side**: `npm install`, `npm run vendor` (the bundled JRE and `karate.jar` into `vendor\`; pass `-Vendor <folder>` to use another) and `npm run package` (the packaged Grace), each only when it has not been done.
3. **Builds the prototype**: `cargo build --release` and `cargo build --release --example floor` into `<Grace>\out\gpui-prototype-target` (git-ignored). A cold build takes 5 to 15 minutes.
4. **Measures** with [measure-windows.mjs](measure-windows.mjs) and writes `results\windows-<label>.json` (every round), `.md` (the table) and `.log` into the prototype's `results\` folder.

All parameters: `Get-Help .\build-and-measure.ps1 -Detailed`. The useful ones: `-Label`, `-Rounds` (5), `-Quick`, `-SkipBuild`, `-SkipPackage`, `-SkipLarge` (no 10,000-test project), `-SkipFluid`, `-SkipFloor`, `-KeepGpuiWindow`, `-Grace`, `-Vendor`, `-GraceExe`.

## What is measured

The same things as `measure/measure.mjs` and `fluid.mjs` on Linux, with the bench's own Windows sampler (`bench/system.mjs`: private working set and CPU time of the process tree, Java apart; one long-lived PowerShell, never one per sample), the bench's way of starting and driving the packaged Grace over CDP (`bench/app.mjs`) and its project generator (`e2e/big-project.ts`). Both apps alternate round by round, one warm-up each, 5 rounds after it, order flipping every round (`order()` of `bench/stats.mjs`). The measures carry the bench's ids, and the table shows Grace's limit from `bench/limits.json` beside them.

| Id | What | How, for each app |
|---|---|---|
| `start.paint.*`, `start.tree.*` | Start to first paint, and to the tree of the 2,000-test project, fresh and reused profile | Grace: first-contentful-paint and the first tree row over CDP (`startTimes`). Prototype: the `PROBE first_frame` and `PROBE tree_shown` lines on stderr (`GRACE_PROBE`), against the clock just before the spawn. The prototype has no profile, so its two figures are two samples |
| `idle.memory`, `idle.procs` | Private working set of the whole tree after 20 s, and its process count | `bench/system.mjs` `sample` |
| `idle.cpu.shown`, `idle.cpu.minimised` | CPU over 60 s, shown, then minimised (`ShowWindow(SW_MINIMIZE)` as the bench does) | `sample` before and after |
| `run.*`, `run.*@10k` | Run all on the 2,000-test and the 10,000-test project: peak memory (app, Java), CPU (app, Java), wall time, memory 3 s after the end | `startSampler` every 250 ms from before the click to the end. Grace is clicked over CDP and read through `run-outcome`; the prototype is sent `run` on stdin and answers `PROBE run_started` and `PROBE run_finished` |
| `scroll.idle.*`, `scroll.run.*` | 60 wheel notches over the tree, one per 16 ms, idle and 0.5 s into Run all: frames drawn, longest frame, 95th percentile | A real `WM_MOUSEWHEEL` from `mouse_event` ([window-tools.ps1](window-tools.ps1)) for both apps. Grace's frames are its `requestAnimationFrame` callbacks, the prototype's are the frames in which its tree lays out rows (`frames-start` / `frames-stop` on stdin). The first is a count of vsyncs, the second of changes: compare the longest frame and the percentile, not the count |
| `size.*` | The exe, the unpacked folder with and without Java, the zip | `folderMB` of the bench. Grace's zip is the one `npm run make` left in `out\make\zip\win32\x64`, if any; the prototype's is a deflate of the exe, an estimate. The prototype's "unpacked" adds the same JRE and jar beside it |
| floors | Each stack's empty 1400 x 900 window: idle memory, CPU over 60 s, processes, 3 rounds | `examples/floor.rs` against `measure/electron-floor` run with Grace's own `electron.exe` |

Differences from `npm run bench` that matter when reading the numbers against `bench/limits.json`:

- No `--release` extras: no `smooth.*` click-to-paint probes (the wheel probe above replaces `smooth.scroll.*`, and sends a real OS wheel where the bench sends CDP wheel events), no `page.*`, no `run.finishedLag`.
- A phase that fails is recorded (the `.md` lists it, the exit code is 1) and the rest goes on. If the first run of an app measures nothing at all, the script stops, since every later round would fail the same way.
- The prototype's window opens at 0, 0 of the main screen in probe mode. Grace's opens where the bench puts it, on the first screen that is not the main one. Unless `-KeepGpuiWindow` is given, the script moves the prototype's window to the middle of that same screen after the tree shows (`window-tools.ps1 move`), so both apps are drawn at the same refresh rate (59 Hz on the reference machine's second screen). The first-paint and tree times are taken before the move. The `.md` says where the window was.

## The run without a GPU

GPUI's Windows backend runs one thread that waits on every vsync (`DwmFlush`) and redraws, so the idle CPU and the smoothness depend on the GPU and the display. #267 asks for both cases:

1. Run with the GPU (above).
2. Device Manager, Display adapters, right-click the GPU, **Disable device**. The screen falls back to the Microsoft Basic Display Adapter (software rendering, what a VM or Remote Desktop gives). Set the resolution to at least 1600 x 1000 if it dropped to 1024 x 768, or the 1400 x 900 windows do not fit.
3. `.\build-and-measure.ps1 -Label no-gpu -SkipBuild -SkipPackage`.
4. **Enable the GPU again.**

The script records the display adapters, their driver and refresh rate in the JSON, so the file says which case it measured.

## What to do with the results

Commit `results\windows-gpu.*` and `results\windows-no-gpu.*` (the `.log` too if something failed; delete the `windows-*-quick.*` files of the `-Quick` runs, which are one round and only prove the pipeline), add the tables to the prototype's README as #267 asks, and propose go or no-go on #152. `node report.mjs results\windows-gpu.json ..\..\..\..\..\bench\limits.json` prints the table of a saved run again.

## What is untested

Everything here was written and checked on a Linux cloud machine, with no Windows and no GPU. What was checked, and what was not:

- **Checked on Linux**
  - `node --check` and a lint for undefined names on the three `.mjs` files.
  - `report.mjs` on a synthetic result.
  - `gpui-app.mjs` driven against the Linux build of the prototype under Xvfb: the start, the `first_frame` and `tree_shown` probes, `run`, `frames-start` / `frames-stop`, and the stop, with no stray process left.
  - The dependency graph for the Windows target (`cargo tree --target x86_64-pc-windows-msvc`: `gpui-pre-windows` and its Windows crates in, `gpui-pre-linux` and X11 out), and `cargo check --all-targets` on Linux after the `Cargo.toml` change.
  - `cargo check --target x86_64-pc-windows-msvc --all-targets`, cross-checked from Linux (with `llvm-rc` for the manifest): the prototype, its tests and the `floor` example type-check against GPUI's Windows backend, so `src/main.rs` and `src/karate.rs` have no Windows-only compile error.
- **Not checked, and the likeliest places to break on the first run**
  - **The prototype has never been linked or run on Windows.** The cross-check above stops before the linker and before the release build's shader step: in a release build on Windows, `gpui-pre-windows`'s build script compiles its HLSL with `fxc.exe`, and the prototype's exe links against the Windows SDK and the MSVC runtime. A missing tool or a linker error would show in `cargo build --release`, and the first lines of its output say which.
  - **The PowerShell scripts were not parsed.** There is no PowerShell here, so `build-and-measure.ps1` and `window-tools.ps1` were only read through. A typo is possible. `-CheckOnly` runs the first half of the build script, `-Quick` the rest.
  - **The Win32 calls of `window-tools.ps1`**: the per-monitor DPI context, `SetForegroundWindow` after an Alt tap, `SetCursorPos` and `mouse_event` for the wheel, `SetWindowPos` for the move. If the wheel does not reach the tree, `scroll.*` reads a frame count of 1 for the prototype. If `move` fails, the prototype stays on the main screen and the `.md` says so.
  - **`bench/system.mjs`, `bench/app.mjs` and `e2e/big-project.ts` used from outside the bench**: they are imported from the checkout and called the way `bench/bench.mjs` calls them, but `measure-windows.mjs` is a second caller. `big-project.ts` is loaded as TypeScript by Node 24's type stripping, as the bench does.
  - **The prototype's console.** `main.rs` has no `windows_subsystem` attribute, so the exe is a console program. Started by the script from a terminal it shares that terminal's console and no `conhost.exe` joins its tree. Started by a double click, it opens a console window, and that `conhost.exe` would count as app memory.
  - **`minimise` on the prototype**: it reads the process's `MainWindowHandle`. If the prototype's window does not report one, `idle.cpu.minimised` is null for it and the log says why.
  - **Timing**: the wheel probe starts its PowerShell 5 s ahead and logs when it started late; on a very slow machine the longest frame of that round includes the wait.

Nothing in the script changes the machine, except `npm install` / `npm run vendor` / `npm run package` in the Grace checkout and the build in `out\gpui-prototype-target`. It does not install anything itself.

## Why `Cargo.toml` changed

The prototype's `gpui_platform` dependency enabled the `x11` feature for all targets. That feature only exists to turn on the Linux backend; on Windows `gpui-pre-platform` picks DirectX 11 (`gpui-pre-windows`) from the target by itself. `cargo tree --target x86_64-pc-windows-msvc` already resolved with the old line, so it was not a blocker, but the dependency is now split by target: `x11` on Linux, nothing elsewhere. The Linux dependency graph is identical before and after (`cargo tree -e normal,features` diffed), and `Cargo.lock` did not change.
