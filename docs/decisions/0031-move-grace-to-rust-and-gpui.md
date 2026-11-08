---
status: accepted
date: 2026-11-08
decision-makers: Tusquito
---

# Move Grace to Rust and GPUI

Supersedes [ADR 0001](0001-solid-router-spa-without-tanstack-start.md), [ADR 0004](0004-playwright-for-end-to-end-tests.md) and [ADR 0005](0005-vitest-for-unit-tests.md) once the cut-over release ships ([migration guide](../specs/2026-11-08-gpui-migration.md), R10; [#278](https://github.com/hpagniez/grace/issues/278)). Until then they stay in force for the Electron app.

## Context and Problem Statement

Grace runs on Electron, SolidJS 2 RC and Tailwind ([ADR 0001](0001-solid-router-spa-without-tanstack-start.md)). Running lightly is its first priority ([ADR 0023](0023-run-lightly-as-a-release-requirement.md)), and [#152](https://github.com/hpagniez/grace/issues/152) asked whether another stack would serve it better. The Tauri spike found no gain, since WebView2 is Chromium ([2026-10-29-tauri-spike](../research/2026-10-29-tauri-spike/README.md)). The GPUI desk study ([2026-11-07-gpui](../research/2026-11-07-gpui.md)) and its prototype ([2026-11-07-gpui-prototype](../research/2026-11-07-gpui-prototype/README.md), [#275](https://github.com/hpagniez/grace/pull/275)) built Grace's main screen in Rust and GPUI and measured it against Grace on Linux and on the Windows reference machine.

On the reference machine, median of 5 rounds, unpatched GPUI against Grace `1bcfdcd`:

| Measure | Grace | GPUI |
|---|---|---|
| Idle memory | 167 MB | 28 MB |
| Run all, peak app memory, 2,000 / 10,000 tests | 249 / 671 MB | 34 / 40 MB |
| Run all, app CPU, 2,000 / 10,000 tests | 3.6 / 14.2 s | 0.3 / 1.3 s |
| First paint, profile reused | 497 ms | 325 ms |
| Unpacked size, Java included | 489 MB | 192 MB |
| Idle CPU a minute, minimised | 47 ms | 891 ms, 0 to 47 ms with the idle patch |

GPUI wins on all three priorities, in order, with one condition: unpatched, its Windows vsync thread keeps redrawing a minimised window, which [ADR 0030](0030-no-wake-ups-while-hidden-and-an-idle-cpu-budget.md) does not allow. A 74-line patch to `gpui-pre`'s Windows platform fixes it ([patches/](../research/2026-11-07-gpui-prototype/patches/README.md)); upstream tracks the same fix in [zed#66221](https://github.com/zed-industries/zed/issues/66221).

Should Grace move to Rust and GPUI, and on what terms?

## Considered Options

1. Stay on Electron and Solid, and keep trimming the Electron floor.
2. Move to Rust and GPUI, through GPUI Kit, carrying the Windows idle patch until upstream has it.
3. Move to Rust and GPUI without the patch, waiting for upstream before the cut-over.

## Decision Outcome

Chosen: option 2, by the owner, on 2026-11-08.

- Option 1 keeps a floor Grace cannot go below: the prototype's whole app uses a sixth of Grace's idle memory and a seventeenth of its run memory at 10,000 tests.
- Option 3 ties the release date to a pre-1.0 framework whose maintainers have said work outside Zed's needs will wait.

### The stack

| Layer | Choice, pinned exactly |
|---|---|
| Language | Rust, edition 2024; the toolchain pinned in `rust-toolchain.toml` (1.97 at the prototype) |
| UI | `gpui-pre` 0.3.11 (a snapshot of Zed's GPUI, `zed@f816e1f`) through GPUI Kit (`gpui-component`, `gpui-base`, `gpui-kit-assets` 0.9.0), upgraded together as one set, as the Solid set was |
| Idle patch | The Windows patch of the prototype, carried in the repository as a patched copy of the platform crate under `[patch.crates-io]`, with its `.patch` file beside it. Dropped when an upstream release has the fix |
| Typefaces, icons | IBM Plex, embedded ([ADR 0021](0021-bundle-ibm-plex-as-the-typeface.md)); Lucide's icons through GPUI Kit's assets ([ADR 0022](0022-lucide-as-the-icon-set.md)). Both stand |
| Java and Karate | Unchanged: the bundled JRE, Karate jar and debug listener ([ADR 0006](0006-bundle-jre-and-karate-jar.md), [ADR 0025](0025-compile-the-debug-listener-with-ecj-on-the-bundled-jre.md)), driven the way the Karate CLI does ([ADR 0026](0026-follow-the-karate-cli-first.md)) |
| Unit tests | `cargo test`; GPUI views through headless `#[gpui::test]` windows. Replaces Vitest ([ADR 0005](0005-vitest-for-unit-tests.md)) |
| End-to-end tests | A Rust test crate that drives the release build through the accessibility tree: UI Automation on Windows, AT-SPI on Linux. Replaces Playwright ([ADR 0004](0004-playwright-for-end-to-end-tests.md)) |
| Tooling | Node stays a development tool only, for `bench/`, `scripts/` (vendor, bump, check-pr, packaging) and their tests. Nothing in the shipped app runs on Node |

Each new crate the app depends on is named in the PR that adds it, with its size, as AGENTS.md asks.

### What does not change

- The release zip keeps the updater's contract of [ADR 0019](0019-update-the-zip-install-from-github-releases.md): the tag, the asset name `Grace-win32-x64-<version>.zip`, `Grace.exe` at the zip's root, `resources\app-files.json`, `update-result.json`, `plan.json`, the `started` and `switching` files, the backup layout and the `updates` key of `settings.json`. The last Electron release installs the first GPUI release with its own updater, and the GPUI updater can install, and roll back to, any earlier release.
- The data folder stays `%APPDATA%\Grace`, and `GRACE_USER_DATA` still overrides it for tests.
- Signing stays as in [ADR 0024](0024-sign-grace-exe-with-signpath-foundation.md): one `Grace.exe`.
- The performance rules of ADR 0023 and ADR 0030, the limits in `bench/limits.json` and the release benchmark. The bench keeps its process, start, idle, run and size measures. Its `page.*` measures read the DOM and the JS heap, which GPUI does not have; they are retired at the cut-over. Its `smooth.*` measures come from a frame probe in the app, off unless an environment variable turns it on, so they are compared with their limits but not with the last Electron release.
- Windows 11 x64 is the only platform Grace ships on. The app also builds and runs on Linux (X11), for development, CI and agents, without a release.
- git flow, Conventional Commits, the issue and PR rules, `merge-when-green` and the release-candidate scheme ([ADR 0015](0015-release-candidates-until-a-stable-release-is-decided.md)).

### How

The [migration guide](../specs/2026-11-08-gpui-migration.md) holds the plan: every feature of Grace mapped to its port, the phases, and the [parity checklist](../specs/2026-11-08-gpui-migration-parity.md) of every spec scenario, end-to-end test and unit test the GPUI app must pass before it replaces the Electron one. The Electron app keeps shipping from `develop` until then.

### Consequences

- Good: a large cut in memory, CPU during runs, start time and size, the three priorities of ADR 0023 in order.
- Good: one language for the whole app, and an app that runs on the Linux machines agents work on.
- Good: the updater contract and the data folder stay, so people update from the last Electron release in one click.
- Bad: a rewrite of about 14,500 lines of TypeScript and their 1,773 unit tests and 194 end-to-end tests, during which new features cost twice or wait.
- Bad: GPUI has no stable release and its line is split; Grace pins one snapshot and carries a patch until upstream has the fix. Every GPUI Kit update is a planned upgrade, like a Solid RC bump.
- Bad: agents know GPUI far less well than the web. AGENTS.md gets a "GPUI rules" section, as it had one for Solid 2.
- Bad: the accessibility tree is set by hand, element by element; nothing gives it for free as the DOM did. A test that walks the tree and fails on an unnamed control makes up for part of it.
- Bad: stored secret values are encrypted with Electron's `safeStorage`, whose key lives in Chromium's `Local State`. GPUI Grace encrypts with DPAPI directly and does not read them: secrets are entered again or imported again after the cut-over update, said in its `BREAKING CHANGE:` footer.
- Neutral: the wake-up gate of the Windows runbook (under 5 a second, minimised) cannot be met by any D3D11 app on an NVIDIA driver, Grace included. Whether that gate is dropped or counts only Grace's own threads is still the owner's call ([guide](../specs/2026-11-08-gpui-migration.md#open-questions)); ADR 0030's CPU limit applies either way.
