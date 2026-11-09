# GPUI Grace check: phase P0 on Windows

Date: 2026-11-09
Commit: `feature/gpui-p0-tooltip` at 3df5de4 (PRs #288, #289, #290 and #291 stacked)
Machine: the reference machine of `bench/limits.json`: Windows 11 Pro, AMD Ryzen 7 5800X (8 cores, 16 threads), 32 GB, NVIDIA GeForce RTX 4060 Ti (driver 32.0.16.2114), two screens of 2560x1440 at 125% (2048x1152 logical), Node 26.13.0, Rust toolchain from `rust-toolchain.toml`.
Run by: an agent, through a remote session on the owner's machine with the owner's go-ahead, in a separate git worktree.

The phase P0 exit check of the [migration guide](../specs/2026-11-08-gpui-migration.md) and #279: the empty Grace window passes ADR 0030 minimised and shown, `idle.wakeups.minimised` is under 5 a second, the zip has the updater contract's layout, and the parity rows of P0 are ticked. The window has no views yet, so the run, smooth and page measures wait for P2 and later.

## Tests

| Check | Result |
|---|---|
| `cargo test --workspace --locked` | 30 passed, 11 ignored (the e2e tests) |
| `sh scripts/e2e-gpui.sh`, the UI Automation driver's first run on Windows | `elements` 4 of 4 and `window` 3 of 3 passed. `tooltips`: 3 of 4 passed in the first run, then 4 of 4 in a rerun of the file and alone |
| The one failure | `escape_in_a_dialog_closes_the_tooltip_first`: no node held the focus through 30 presses of Tab. The test opens the dialog through the tree, never by a click, so the window was not the foreground one and Windows sent the keys elsewhere. Fixed with this record: the driver brings the window to the foreground before a key, as the Linux driver does with `xdotool windowfocus`. The script now runs every test file even when one fails (`--no-fail-fast`) |

## Zip

`npm run package:gpui`: `out\gpui\make\Grace-win32-x64-1.0.0-rc.9.zip`, 67,870,259 bytes (64.7 MiB), 620 entries; Electron Grace's 1.0.0-rc.9 zip is 206,168,661 bytes.

| Entry | Size |
|---|---|
| `Grace.exe` | 19,885,056 bytes |
| `resources\app-files.json` | 25 bytes |
| `resources\jre\` | the bundled JRE |
| `resources\karate.jar` | 12,507,496 bytes |
| `resources\listener.jar` | 4,451 bytes |
| `resources\licenses\` | Lucide's, IBM Plex Sans's and IBM Plex Mono's, as `forge.config.mts` adds them |

This is the layout of ADR 0019. The Electron updater still also requires `resources\app.asar`; the plan lists that for P5.

## Performance

`node bench/bench.mjs --release --b out/gpui/Grace-win32-x64/Grace.exe --b-zip <zip> --label-b "gpui p0"`, release mode, median of 5 rounds after a warm-up, on the second screen. The machine was not quiet: its load was 19.7% to 28.2% before the rounds, so the idle numbers are if anything high. No Electron build was measured beside it. The limits are Electron Grace's.

| Measure | GPUI Grace | Limit | |
|---|---|---|---|
| `size.unpacked` (MB) | 184.5 | 489.1 | within |
| `size.zip` (MB) | 67.9 | 207.2 | within |
| `start.paint.fresh` (ms) | 369 | 598 | within |
| `start.paint.reused` (ms) | 370 | 612 | within |
| `idle.memory` (MB) | 17 | 163.3 | within |
| `idle.cpu.shown` (ms) | 500 | 1453 | within |
| `idle.cpu.minimised` (ms) | 62 | 125 | within |
| `idle.wakeups.minimised` (/s) | **0.1** | aim 5, reported only | within the aim |
| `idle.wakeups.gpu` (/s) | 66.5 | reported only | the NVIDIA driver's thread (`nvwgf2umx.dll`), as found in #267 |

The bench's line reads `Result: miss` only because the measures that need Grace's views (`start.tree.*`, `run.*`, `smooth.*`, `page.*` and their `@10k` forms) were not measured. No measured value is over its limit. The full `bench.md` and `bench.json` stay on the owner's machine (`grace-p0-bench\`, beside the repository); this is not a release record, so they are not copied here.

## Restore and show after hide

The packaged `Grace.exe`, and the e2e build's gallery (real buttons and icons), each on the second screen with a fresh data folder. A step was judged by screenshots and by the share of sampled pixels that differ from the frame before the window left.

| Build | Step | First frame back at 0% difference | After |
|---|---|---|---|
| Packaged | 10 s minimised, restored | +207 ms | visible, not minimised, responding |
| Packaged | 10 s hidden, shown | +2 ms | visible, not minimised, responding |
| Gallery | 10 s minimised, restored | +213 ms, every control drawn | visible, not minimised, responding |
| Gallery | 10 s hidden, shown | +9 ms | visible, not minimised, responding |

The frames before about 200 ms after a restore show the window behind Grace, as Windows' restore animation does; none showed a blank or half-drawn Grace.

## Still needs a person

- Narrator and Accessibility Insights reading the window and the gallery (the Windows half of #269). The UI Automation e2e reads the same tree, but a screen reader's own reading has not been heard.
- An Intel or AMD GPU, or the Basic Display Adapter: the same idle check. The agent changes no driver on the owner's machine.
