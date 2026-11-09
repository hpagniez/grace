# Plan: GPUI migration, phase P0 foundation

Date 2026-11-08. Issue [#279](https://github.com/hpagniez/grace/issues/279), under [#278](https://github.com/hpagniez/grace/issues/278). Spec: the [migration guide](../specs/2026-11-08-gpui-migration.md) (Phases, R4 to R8), [ADR 0031](../decisions/0031-move-grace-to-rust-and-gpui.md). Checklist: [parity](../specs/2026-11-08-gpui-migration-parity.md), phase P0 rows.

P0 builds what every later phase stands on, and ends with an empty Grace window that passes ADR 0030 on the reference machine and that Narrator reads. It lands in three PRs into `develop`, each `Part of #279`, then a by-hand check on Windows.

## PR 1: the workspace, the look and CI

1. **Workspace.** `Cargo.toml` at the root with `crates/grace-core` and `crates/grace`; `docs/research` excluded, so the prototype still builds on its own. `rust-toolchain.toml` pins Rust 1.97.0 with rustfmt and clippy; `rustfmt.toml` keeps the prototype's 120 columns. `Cargo.lock` is committed. `target/` is ignored.
2. **Pins.** `gpui-pre` and `gpui-pre-platform` 0.3.11, `gpui-component` and `gpui-kit-assets` 0.9.0, all `=`, in `[workspace.dependencies]`.
3. **Idle patch.** `patches/gpui-pre-windows/`: crates.io's `gpui-pre-windows` 0.3.11 with the prototype's 74-line patch applied, the `.patch` beside it, `[patch.crates-io]` pointing at it. `patches/check.sh` rebuilds the copy from Cargo's registry and fails on any difference; CI runs it.
4. **grace-core.** The crate, with `paths::data_dir()`: `GRACE_USER_DATA`, else `%APPDATA%\Grace` or `~/.config/Grace`, as Electron Grace. Everything else in it is P1.
5. **Theme** (`grace::theme`). The 30 colour tokens of `src/index.css` in `LIGHT` and `DARK`, the shadow and backdrop colours, and one `METRICS` both themes share (type scale, durations, sheet, focus ring, radii, control heights, shadow geometry). `apply` maps them onto GPUI Kit's theme through `Theme::update`; `follow_system` follows the window's appearance, `GRACE_THEME` pins it. Tests: the tokens and sizes equal `src/index.css` until the cut-over, the contrast rules of `tokens.test.ts`, and the palette rule (no colour literal outside `theme.rs`).
6. **Assets** (`grace::assets`). IBM Plex Sans (variable) and Mono 400 and 500, TrueType, embedded. The 50 Lucide icons Electron Grace imports, from `lucide-static` 1.64.0 (the same version as its `lucide`), as `Icon`; GPUI Kit's own icons served after them. A test fails when `src/renderer` imports an icon `Icon` lacks.
7. **Elements** (`grace::elements`). `icon`, `status_mark` (the `MARK` table of `StatusMark.tsx`: shape, colour token, accessible name; the running mark turns only while running and not with reduced motion), `button` (plain, primary, danger, ghost; 28 px, 24 px or a dialog's 14 px text) and `icon_button`, on GPUI Kit's `Button` for focus, the ring and the accessible name.
8. **Accessibility base** (`grace::a11y`). The prototype's live region, `live`.
9. **Window** (`grace::window`). `Grace.exe` opens one window titled Grace, 1200 by 800, at least 760 by 480, drawn in the ground colour, with the application role and the live region. A headless test opens it in the theme.
10. **CI.** `checks`: `cargo fmt --all --check`. `test`: a `rust` job on Ubuntu and Windows (`patches/check.sh` on Linux, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`), with `actions/cache` on the registry and `target/`. Clippy needs a full GPUI build, so it sits in `test`, not in `checks` as guide R6.5 says: `checks` stays a few minutes.
11. **AGENTS.md.** The Cargo commands, and a first "GPUI rules" section: the pins and the patch, the Linux packages, colours, GPUI Kit's theme trap, icons, the shared elements, accessibility, headless tests without a tree, idle.
12. **Parity.** Tick PU-01, PU-02, PU-04, PU-08, `palette.test.ts` and `tokens.test.ts`, each with its proof.

## PR 2: the e2e driver and the tree check

1. `crates/grace-e2e`: one `Driver` trait (find by role and name, read name, value, description and states, press, type, wait until the tree settles, read announcements), with an AT-SPI backend (Linux, `atspi` crate or the session bus) and a UI Automation backend (Windows, the `uiautomation` crate or `windows` bindings). Each crate is named with its size in the PR.
2. The `e2e` Cargo feature of `crates/grace`: reads `GRACE_E2E_SEAMS` and answers the dialog, open-path and clipboard seams; the release build has none of it. P0 adds the seam traits and their recorder, with nothing yet behind them but the window.
3. Windows on a screen other than the main one, as `e2e/helpers.ts` `launch()` does.
4. The tree check (guide R8.2): walk the whole tree of every window and fail on an interactive element (button, input, checkbox, tab, tree item, link, menu item) without a name. Its first subject is a test window with every shared element.
5. The tooltip helper (`tooltip()`, the 500 ms pause, `side`, one at a time, Escape and a click close it) and `cut`, with their tests: PU-19, PU-20, PU-21.
6. A first e2e test: the empty window opens, is named Grace, and has its live region. CI: the AT-SPI e2e on Linux under Xvfb for PRs into `develop`; UI Automation on Windows for PRs into `main` (ADR 0017).

## PR 3: packaging and the bench

1. A packaging script (Node, `scripts/`) that builds `Grace.exe` in release and lays out the zip of ADR 0019: `Grace.exe` at the root, `resources\app-files.json`, `resources\jre\`, `karate.jar`, `listener.jar`, `licenses\`, `switch.ps1`. The zip is not released until P5; a test checks the layout against the Electron zip's.
2. The bench launches `Grace.exe` without Playwright (guide R7.2): start, idle, run-less memory and size from processes and files. The frame probe: an environment variable turns it on, one check at start when off.
3. `idle.wakeups.minimised` (ADR 0031): context switches a second of Grace's threads while minimised and idle, limit 5, counting every thread except those whose start address is in a GPU driver's user-mode DLL (NVIDIA `nvwgf2umx.dll`, AMD `amdxx64.dll`, Intel `igd10iumd64.dll` and their siblings), which are reported on their own line. From the prototype's `measure/windows/thread-wakeups.ps1`. Added to `bench/limits.json` with its limit; reported, not judged, for the GPUI build until the cut-over (guide R7.1).

## By hand on Windows (the owner's machine)

Nothing runs on the owner's PC without the owner's go-ahead. After PR 3, a runbook in `docs/checks/` for the owner, then the check record:

1. On the reference machine: the empty window minimised and shown for 60 s each, `idle.cpu.minimised`, `idle.cpu.shown`, `idle.wakeups.minimised` under 5 a second; restore from minimised and show after hide draw at once.
2. Narrator and Accessibility Insights read the window (the Windows half of [#269](https://github.com/hpagniez/grace/issues/269)).
3. An Intel or AMD GPU, or the Basic Display Adapter if none is at hand: the same idle check (guide, Open questions).

## Exit check (from #279)

An empty Grace window passes ADR 0030 on the reference machine, minimised and shown, with `idle.wakeups.minimised` under 5 a second, and Narrator reads it; an Intel or AMD GPU is checked too; the zip has the contract's layout; CI is green on both platforms; the parity rows of phase P0 are ticked.

## Cost

PR 1 ships nothing: the Electron app is unchanged, and the Rust app is not packaged. What it adds:

- **CI time.** The `rust` job builds GPUI on two VMs, about 5 minutes on 4 cores cold, less from the cache. It runs beside the Electron `unit` halves, so `test` takes about as long as before once the cache is warm.
- **Repository size.** 0.8 MB of fonts, 50 icons of 0.3 KB, the patched crate (0.7 MB of source).
- **The app.** An empty window: GPUI's floor, about 28 MB idle on Windows in the prototype. No timer, poll or animation of its own; the running mark's animation exists only while a test runs.
