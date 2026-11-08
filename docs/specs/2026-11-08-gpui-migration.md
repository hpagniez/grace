# GPUI migration guide: port every feature of Grace to Rust and GPUI

Date 2026-11-08. Status: draft. Decision: [ADR 0031](../decisions/0031-move-grace-to-rust-and-gpui.md). Issue: [#278](https://github.com/hpagniez/grace/issues/278), under [#152](https://github.com/hpagniez/grace/issues/152). Checklist: [parity](2026-11-08-gpui-migration-parity.md).

## Intent

The owner chose to move Grace from Electron and Solid to Rust and GPUI ([ADR 0031](../decisions/0031-move-grace-to-rust-and-gpui.md)), on the strength of the [prototype](../research/2026-11-07-gpui-prototype/README.md). The prototype covered the main screen. Grace has 42 specs, 594 acceptance scenarios, 194 end-to-end tests and 1,773 unit and component tests behind it, plus everything Electron and Chromium do without being asked: text selection, focus, tooltips, scrollbars, an accessibility tree, file dialogs.

This guide is how the move happens without losing any of it. It names every feature, says where its port goes and what proves it, splits the work into phases that each end in something measurable, and sets the bar the GPUI app must clear before it replaces the Electron one.

Who: the agents who build the port, one phase issue at a time, and the owner, who checks the parity list and makes the cut-over call.

## Goals, non-goals, anti-goals

- Goal: GPUI Grace does everything Electron Grace does at the cut-over, as the people who use it see it: same features, same words, same keys, same data folder, same Karate command lines.
- Goal: every behaviour has a test in the new app before the old app goes, listed in the [parity checklist](2026-11-08-gpui-migration-parity.md).
- Goal: the update from the last Electron release to the first GPUI release is the ordinary one-click update.
- Non-goal: new features in the port. A feature not in Electron Grace at the cut-over waits until after it (R9).
- Non-goal: a Linux or macOS release. The app builds and runs on Linux for development and CI only (R5).
- Non-goal: porting the tooling. `bench/`, `scripts/` and the vendor pipeline stay in Node.
- Anti-goal: a cut-over with a known gap. A row of the parity list that is neither ported nor accepted as `n/a` by the owner blocks it.
- Anti-goal: a port that is heavier than the prototype promised. Every phase is measured (R7).

## Architecture

The Electron app has three parts that talk over IPC: main (`src/main`, Node), the renderer (`src/renderer`, Solid) and the types both share (`src/shared`). GPUI Grace is one process, in a Cargo workspace at the repository's root, beside the Electron app until the cut-over:

| Crate | Holds | Replaces |
|---|---|---|
| `crates/grace-core` | Everything without a window: the model, the Karate command, the run and its events, the tree, history, flaky, trends, settings, recent folders, secrets, project setup (Maven, Gradle, pom, libraries, sub-folders), watch, the editor link, report export, the updater. No GPUI dependency, so its tests run in plain `cargo test` | `src/main`, `src/shared` |
| `crates/grace` | The GPUI app, built as `Grace.exe`: the window, the views, the theme, accessibility, and the platform seams (dialogs, opening a path or URL, the clipboard) | `src/renderer`, `src/main.ts`, `src/preload.ts` |
| `crates/grace-e2e` | End-to-end tests that start the built app and drive it through the accessibility tree (R6) | `e2e/` |

- **No IPC.** The renderer's `window.grace` API (`src/shared/ipc.ts`, 38 calls) becomes method calls on one `Grace` entity; the `AppEvent` stream (`src/shared/events.ts`, about 25 event types) becomes updates to that entity's state. The run's events are still batched to one update per tick, as `src/main/batch.ts` does today, and only views whose data changed are notified.
- **Background work** (the Karate child process, tailing `karate-events.jsonl`, the dry run, the build tool, the debugger's socket, the file watcher, the update download) runs on GPUI's background executor or its own thread, and reaches the UI through the entity, never by blocking the UI thread.
- **State.** `src/renderer/store.ts` and `reduce.ts` hold the window's state today; their logic moves to `grace-core` as plain Rust and keeps its tests, and the views read it.
- **Seams for tests.** Dialogs, opening a path or URL, the clipboard, the update feed and the build tool are behind traits. A build with the `e2e` Cargo feature reads `GRACE_E2E_SEAMS` and records or answers them, as the Playwright tests stub `dialog.showSaveDialog` or `shell.openExternal` today. The release build has no such code.

## Requirements

### R1. Scope

1. The port covers everything Electron Grace ships on `develop` at the cut-over: each feature in the [feature map](#feature-map), each behaviour in the [parity checklist](2026-11-08-gpui-migration-parity.md), and each item of R4.
2. UI text stays word for word, including tooltips, notices, empty states and accessible names, unless a row of the parity list records the change and the reason. The end-to-end tests find controls by those names.
3. Keyboard behaviour stays: arrow keys in the tree and the HTTP list, Enter and Space, Escape order (tooltip, then dialog, then page), Tab order, the splitter from the keyboard, Find in the console.
4. Data formats stay as they are at the cut-over: `settings.json` (every key), `recent.json`, `window.json`, the run folders and `run.json` of [ADR 0010](../decisions/0010-run-history-one-folder-per-run.md), the secrets file import format of the [env-var properties spec](2026-11-04-env-var-properties.md). One exception: stored secret values (R5.4).
5. The Karate command line, the Java argument file, the environment cleaning and the exit-code handling stay byte for byte. `src/main/karate/command.test.ts` and `argfile.test.ts` port first, on the same expected strings.

### R2. Feature map

Each feature of Grace, where its code is today, where its port goes, and its phase. The [parity checklist](2026-11-08-gpui-migration-parity.md) breaks each one down to its scenarios and tests. See the [feature map](#feature-map) below.

### R3. What ported means

1. A row of the parity list is ported when the behaviour it names works in GPUI Grace and a test proves it. Its **Proof** column names the test; its box is ticked in the PR that ports it, never later.
2. Where two specs disagree, the later spec wins, then the e2e test as it is at the cut-over: they describe Grace as it ships. The porting PR says which it followed.
3. A row that cannot apply any more (it tests Electron, the DOM, Chromium or Solid itself) is marked `n/a` with the reason. Only the owner accepts an `n/a` on a row someone using Grace would notice.
4. A unit test ports test for test, with the same name in snake case and the same fixtures (`test/fixtures/events/`, `test/fixtures/projects/`). It may merge or split only when the PR says why.
5. Every bug fixed in Electron Grace stays fixed: the [regressions](#regressions-to-keep-fixed) list is part of the parity bar.

### R4. What Electron gave for free

Electron and Chromium do these without code; GPUI Grace must do each one explicitly, and each has a test:

| Behaviour | Electron Grace | GPUI Grace |
|---|---|---|
| Selectable, copyable text | Any text | Only where Grace makes it selectable: the console, the test name, file and error, step logs, HTTP headers and bodies, variables (GPUI Kit's selectable text, as in the prototype) |
| Keyboard focus, Tab order, focus ring | DOM | GPUI focus handles on every control, the focus ring from the theme ([premium UI spec](2026-10-29-premium-ui.md)); `look.spec.ts` "Every control shows the focus ring" ports to a headless test per view |
| Tooltips on hover and focus, closed by Escape or a click | `Tooltip`, `Cut` | GPUI Kit's tooltip wrapped in one `tooltip()` helper with the same delays and `side`; `Cut` becomes a text element that tells whether it is cut |
| Mouse cursor per action | CSS | GPUI's cursor style per element ([#87](https://github.com/hpagniez/grace/issues/87), `cursor.spec.ts`) |
| Themed scrollbars | CSS | GPUI Kit's scrollbars, themed ([#89](https://github.com/hpagniez/grace/issues/89)) |
| Light, dark, follow Windows | `nativeTheme` | GPUI's window appearance, and its change event |
| No white flash, first frame in the theme | `backgroundColor`, static startup screen | The first frame is drawn in the theme's ground; the window shows only after it ([#90](https://github.com/hpagniez/grace/issues/90)) |
| Reduced motion | `prefers-reduced-motion` | Windows' "Animation effects" setting (`SPI_GETCLIENTAREAANIMATION`), read at start and on change; every animation checks it |
| High DPI, several screens with different scales | Chromium | GPUI's scale factor; window bounds stored in logical pixels as today ([window bounds spec](2026-10-26-window-bounds.md)) |
| Native file dialogs | `dialog.showOpenDialog`, `showSaveDialog` | GPUI's `prompt_for_paths` and `prompt_for_new_path`, behind the dialog seam |
| Open a path, a URL, a `vscode://` link | `shell.openPath`, `openExternal` | GPUI's `open_url` and the platform's open, behind the seam |
| Context menu ("Open in VS Code") | `Menu.popup` | GPUI Kit's context menu |
| Clipboard ("Copy as cURL") | `navigator.clipboard` | GPUI's clipboard |
| Drag to reorder panel tabs | DOM drag | GPUI's drag and drop ([reorder spec](2026-11-04-reorder-panel-tabs.md)) |
| One instance per data folder, focus the first on a second start | `requestSingleInstanceLock`, `second-instance` | A named mutex per data folder and a message to the first instance's window |
| Closing during a run kills Java | `before-quit` with `taskkill` | The window's close handler cancels the run and waits for the process tree, as `cancel.spec.ts` "Closing the app during a run" checks |
| Child processes die with Grace | Node | A Windows job object with kill-on-close for Java and the build tool. The updater's switch script must not be in it: it is started with `CREATE_BREAKAWAY_FROM_JOB`, as it must outlive Grace ([ADR 0019](../decisions/0019-update-the-zip-install-from-github-releases.md)) |
| Screen reader tree | Chromium's, from the DOM | AccessKit, set by hand on every element (R8) |
| Live announcements | `aria-live` | The prototype's announcement element ([src/a11y.rs](../research/2026-11-07-gpui-prototype/src/a11y.rs)) |
| Text in the window's language | Chromium locale, en-US only ([#150](https://github.com/hpagniez/grace/issues/150)) | No locale files at all; Grace's text is English. Number and date formats stay en-US as today |
| Typefaces | IBM Plex as web fonts ([ADR 0021](../decisions/0021-bundle-ibm-plex-as-the-typeface.md)) | IBM Plex embedded in the exe; no network request (`look.spec.ts`) |
| The window's icon, taskbar grouping | Forge, the app user model ID | The `.ico` in the exe's resources and the same app user model ID |
| Virtual lists | `VirtualList` | GPUI's `uniform_list` or GPUI Kit's virtual list; 10,000 tests stay fast ([virtual lists spec](2026-10-30-virtual-lists.md)) |

### R5. Platforms

1. **Windows 11 x64 ships.** Every acceptance test passes there, and the release benchmark runs there.
2. **Linux (X11) builds and runs, unreleased.** It is where agents and CI do most of the work: `cargo test`, the headless view tests and the AT-SPI end-to-end tests run on Linux runners. A Linux release, if wanted, is its own issue.
3. What differs by platform, each behind one function in `grace-core` or `grace` with a test on each platform:

   | Concern | Windows | Linux |
   |---|---|---|
   | Drawing | DirectX 11 | GPUI's Linux renderer on X11 |
   | Idle while minimised | The idle patch is required (ADR 0030) | Unpatched GPUI already stops its timer while minimised or on another workspace ([hidden idle](../research/2026-11-07-gpui-prototype/results/hidden-idle-linux.md)) |
   | Accessibility | UI Automation through AccessKit | AT-SPI through AccessKit |
   | Secret values | DPAPI, current user (`CryptProtectData`) | The desktop's Secret Service; without one, secret properties are refused with a notice |
   | Cancel a run | `taskkill /PID <pid> /T /F`, recorded as cancelled | Kill the process group |
   | Java priority | `BELOW_NORMAL_PRIORITY_CLASS` ([#148](https://github.com/hpagniez/grace/issues/148)) | `nice` 10 |
   | Finding Maven and Gradle | `mvnw.cmd`, `gradlew.bat`, PATH, `MAVEN_HOME`, `M2_HOME`, `GRADLE_HOME`, IntelliJ's bundled Maven ([find build tools spec](2026-11-03-find-build-tools.md)) | `mvnw`, `gradlew`, PATH and the same variables |
   | Classpath separator | `;` | `:` |
   | Updates | The updater of ADR 0019 | Off: no Updates section, no startup check |
   | File watching | `ReadDirectoryChangesW` | inotify |

4. **Secret values do not carry over.** Electron's `safeStorage` keeps its key in Chromium's `Local State`; GPUI Grace uses DPAPI directly and does not read the old values. Their names and the non-secret values stay; the values are entered or imported again. Grace is still in development, so nothing reads or converts the old values ([No backward compatibility](../../AGENTS.md#no-backward-compatibility)); the cut-over PR says so in a `BREAKING CHANGE:` footer.

### R6. Tests

Three levels, each replacing one of today's:

1. **`grace-core`: `cargo test`.** The Vitest files of `src/main` and `src/shared` port test for test (R3.4). The tests that run the real bundled Java and Karate (`runner.*.test.ts`) stay real. Files run in parallel, as [ADR 0020](../decisions/0020-run-unit-test-files-in-parallel.md) allows, each on its own copy of a fixture project.
2. **Views: headless `#[gpui::test]`.** The component tests of `src/renderer` become tests of the views that replace them, in a headless window with a fake `Grace` state. The token rules of `palette.test.ts` and `tokens.test.ts` become tests on the theme: every colour comes from a theme token, and the two themes differ only in colours.
3. **End to end: `crates/grace-e2e`.** Each e2e test starts the built app with a fresh data folder (`GRACE_USER_DATA`) and the `e2e` seams, and drives it through the accessibility tree by role and name, as the prototype's [e2e_run_all.py](../research/2026-11-07-gpui-prototype/a11y-linux/e2e_run_all.py) did on Linux. One driver trait, two backends: UI Automation on Windows, AT-SPI on Linux. Tests keep their Playwright names. The rule that e2e windows open on a screen other than the main one carries over.
4. Visual checks Playwright did with screenshots (`look.spec.ts`, `screens.shots.ts`, `readme.shots.ts`) use GPUI's own capture of a headless window, or a screenshot of the release build for the README.
5. CI: `cargo fmt --check` and `cargo clippy -- -D warnings` join `checks`; `cargo test` joins `test`, on Windows and Linux; the AT-SPI e2e runs on Linux for PRs into `develop`, and the UI Automation e2e on Windows for PRs into `main`, as [ADR 0017](../decisions/0017-e2e-on-ci-only-for-prs-into-main.md) does today. Only GitHub-owned actions, pinned; Rust comes from the runners' `rustup`, with `rust-toolchain.toml`.

### R7. Performance

1. Each phase that adds to the app ends with `npm run bench` on the GPUI build, reported in the phase's PR. Until the cut-over the GPUI numbers are reported, not judged.
2. The bench learns to launch `Grace.exe` without Playwright: start, idle, run and size measures read processes and files, as today. The `smooth.*` measures come from a frame probe in the app, turned on by an environment variable and costing one check at start when off. Selecting, expanding and scrolling are driven through the accessibility tree or synthetic input. The `page.*` measures are retired at the cut-over (ADR 0031).
3. Every phase keeps ADR 0030: no wake-ups while minimised or hidden, and the shown idle within `idle.cpu.shown`. Grace's own code adds no timer, poll or animation while idle. A view that animates (the waiting stripe, the progress bar) stops when no run is going and when reduced motion is on.
4. At the cut-over, the release benchmark compares the GPUI release with the last Electron release, on the reference machine, with every limit of `bench/limits.json` except `page.*` (R10).

### R8. Accessibility

1. Every interactive element has a role and a name; every row of the tree and the HTTP list has its status, level, selected and expanded state; the progress bar has its value; the run summary is a status; the run's start and end are announced.
2. A test walks the accessibility tree of each view and fails on an interactive element without a name. It makes up for what the DOM gave for free.
3. The Windows half of [#269](https://github.com/hpagniez/grace/issues/269) (UI Automation, Narrator, Accessibility Insights) is done in phase P0, before any view is ported, so every view is built against a tree that works.
4. Known holes of the pinned AccessKit adapter (on Linux: a disabled button reads enabled; level and expanded do not reach AT-SPI) are listed in the phase issue and checked again at each GPUI upgrade.

### R9. While both apps live

1. The Electron app keeps shipping from `develop` until the cut-over. Releases follow the usual git flow.
2. A bug in Electron Grace is fixed there, and the fix lands in GPUI Grace too if the feature is already ported, with the same regression test.
3. A new feature waits for the cut-over, unless the owner says otherwise. If one lands in Electron Grace anyway, the same PR adds its spec scenarios and tests to the parity list.
4. The GPUI pins (`gpui-pre`, GPUI Kit, the toolchain) are upgraded only between phases, as one set, with the idle patch rebased and the bench run again.
5. Phase PRs follow the repository's rules: a `feature/*` branch, Conventional Commits with scope `gpui` or the area, `Part of #N` the phase issue, draft while working.

### R10. Cut-over

The cut-over is one release candidate: its zip holds GPUI `Grace.exe` instead of Electron's. It happens when all of these hold:

1. Every row of the parity list is ported or an `n/a` the owner accepted.
2. The full e2e suite passes on Windows, three times running, on CI and on the reference machine.
3. The release benchmark passes against the last Electron release, or misses with an analysis and the owner's `Decision: go` (AGENTS.md).
4. An update from the last Electron release to the cut-over build, and a rollback to it, work by hand on the reference machine, recorded in the check record.
5. Narrator reads the main flows (open, run, read a failure, the HTTP list, settings), recorded in the check record.

Then, on the release branch: the Electron sources, `forge.config.mts`, the Electron, Solid, Vite, Tailwind, Vitest and Playwright dependencies and the Playwright `e2e/` go; AGENTS.md's Commands, Stack, "Solid 2 rules" and UI conventions sections are rewritten for Rust and GPUI ("GPUI rules", with the traps the prototype found); README's build-from-source and badges change. ADR 0001, 0004 and 0005 stay as they are, superseded by ADR 0031.

## Feature map

Every feature issue that is done, plus what Grace does without an issue. Phases are in [Phases](#phases). "Proof" names the parity rows; each row names its test once ported.

### P0 Foundation and look

| Feature | Issue, spec | Today | GPUI Grace | Proof |
|---|---|---|---|---|
| Grace's look: tokens, both themes, IBM Plex, Lucide, status marks, buttons, focus ring, tooltip rules | [#107](https://github.com/hpagniez/grace/issues/107), [premium UI](2026-10-29-premium-ui.md) | `src/index.css`, `buttons.ts`, `ui.ts`, `StatusMark`, `Icon`, `Tooltip`, `Cut` | `grace::theme` (tokens from `index.css`, as the prototype's `look.rs`), `status_mark`, `icon`, `tooltip`, `cut`, button styles | PU-*, e2e `look.spec.ts` |
| Console that reads as a terminal | [#75](https://github.com/hpagniez/grace/issues/75), premium UI | `Console.tsx` | `grace::console`, IBM Plex Mono, colours from tokens | PU-*, `console.spec.ts` |
| A success notice that does not look like a warning | [#158](https://github.com/hpagniez/grace/issues/158) | `Notice.tsx` | `grace::notice` | PU-* |
| Bundled JRE, Karate and listener, and their weekly bumps | [#84](https://github.com/hpagniez/grace/issues/84), [bundled updates](2026-10-27-bundled-dependency-updates.md) | `scripts/vendor.json`, `fetch-vendor.mjs`, `vendor-updates` | Unchanged; packaging puts `vendor/` under `resources\` | BD-* |
| CI that stays fast | [#95](https://github.com/hpagniez/grace/issues/95), [faster CI](2026-10-28-faster-ci.md) | `test.yml` halves | Cargo caching, Linux for most jobs | FC-* |
| The light and fast guard | [#147](https://github.com/hpagniez/grace/issues/147), [light and fast](2026-10-31-light-and-fast.md) | `bench/`, `perf` | Bench launches `Grace.exe`, frame probe (R7) | LF-* |

### P1 Core: Karate, the run, the data

| Feature | Issue, spec | Today | GPUI Grace | Proof |
|---|---|---|---|---|
| Karate command, argument file, environment cleaning, exit codes | [walking skeleton](2026-10-24-walking-skeleton.md), [ADR 0008](../decisions/0008-secret-run-properties-and-java-argument-file.md) | `karate/command.ts`, `argfile.ts`, `process.ts` | `grace-core::karate` | unit tests of those files |
| Dry-run discovery and the test tree | [#52](https://github.com/hpagniez/grace/issues/52) | `karate/tree.ts`, `events.ts`, `run-state.ts` | `grace-core::karate::{tree, events, run_state}` | WS-01..03 |
| Live events, batched per tick | [#56](https://github.com/hpagniez/grace/issues/56), [#123](https://github.com/hpagniez/grace/issues/123) | `batch.ts`, `events.ts` | `grace-core::batch` | VL-*, `batch.test.ts` |
| Steps, logs, HTTP calls from step text | [#58](https://github.com/hpagniez/grace/issues/58), [#62](https://github.com/hpagniez/grace/issues/62) | `karate/steps.ts`, `http.ts` | `grace-core::karate::{steps, http}`; `npm run fixtures` still re-records | HI-*, `http.test.ts` |
| Java memory cap and JIT settings, below-normal priority | [#149](https://github.com/hpagniez/grace/issues/149), [#179](https://github.com/hpagniez/grace/issues/179), [#148](https://github.com/hpagniez/grace/issues/148) | `process.ts`, `command.ts` | `grace-core::karate::process` | JM-*, JJ-*, LF-* |
| Settings, recent folders, window bounds, appearance | several | `settings.ts`, `recent.ts`, `window.ts` | `grace-core::{settings, recent, window}`; same files | unit tests |
| Run history on disk, 20 runs | [#61](https://github.com/hpagniez/grace/issues/61), [ADR 0010](../decisions/0010-run-history-one-folder-per-run.md) | `history.ts`, `shared/history.ts` | `grace-core::history` | RH-* |
| Flaky detection and fingerprint | [#66](https://github.com/hpagniez/grace/issues/66) | `flaky.ts`, `fingerprint.ts` | `grace-core::{flaky, fingerprint}` | FL-* |
| Trends | [#93](https://github.com/hpagniez/grace/issues/93) | `trends.ts` | `grace-core::trends` | TR-* |
| Secrets, masking, properties file import | [#59](https://github.com/hpagniez/grace/issues/59), [#230](https://github.com/hpagniez/grace/issues/230) | `secrets.ts`, `mask.ts`, `propertiesFile.ts` | `grace-core::{secrets, mask, properties_file}`, DPAPI (R5.4) | EV-*, EP-* |
| Tags, environments, thread count logic | [#55](https://github.com/hpagniez/grace/issues/55), [#59](https://github.com/hpagniez/grace/issues/59), [#63](https://github.com/hpagniez/grace/issues/63) | `shared/tags.ts`, `env.ts`, `threads.ts`, `karate/environments.ts` | `grace-core::model::{tags, env, threads}` | TF-*, EV-*, TC-* |

### P2 Main screen

| Feature | Issue, spec | Today | GPUI Grace | Proof |
|---|---|---|---|---|
| Home screen, recent folders, first run, missing folder | [#78](https://github.com/hpagniez/grace/issues/78), [home](2026-10-27-home-screen.md) | `Home.tsx`, `EmptyState.tsx` | `grace::home` | HS-*, `home.spec.ts` |
| Open…, starting in the last folder | [#82](https://github.com/hpagniez/grace/issues/82) | `main.ts` dialog | dialog seam | UP-*, `open.spec.ts` |
| Toolbar readable with a long path | [#81](https://github.com/hpagniez/grace/issues/81), [toolbar](2026-11-02-toolbar.md) | `Toolbar.tsx` | `grace::toolbar` | TB-* |
| Test tree, folders open first, keyboard, 10,000 tests | [#52](https://github.com/hpagniez/grace/issues/52), [#91](https://github.com/hpagniez/grace/issues/91), [#125](https://github.com/hpagniez/grace/issues/125), [#224](https://github.com/hpagniez/grace/issues/224) | `Tree.tsx`, `treeRows.ts`, `nodes.ts`, `VirtualList` | `grace::tree` on a virtual list | VL-*, TO-*, `virtual.spec.ts` |
| Run anything, cancel anything | [#53](https://github.com/hpagniez/grace/issues/53), [#54](https://github.com/hpagniez/grace/issues/54), [#83](https://github.com/hpagniez/grace/issues/83) | `runner.ts`, `Toolbar.tsx` | `Grace::run`, `cancel` | WS-*, `run.spec.ts`, `cancel.spec.ts` |
| Tag filter, one-line filter bar | [#55](https://github.com/hpagniez/grace/issues/55), [#127](https://github.com/hpagniez/grace/issues/127) | `FilterBar.tsx` | `grace::filter_bar` | TF-*, CF-*, `tags.spec.ts` |
| Environment picker, run properties, secrets, env vars | [#59](https://github.com/hpagniez/grace/issues/59), [#230](https://github.com/hpagniez/grace/issues/230) | `EnvPicker.tsx`, `PropertiesDialog.tsx`, `ImportStep.tsx` | `grace::env_picker`, `grace::properties` | EV-*, EP-*, `env.spec.ts` |
| Thread count | [#63](https://github.com/hpagniez/grace/issues/63) | `Threads.tsx` | GPUI Kit number field | TC-*, `threads.spec.ts` |
| Live progress, status bar, loading always visible | [#56](https://github.com/hpagniez/grace/issues/56), [#80](https://github.com/hpagniez/grace/issues/80) | `Progress.tsx`, `StatusBar.tsx` | `grace::progress`, `grace::status_bar` | WS-*, LV-* |
| Failure details, steps, variables | [#58](https://github.com/hpagniez/grace/issues/58) | `Details.tsx`, `Variables.tsx` | `grace::details` | WS-06, `details.spec.ts` |
| Console, banner, colours, find, step through errors | [#57](https://github.com/hpagniez/grace/issues/57), [#73](https://github.com/hpagniez/grace/issues/73), [#75](https://github.com/hpagniez/grace/issues/75) | `Console.tsx`, `Banner.tsx`, `bannerText.ts`, `consoleFind.ts` | `grace::console` | CB-*, CS-*, `console*.spec.ts` |
| Bottom panel and splitter | [#88](https://github.com/hpagniez/grace/issues/88) | `BottomPanel.tsx`, `Splitter.tsx`, `panelHeight.ts` | `grace::bottom_panel` | UP-*, `panel.spec.ts` |
| Window: bounds, screens, single instance, no flash, theme | [#90](https://github.com/hpagniez/grace/issues/90), [#92](https://github.com/hpagniez/grace/issues/92) | `main.ts`, `window.ts` | `grace::window` | WB-*, `window.spec.ts`, `startup.spec.ts` |
| Settings: appearance | [#100](https://github.com/hpagniez/grace/issues/100) (open, partly built) | `Settings.tsx` | `grace::settings` | `appearance.spec.ts` |

### P3 Panels

| Feature | Issue, spec | Today | GPUI Grace | Proof |
|---|---|---|---|---|
| HTTP inspector, Copy as cURL, cheap during a run | [#62](https://github.com/hpagniez/grace/issues/62), [#124](https://github.com/hpagniez/grace/issues/124) | `HttpCalls.tsx`, `CallDetails.tsx`, `http.ts` | `grace::http` | HI-*, `http.spec.ts` |
| Run history, re-run, re-run failed, clear | [#61](https://github.com/hpagniez/grace/issues/61) | `History.tsx` | `grace::history` | RH-*, `history.spec.ts` |
| Trends | [#93](https://github.com/hpagniez/grace/issues/93) | `Trends.tsx`, `trendLabels.ts` | `grace::trends` | TR-*, `trends.spec.ts` |
| Flaky marks and proof | [#66](https://github.com/hpagniez/grace/issues/66) | `FlakyProof.tsx` | `grace::flaky_proof` | FL-*, `flaky.spec.ts` |
| Open the HTML report, export a run's report | [#60](https://github.com/hpagniez/grace/issues/60), [#70](https://github.com/hpagniez/grace/issues/70) | `report-zip.ts`, `main.ts` | open seam, `grace-core::report_zip` | WS-*, ER-*, `report.spec.ts` |
| Data-driven view | [#71](https://github.com/hpagniez/grace/issues/71) | `Examples.tsx` | `grace::examples` | DD-*, `examples.spec.ts` |
| Config failure said once, failures grouped | [#229](https://github.com/hpagniez/grace/issues/229) | `ConfigBanner.tsx`, `failures.ts` | `grace::failures` | GF-*, `failures.spec.ts` |
| Step comments | [#243](https://github.com/hpagniez/grace/issues/243) | `Details.tsx` | `grace::details` | SC-* |
| Open in VS Code, from details and the tree | [#68](https://github.com/hpagniez/grace/issues/68), [#222](https://github.com/hpagniez/grace/issues/222) | `editor.ts` | `grace-core::editor`, context menu | OE-*, OT-*, `editor.spec.ts` |
| Reorder the panel tabs | [#94](https://github.com/hpagniez/grace/issues/94) | `panelTabs.ts` | drag and drop | RP-* |

### P4 Project setup, watch, debugger

| Feature | Issue, spec | Today | GPUI Grace | Proof |
|---|---|---|---|---|
| Maven, Gradle, standalone and `karate-pom.json` projects; libraries from the build | [#64](https://github.com/hpagniez/grace/issues/64), [libraries](2026-11-02-project-libraries.md), [setup](2026-11-02-project-setup.md) | `karate/maven.ts`, `libraries.ts`, `compiled.ts`, `pom.ts` | `grace-core::karate::*` | PL-*, PS-*, `project-setup.spec.ts` |
| Find Maven and Gradle off PATH | [#231](https://github.com/hpagniez/grace/issues/231) | `libraries.ts` | same, per platform (R5.3) | FB-* |
| Start from the pom's defaults | [#228](https://github.com/hpagniez/grace/issues/228) | `karate/pom.ts` | `grace-core::karate::pom` | PD-* |
| Offer the sub-folder with the project | [#232](https://github.com/hpagniez/grace/issues/232) | `karate/subfolders.ts` | `grace-core::karate::subfolders` | SO-* |
| Build from the not-built notice | [#233](https://github.com/hpagniez/grace/issues/233) | `runner.ts` | `Grace::build` | BN-* |
| Watch mode | [#67](https://github.com/hpagniez/grace/issues/67) | `watch.ts` | `grace-core::watch` (`notify`) | WM-*, `watch.spec.ts` |
| Step debugger: breakpoints, pause, step, variables | [#65](https://github.com/hpagniez/grace/issues/65), [debugger](2026-11-02-step-debugger.md) | `karate/debug.ts`, `listener/grace/DebugListener.java`, `Variables.tsx` | `grace-core::karate::debug` (same socket protocol, same listener jar), `grace::debugger` | SD-*, `debug.spec.ts` |

### P5 Updates and release

| Feature | Issue, spec | Today | GPUI Grace | Proof |
|---|---|---|---|---|
| Check, update, auto-update, skip, roll back, install any version, back up data | [#77](https://github.com/hpagniez/grace/issues/77), [app updates](2026-10-28-app-updates.md), [ADR 0019](../decisions/0019-update-the-zip-install-from-github-releases.md) | `updates/*`, `switch.ps1`, `UpdateModal`, `UpdateProgress`, `SwitchDialog`, `Settings` | `grace-core::updates` (HTTP with `ureq` or the platform's, SHA-256, `tar.exe` to unpack as today), the same `switch.ps1`, `grace::updates` | AU-*, `updates.spec.ts` |
| The zip and its contract | ADR 0019 | `forge.config.mts`, `app-files.json` | A Node packaging script: `Grace.exe` at the root, `resources\` with `jre\`, `karate.jar`, `listener.jar`, `licenses\`, `switch.ps1`, `app-files.json` | AU-*, a by-hand Electron → GPUI update (R10.4) |
| Signing | [#176](https://github.com/hpagniez/grace/issues/176), [ADR 0024](../decisions/0024-sign-grace-exe-with-signpath-foundation.md) | `release.yml` | Same step, on the Rust exe | release run |

### P6 Cut-over

| Item | Today | GPUI Grace | Proof |
|---|---|---|---|
| Only en-US ([#150](https://github.com/hpagniez/grace/issues/150), [spec](2026-11-02-en-us-locale.md)) | Forge hook deletes other `.pak` files | Nothing to delete; English only | EN-*, mostly `n/a` |
| The release benchmark, the parity list, the check record | — | R10 | R10 |

### Regressions to keep fixed

Each fixed bug that describes behaviour the port could break again. Each gets a test in the GPUI app, listed with its phase:

| Bug | What must hold | Phase |
|---|---|---|
| [#119](https://github.com/hpagniez/grace/issues/119) | A failed update download leaves no empty file | P5 |
| [#131](https://github.com/hpagniez/grace/issues/131) | Updating does not hang at Unpacking in the packaged app | P5 |
| [#144](https://github.com/hpagniez/grace/issues/144) | The progress bar's passed part fills during a big run | P2 |
| [#153](https://github.com/hpagniez/grace/issues/153) | The tree's focus ring starts at the row's indent | P2 |
| [#160](https://github.com/hpagniez/grace/issues/160) | The switch script survives a process exiting while it lists Grace's | P5 |
| [#170](https://github.com/hpagniez/grace/issues/170) | Nothing blinks or animates while idle (the console cursor did) | P2 |
| [#208](https://github.com/hpagniez/grace/issues/208), [#223](https://github.com/hpagniez/grace/issues/223) | Running one test of a Maven or Gradle project runs it and shows it in the tree, Details and the top bar | P2 |
| [#219](https://github.com/hpagniez/grace/issues/219), [#220](https://github.com/hpagniez/grace/issues/220) | Open in VS Code works for a share or mapped drive; a long wrapped error place is not centred | P3 |
| [#236](https://github.com/hpagniez/grace/issues/236) | A project whose only Java code is its JUnit runner is not said to be unbuilt | P4 |
| [#238](https://github.com/hpagniez/grace/issues/238) | The updater unpacks under a folder name outside the ANSI code page | P5 |
| [#263](https://github.com/hpagniez/grace/issues/263) | The waiting stripe does not slide over the last run's results | P2 |
| [#261](https://github.com/hpagniez/grace/issues/261) (open) | Run memory and run frames within their limits; the GPUI build is the fix | P6 |

### Not ported, because not built

Open feature issues stay open and are built after the cut-over, on GPUI: [#64](https://github.com/hpagniez/grace/issues/64)'s remaining parts, [#69](https://github.com/hpagniez/grace/issues/69), [#72](https://github.com/hpagniez/grace/issues/72), [#74](https://github.com/hpagniez/grace/issues/74), [#76](https://github.com/hpagniez/grace/issues/76) (a custom title bar is easier in GPUI), [#79](https://github.com/hpagniez/grace/issues/79) (GPUI Kit has a code editor), [#85](https://github.com/hpagniez/grace/issues/85), [#86](https://github.com/hpagniez/grace/issues/86), [#100](https://github.com/hpagniez/grace/issues/100)'s remaining parts, [#126](https://github.com/hpagniez/grace/issues/126), [#174](https://github.com/hpagniez/grace/issues/174), [#180](https://github.com/hpagniez/grace/issues/180), [#181](https://github.com/hpagniez/grace/issues/181), [#198](https://github.com/hpagniez/grace/issues/198), [#234](https://github.com/hpagniez/grace/issues/234), [#235](https://github.com/hpagniez/grace/issues/235).

## Phases

Each phase has an issue, a plan in `docs/plans/` written just before it starts, and an exit check. A phase ends when its parity rows are ticked and its exit check passes. Phases run in order; P3 and P4 may overlap once P2 is done.

| Phase | Builds | Exit check |
|---|---|---|
| **P0 Foundation** ([#279](https://github.com/hpagniez/grace/issues/279)) | The workspace and pins; the idle patch under `[patch.crates-io]`; `rust-toolchain.toml`; the theme, fonts, icons and shared elements; the accessibility base and the tree-walking test (R8.2); the e2e driver on UI Automation and AT-SPI with a first test; packaging into the release zip layout; CI (R6.5); the bench launching `Grace.exe` (R7.2); AGENTS.md's first "GPUI rules" | An empty Grace window passes ADR 0030 on the reference machine (minimised and shown) and Narrator reads it; the zip has the contract's layout; CI is green on both platforms; the Windows half of #269 is closed |
| **P1 Core** ([#280](https://github.com/hpagniez/grace/issues/280)) | `grace-core`: every module in the P1 map, with its unit tests ported | Every `src/main` and `src/shared` unit test row ported or `n/a`; a headless run of the 2,000-test bench project from `grace-core` alone matches Electron Grace's `run.json` |
| **P2 Main screen** ([#281](https://github.com/hpagniez/grace/issues/281)) | The window, home screen, toolbar, tree, filter bar, environments and properties, threads, run and cancel, progress, details, console, bottom panel, appearance | P2 parity rows ticked; bench on the GPUI build within every limit it shares with Electron Grace |
| **P3 Panels** ([#282](https://github.com/hpagniez/grace/issues/282)) | HTTP, history, trends, flaky, reports, data-driven view, failures, step comments, editor, tab order | P3 parity rows ticked |
| **P4 Project setup, watch, debugger** ([#283](https://github.com/hpagniez/grace/issues/283)) | Maven, Gradle, pom, libraries, sub-folders, build, watch, the step debugger | P4 parity rows ticked; the debugger works with the unchanged listener jar |
| **P5 Updates and release** ([#284](https://github.com/hpagniez/grace/issues/284)) | The updater, the zip, signing, the release workflow building the Rust exe | P5 parity rows ticked; Electron → GPUI → Electron update and rollback by hand on a test build |
| **P6 Cut-over** ([#285](https://github.com/hpagniez/grace/issues/285)) | R10 | R10's five conditions, then the release |

## Cost

What the move changes, by ADR 0023's priorities, from the prototype's numbers on the reference machine; the port adds features the prototype lacked, so each phase re-measures (R7.1):

- **Memory.** Idle from 167 MB to about 28 MB plus what the remaining features hold; during a 10,000-test run, the app from 671 MB to about 40 MB. Java's share does not change.
- **CPU.** During runs, the app's CPU drops to about a tenth. Idle shown stays within `idle.cpu.shown` (422 ms a minute measured, limit 1,453 ms); minimised, 0 to 47 ms a minute with the idle patch, limit 125 ms.
- **Start.** First paint from about 500 ms to about 325 ms.
- **Size.** Unpacked from 489 MB to about 192 MB with Java; the zip shrinks accordingly.
- **Idle rule.** No timer, poll or animation while idle, in Grace's code (R7.3). GPUI's shown refresh is the engine's housekeeping, inside the budget; the patch removes the minimised one.
- **CI.** A Rust build of GPUI is several minutes cold; `actions/cache` keeps `target/` and the Cargo registry. `perf` builds two trees once it judges the GPUI build (P6).
- **People.** About 14,500 lines of TypeScript and 1,967 tests to port while Electron Grace keeps shipping. That cost is the reason for the phases and the freeze of R9.3.

## Design decisions already made

- Rust and GPUI through GPUI Kit, pinned as one set, the Windows idle patch carried in the repository (ADR 0031).
- The updater contract, the data folder and the data formats stay (ADR 0031, R1.4). Secret values do not carry over (R5.4).
- Windows ships; Linux builds for development (R5).
- Node stays for tooling only.
- One process: no IPC layer to rebuild.
- Tests: `cargo test`, headless GPUI views, e2e through the accessibility tree.
- The parity list is generated once from `develop` at `32a5ef6` and kept by hand afterwards (R9.3).

## Left to the builder

- Crate choices inside `grace-core` (HTTP client, JSON, file watching, Windows API bindings): the smallest that does the job, named with its size in its PR.
- How the e2e driver waits for the tree to settle.
- Where the frame probe hooks into GPUI's frame loop.
- The exact module layout under `crates/grace`.

## Open questions

- **The wake-up gate (owner).** The Windows runbook's gate of under 5 wake-ups a second while minimised cannot be met by any D3D11 app on an NVIDIA driver: its driver thread wakes about 61 times a second, and Grace's GPU process has the same thread at 92. Drop the gate, or count only Grace's own threads? ADR 0030's `idle.cpu.minimised` limit applies either way. P0's exit check waits for this answer.
- **Other GPUs (P0).** Only NVIDIA was measured. P0 checks an Intel or AMD GPU too, and the minimised idle with the patch on it.
- **Untested platform cases (P0).** A GPU reset, windows across screens with different refresh rates, showing a window again after hiding it.
- **Upstream (P0 and each upgrade).** Whether zed#66221 or zed#66449 lands, so the patch can go. Grace follows them from its own issues and posts nothing on Zed's repository.

## Acceptance criteria

```gherkin
Feature: Port Grace to Rust and GPUI

  Scenario: Every feature is listed
    Given the parity checklist generated from develop at the go decision
    Then it lists every Scenario of every spec, every e2e test and every Vitest file of src

  Scenario: A ported row names its proof
    Given a PR that ports a feature
    Then each parity row it ticks names the test that proves it
    And that test passes on CI

  Scenario: A row that no longer applies
    Given a parity row that tests Electron, the DOM or Solid itself
    When it is marked n/a with its reason
    Then the owner accepts it before the cut-over

  Scenario: Idle while minimised
    Given GPUI Grace on the reference machine with a project open and no run going
    When the window is minimised for 60 s
    Then idle.cpu.minimised is within its limit

  Scenario: Idle while shown
    Given GPUI Grace on the reference machine with a project open and no run going
    When the window is shown for 60 s
    Then idle.cpu.shown is within its limit

  Scenario: Every control has a name
    Given any view of GPUI Grace
    When the accessibility tree is walked
    Then every interactive element has a role and a name

  Scenario: Update from the last Electron release
    Given the last Electron release installed with settings and run history
    When it updates to the cut-over release
    Then GPUI Grace starts, reports the update as done, and shows the same settings and run history

  Scenario: Roll back to the last Electron release
    Given the cut-over release installed
    When the person rolls back to the last Electron release
    Then Electron Grace starts with the data saved for it

  Scenario: The same Karate command
    Given the same project, selection, environment, tags, properties and threads
    Then GPUI Grace and Electron Grace run Karate with the same arguments and argument file

  Scenario: Cut-over
    Given every parity row ported or accepted as n/a
    And the e2e suite green three times running on Windows
    And the release benchmark passed or decided go by the owner
    Then the release zip holds GPUI Grace.exe and the Electron sources are removed
```
