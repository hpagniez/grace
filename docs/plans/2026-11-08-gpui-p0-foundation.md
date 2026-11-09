# Plan: GPUI migration, phase P0 foundation

Date 2026-11-08. Issue [#279](https://github.com/hpagniez/grace/issues/279), under [#278](https://github.com/hpagniez/grace/issues/278). Spec: the [migration guide](../specs/2026-11-08-gpui-migration.md) (Phases, R4 to R8), [ADR 0031](../decisions/0031-move-grace-to-rust-and-gpui.md). Checklist: [parity](../specs/2026-11-08-gpui-migration-parity.md), phase P0 rows.

P0 builds what every later phase stands on, and ends with an empty Grace window that passes ADR 0030 on the reference machine and that Narrator reads. It lands in four PRs into `develop`, each `Part of #279`, then a by-hand check on Windows.

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

1. `crates/grace-e2e`: one `Driver` trait that reads the tree as a snapshot (role, name, description, value, enabled, focusable, focused, selected, expanded, children) and presses a node; searching and waiting are plain Rust on the snapshot (`App::wait_for`, every 50 ms, never a fixed sleep). Backends: AT-SPI on Linux (`atspi` 0.29, already in the lock through GPUI's AccessKit adapter), UI Automation on Windows (`uiautomation` 0.25.1, on the `windows` 0.62 GPUI already uses). `App::launch` starts the built app with a fresh `GRACE_USER_DATA` and kills it and deletes the folder when the test ends.
2. The `e2e` Cargo feature of `crates/grace`, never in a release: `GRACE_E2E_SCREEN=other` opens the window on a screen other than the main one, as `e2e/helpers.ts` `launch()` does; `GRACE_E2E_VIEW=elements` opens a gallery of every shared element, the tree check's first subject. The dialog, open-path and clipboard seams of the guide's Architecture come with their first caller (P2, P3): a seam with nothing behind it would be guessed, not ported.
3. The tree check (guide R8.2): `Node::unnamed_controls` lists every button, check box, radio, entry, combo box, spin button, slider, tab, tree or list item, link and menu item without a name; a test fails on any, in the window and in the gallery.
4. Tests: the window opens named Grace with its live region and no unnamed control; every status mark reads as its name (PU-08); icon buttons are named; the driver presses a button and the app sees it. They are `#[ignore]` in `cargo test --workspace`, since they start the app; `scripts/e2e-gpui.sh` builds with the `e2e` feature and runs them, starting Xvfb, a session bus and the AT-SPI bus launcher on a Linux without a desktop.
5. CI: in the `rust` job, the AT-SPI e2e on Linux for every PR; the UI Automation e2e on Windows for PRs into `main` and runs by hand (ADR 0017). The step is named `e2e`, so `merge-when-green`'s back-merge check counts it.

## PR 4: tooltip and cut

1. `grace::tooltip`: `tooltip(id, text)` wraps a trigger, `cut(id, text)` a line that may be cut (its tooltip shows only when the line is cut, measured with the text system). One `Tips` entity per window draws the open tooltip with the Tooltip role, deferred above dialogs, placed by `gpui_base::Positioner` on its `side` (below, left, right, or over the line for `cut`). It shows after 500 ms of hover, also over a disabled button, or at once on a keyboard focus (`.focusable(..)`), once per focus; one at a time; Escape (taken before any other handler, so a dialog stays open) and a click close it. A keystroke makes GPUI report an un-hover, which the tooltip ignores while the pointer is still on its trigger. `elements::icon_button` no longer brings GPUI Kit's own tooltip.
2. Tab order: the window's root tracks a focus handle and binds Tab and Shift-Tab (`a11y::tab_order`), focused when the window opens; GPUI Kit's `Root` does this only for its own views.
3. The e2e driver gains a pointer and keys (`App::hover`, `App::key`, `wait_for_no_node`) and each node's place: `xdotool` on Linux, in the window's coordinates since AT-SPI's screen extents lack the X window's place without a window manager; UI Automation's mouse and keyboard on Windows. The gallery gets a disabled button with a tooltip, a cut line and a dialog. PU-19, PU-20 and PU-21 pass through the tree on Linux.

## PR 3: packaging and the bench

1. `scripts/package-gpui.mjs` (`npm run package:gpui`) builds `Grace.exe` in release and lays out the zip of ADR 0019 as Electron Grace's has it: `Grace.exe` at the root; `resources\app-files.json`, `resources\jre\`, `karate.jar`, `listener.jar` and `licenses\` (with Lucide's and IBM Plex's, as `forge.config.mts` adds them). `switch.ps1` is not in the zip: Electron Grace writes it into the staging folder at update time. The zip is not released until P5; `scripts/package-gpui.test.ts` checks the layout, the list and the zip's name against the updater's.
2. The bench launches `Grace.exe` without Playwright (guide R7.2, `bench/gpui.mjs`): a build with no `resources\app.asar` is GPUI Grace. It takes size, start (the first frame, from the probe) and idle shown and minimised; the run, smooth and page measures wait for the views. The probe is `grace::probe`: `GRACE_PROBE` turns it on, one look at the environment at start when off. GPUI Grace cannot be told where to open until it remembers its window (P2), so the bench moves it to its screen once it has drawn.
3. `idle.wakeups.minimised` (ADR 0031): context switches a second of Grace's threads over the minimised minute, counting every thread of the process tree except those whose start address is in a GPU driver's user-mode DLL (`system.mjs` `GPU_DRIVER`), which are `idle.wakeups.gpu`. From the prototype's `measure/windows/thread-wakeups.ps1`. Both are only reported: the catalogue shows the aim of 5 a second beside the first, and neither gets a line in `bench/limits.json` until the cut-over (guide R7.1). A limit there now would fail every Electron release bench, which has never been held to it.

Found while building it, for P5: the Electron updater refuses an unpacked update without `resources\app.asar` (`src/main/updates/fetchZip.ts`), so the last Electron release must drop that check before the first GPUI release; `Grace.exe` has no icon yet (Electron's comes from `assets/icon`); and the zip must carry the licences of the Rust crates GPUI Grace is built from.

## By hand on Windows (the owner's machine)

Nothing runs on the owner's PC without the owner's go-ahead. After PR 3, a check record in `docs/checks/`:

1. On the reference machine: the empty window minimised and shown for 60 s each, `idle.cpu.minimised`, `idle.cpu.shown`, `idle.wakeups.minimised` under 5 a second; restore from minimised and show after hide draw at once.
2. Narrator and Accessibility Insights read the window (the Windows half of [#269](https://github.com/hpagniez/grace/issues/269)).
3. An Intel or AMD GPU, or the Basic Display Adapter if none is at hand: the same idle check (guide, Open questions).

Done on 2026-11-09 for 1: [docs/checks/2026-11-09-gpui-p0-windows.md](../checks/2026-11-09-gpui-p0-windows.md). `idle.wakeups.minimised` was 0.1 a second, every measure taken is within Electron Grace's limits, the zip has the contract's layout, and the UI Automation e2e passed once the driver brought the window to the foreground before a key. The owner made 2 and 3 optional on 2026-11-09 (no other GPU at hand; the UI Automation e2e reads the same tree as Narrator), so they are not done.

## Exit check (from #279)

An empty Grace window passes ADR 0030 on the reference machine, minimised and shown, with `idle.wakeups.minimised` under 5 a second; the zip has the contract's layout; CI is green on both platforms; the parity rows of phase P0 are ticked. Narrator and an Intel or AMD GPU are optional (the owner, 2026-11-09).

## Cost

PR 1 ships nothing: the Electron app is unchanged, and the Rust app is not packaged. What it adds:

- **CI time.** The `rust` job builds GPUI on two VMs, about 5 minutes on 4 cores cold, less from the cache. It runs beside the Electron `unit` halves, so `test` takes about as long as before once the cache is warm.
- **Repository size.** 0.8 MB of fonts, 50 icons of 0.3 KB, the patched crate (0.7 MB of source).
- **The app.** An empty window: GPUI's floor, about 28 MB idle on Windows in the prototype. No timer, poll or animation of its own; the running mark's animation exists only while a test runs.
- **The tooltip (PR 4).** No new crate: `gpui-base` was already built through GPUI Kit. One timer per hover, spawned only when the pointer rests on a trigger and dropped when it leaves or the tooltip shows; nothing runs while idle or hidden. One small entity per window.
