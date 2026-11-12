---
paths:
  - "crates/grace/**"
  - "crates/grace-core/**"
  - "crates/grace-e2e/**"
  - "patches/**"
  - "Cargo.*"
  - "rust-toolchain.toml"
  - "scripts/e2e-gpui.sh"
  - "scripts/package-gpui.mjs"
  - "scripts/bump-cargo.mjs"
---

# GPUI Grace

Grace is moving to Rust and GPUI ([ADR 0031](../../docs/decisions/0031-move-grace-to-rust-and-gpui.md)); the [migration guide](../../docs/specs/2026-11-08-gpui-migration.md) holds the phases, and the [parity checklist](../../docs/specs/2026-11-08-gpui-migration-parity.md) what each must prove. Until the cut-over the Electron app above ships, and GPUI Grace lives beside it in a Cargo workspace at the root: `crates/grace-core` (no GPUI, plain `cargo test`) and `crates/grace` (the app, built as `Grace.exe`).

- A port is not a transcription (guide R3.6): where Rust or GPUI does better than the TypeScript, do it that way, keeping what people see, the data on disk and the Karate command line at parity. Record each such change in the phase plan's Redesigns table, in the PR that makes it. Tick a parity row in the PR that ports it. Only the owner accepts an `n/a` on a row someone using Grace would notice (R3.3).
- A phase's issue closes only when all its work is done: every PR merged, every row ticked or handed on by its Proof text, its exit check passed.
- Stock GPUI and GPUI Kit first: their public API and stock behaviour. A new patch or a replaced widget states its reason in the PR and in the Redesigns table. Drop a patch as soon as upstream has the fix.
- Never post on Zed's repository: no issue, PR or comment. Follow upstream from this repository's issues.
- Before pushing Rust: `cargo fmt --all --check` after the last edit, clippy and `cargo test --workspace` as in the commands below, and `sh scripts/e2e-gpui.sh`.
- Pins: `gpui-pre` and GPUI Kit (`gpui-component`, `gpui-base`, `gpui-kit-assets`) exactly in the workspace's `Cargo.toml`, the toolchain in `rust-toolchain.toml`. They follow the newest releases ([ADR 0032](../../docs/decisions/0032-keep-the-gpui-set-rust-and-cargo-lock-current.md)): `cargo-updates` opens a bump PR every week and merges it once its tests pass, and `npm run cargo:bump` makes the same change in the working tree. GPUI and GPUI Kit move together, the patches rebased ([patches/](../../patches/README.md)). Never edit `patches/gpui-pre-windows/` by hand: change the `.patch`, then the copy; CI's `patches/check.sh` fails when they differ.
- Linux needs `libxkbcommon-x11-dev`, `libxcb1-dev`, `libfontconfig-dev` and `libfreetype-dev` to build; run the app under Xvfb on a machine without a screen, with a Vulkan driver (`mesa-vulkan-drivers` gives the software one on a machine without a GPU, such as CI). The Windows build needs nothing beside the exe.
- Colours only through `grace::theme` (`Theme::global(cx).tokens`, `Theme::color`): no `rgb(`, `rgba(`, `hsla(` or `Hsla {` elsewhere; `theme::tests::colours_come_only_from_the_theme` enforces it. Until the cut-over the tokens must equal `src/index.css`'s, which the theme's tests check: change both together.
- GPUI Kit reads its own theme: set it through `gpui_component::Theme::update` (as `theme::apply` does), never `Theme::global_mut`, whose edits do not reach the buttons.
- Icons only through `grace::assets::Icon`, the same Lucide files as Electron Grace's `lucide` version; a new icon in `src/renderer` fails `has_the_icons_of_electron_grace` until it is added to `crates/grace/assets/icons/`. GPUI Kit's own `IconName` holds only the icons its controls use.
- Tooltips only through `grace::tooltip`: `tooltip(id, text)` around the trigger (`.focusable(..)` for a control that takes focus, `.side(..)` in a row), or `cut(id, text)` for a line that may be cut. A tooltip shows after 500 ms on hover or at once on keyboard focus, one at a time, and Escape or a click closes it before anything else. The window's root goes through `a11y::tab_order`, so Tab moves the focus.
- Statuses only through `elements::status_mark`, buttons through `elements::button` and `icon_button` (GPUI Kit's `Button`, which brings focus, the ring and the accessible name). Turn a button off with `elements::Off::off`, not GPUI Kit's `disabled`, make a checkbox with `elements::checkbox` (its `off` turns it off) and a radio button with `elements::radio`, and a text box with `elements::input`: they set the cursor a browser would (the hand, the not-allowed sign, the text cursor), and the checkbox and radio are drawn as a browser draws Electron Grace's.
- Accessibility is set by hand: an element reaches the tree only with both `.id(...)` and `.role(...)`, then `.aria_label(...)` and its states. A `Role::Label` reads its name from `.aria_value(...)`, not `.aria_label(...)`. `div` has no live region: wrap it in `a11y::live`. On Linux the pinned AccessKit adapter drops a button's disabled state and a row's level and expanded state.
- Headless `#[gpui::test]` windows build no accessibility tree: GPUI fills it only when an assistive technology connects. Test the tree through the e2e driver, `crates/grace-e2e`: find nodes by role and name, wait with `App::wait_for`, never sleep. Its tests are `#[ignore]`d in `cargo test` and run by `scripts/e2e-gpui.sh`, which builds Grace with the `e2e` feature. Test hooks live behind that feature only (`grace::e2e`); the release build has none. On Linux the driver moves the pointer and types through `xdotool`, in the window's coordinates. A tree read can be torn mid-update: wait for all the final values together. A cached view (`.cached(..)`) adds no accessibility nodes: cache only while `!window.is_a11y_active()`.
- Through AT-SPI, AccessKit's `Application` role reads as `Embedded`, and a node answers `GetRole` but not `GetRoleName`. Nor can it read the text in a GPUI Kit `Input` (a combo box or spin button): on Linux an e2e test checks what the box did, and `App::wait_for_text` reads the text only on Windows.
- The bench reads GPUI Grace through `grace::probe`: with `GRACE_PROBE` set it prints `PROBE <name> <unix ms>` on stderr (`bench/gpui.mjs`). It is in the release build, the one the bench measures, and costs one look at the environment when off. A new probe goes through `probe::mark`, never its own `eprintln!`.
- Idle ([ADR 0030](../../docs/decisions/0030-no-wake-ups-while-hidden-and-an-idle-cpu-budget.md)): no timer, `cx.spawn` loop or `with_animation` that runs while no run is going. An animation runs only while its state lasts (a running test's mark), and not with reduced motion.
- End-to-end tests before a PR is ready: run those of the app the PR changes (`sh scripts/e2e-gpui.sh` for `crates/`). Small: the spec files of the touched area. Feature and Project: the whole suite once, and paste its summary in the PR. CI runs no e2e on PRs into `develop`; PRs into `main` run it on CI once ready.
- A PR that changes what the app shows carries before and after screenshots (a Small PR only when the look changes). They live on the never-merged branch `feature/pr-screenshots`, as `pr-<N>/<name>-before.png` and `-after.png`, linked by `https://raw.githubusercontent.com/hpagniez/grace/<sha>/...`. Take GPUI Grace's on Linux under Xvfb, or on [the owner's machine](../../AGENTS.md#graces-own) with his go.
- A PR that changes `crates/` gets a Windows run by hand before it merges (`gh workflow run test.yml --ref <branch>`). `merge-when-green` does not wait for it, and a new dispatch cancels the one before: add the label only once the run is green.

## Commands

| Command | What it does |
|---|---|
| `cargo run -p grace` | Run GPUI Grace, the port in `crates/` |
| `cargo test --workspace` | GPUI Grace's tests: `grace-core` and the headless views |
| `npm run package:gpui` | Build GPUI Grace in release and lay it out as the release zip (ADR 0019) in `out/gpui/`; Windows, after `npm run vendor`. Not released until phase P5 |
| `npm run bench:gpui` | The bench on GPUI Grace: size, start, idle and `idle.wakeups.minimised` (no Playwright). `-- --a <exe>` compares it with another build, such as Electron Grace's |
| `npm run cargo:bump` | Move GPUI Grace to the newest `gpui-pre`, GPUI Kit and Rust, the Windows patches rebased, then `cargo update` (ADR 0032); prints what moved. `cargo-updates` runs it weekly |
| `sh scripts/e2e-gpui.sh` | GPUI Grace's end-to-end tests through the accessibility tree (`crates/grace-e2e`); on Linux it starts Xvfb and AT-SPI itself |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo fmt --all` | GPUI Grace's lint and format |
