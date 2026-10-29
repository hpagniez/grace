# Tauri spike: Tauri 2 as a replacement for Electron (#106)

Date 2026-10-29. Windows 11 Pro 10.0.26300, AMD Ryzen 7 5800X (8 cores), 32 GB, Node 24.15.0, WebView2 Runtime 156.0.4314.50. Electron baseline: Grace 1.0.0-rc.3 on Electron 45.0.0, built with `npm run make` from `develop` at cee6dc98.

The spike is a reference only: `node_modules`, `dist`, `src-tauri/target`, the sidecar build and the lockfiles are not kept.

## Verdict: no-go

Stay on Electron. Tauri 2 makes Grace much smaller, but it brings no gain in memory or start time on Windows 11. To get the size gain, the main process would have to be rewritten in Rust.

- **Size is the only gain.** The zip goes from 218.6 MB to 62.5 MB (−71%) with a Rust backend, or to 97.8 MB (−55%) if `src/main` stays in Node as a sidecar.
- **No gain in memory or start time.** Idle memory is 8–10 MB higher in private working set, because WebView2 runs 6 processes. First paint is 124–134 ms slower. The project tree shows 28–57 ms sooner, but only after a startup change that took a fix in the spike.
- **The cost is a rewrite.**
  - About 2,900 lines of `src/main` and 5,900 lines of its tests would move to Rust.
  - `src/shared` would split across two languages.
  - All 18 e2e spec files would need new test seams.
  - CI would need a Rust toolchain.
  - A second language would add review load in a repo built by AI agents.
  - The cheaper backends lose most of the gain (Node sidecar) or the trust boundary (plugins from the webview).

Reopen the question if download size becomes a reported problem. A cheaper size cut on Electron is still open: ship only the `en-US` locale, about −12.6 MB zipped and −50.6 MB unpacked. It is not tested.

No decision record: a go would have needed one that supersedes ADR 0001 and ADR 0004. A no-go keeps both.

## What was built

The real Grace renderer (`src/renderer`, unchanged) runs in a Tauri 2 window. Only `window.api` changes: [src/api.ts](src/api.ts) implements the typed `Api` of `src/shared/ipc.ts` with Tauri's `invoke` and `listen`. One Rust shell ([src-tauri/src/main.rs](src-tauri/src/main.rs)) serves two backends:

| Backend | Code | Thin path |
|---|---|---|
| Option 1: Rust port | [src-tauri/src/karate.rs](src-tauri/src/karate.rs), 560 lines | Open (dry run, tree), run with live progress from `karate-events.jsonl`, cancel with `taskkill /PID <pid> /T /F`. Keeps the spawn rules of AGENTS.md: no shell, no console window, the four option variables removed, `-cp "karate.jar;<roots>"`. |
| Option 2: Node sidecar | [sidecar/main.ts](sidecar/main.ts) runs `src/main/runner.ts` unchanged as a Node single executable application; [src-tauri/src/sidecar.rs](src-tauri/src/sidecar.rs) pipes JSON lines | All of Grace's `Api`, since it is Grace's own Runner (secret properties off: no `safeStorage` in Node). Selected with `SPIKE_BACKEND=node`. |
| Option 3: plugins from the webview | [src-tauri/tauri.plugins.conf.json](src-tauri/tauri.plugins.conf.json), `--features plugins` | Not built as a backend. A probe checks what a shell-plugin capability scoped to `java.exe` lets the page run. |

The page holds one capability ([capabilities/default.json](src-tauri/capabilities/default.json)): call `api`, listen to events, nothing else. The folder picker and report opening run in Rust (`tauri-plugin-dialog`, `tauri-plugin-opener`), as they run in Electron's main process today.

Run it (after `npm install` here and `npm run vendor` at the repo root):

```bash
npx tauri build --no-bundle            # src-tauri/target/release/Grace.exe, resources copied beside it
npm run sidecar                        # option 2: builds src-tauri/binaries/grace-sidecar-*.exe, then
npx tauri build --no-bundle --config src-tauri/tauri.sidecar.conf.json   # copies it beside Grace.exe
npx tauri build --no-bundle --features plugins --config src-tauri/tauri.plugins.conf.json   # option 3 probe
SPIKE_EXE=src-tauri/target/release/Grace.exe npx playwright test -c e2e
node measure/measure.mjs <exe> <electron|tauri> <project folder>
```

`SPIKE_PROJECT` opens a folder at start, like Grace's last project; `SPIKE_DATA` replaces the data folder.

## Measured

Every number comes from [measure/measure.mjs](measure/measure.mjs), [measure/roundtrip.mjs](measure/roundtrip.mjs) or the zip files on this machine.

- **Timing runs:**
  - The app reopens the `clean` fixture at start (Grace's last project, the spike's `SPIKE_PROJECT`).
  - Playwright attaches over CDP: Electron's `--remote-debugging-port`, or `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` for WebView2.
  - First paint is the page's `first-contentful-paint`, measured from process spawn.
  - Tree is the first time `[data-testid="tree"]` is found, polled every 20 ms.
- **Memory runs:**
  - No debugging port; the app sits idle 20 s with the project open.
  - Memory is summed over the app's whole process tree, Java excluded (none was running).
  - Private working set is Task Manager's Memory column.
- **Medians:** of 5 runs after one warm-up run.
- **Profile:**
  - A *fresh profile* starts each run on a new data folder.
  - A *reused profile* keeps one data folder for all runs, as on a second start.

### Size

| | Electron (Forge zip) | Tauri, Rust backend | Tauri, Node sidecar |
|---|---|---|---|
| Portable zip | 218.6 MB | **62.5 MB** | 97.8 MB |
| of which shell (zipped) | 158.1 MB | 1.5 MB (`Grace.exe`, 3.5 MB unzipped) | 36.9 MB (`Grace.exe` + 91.9 MB sidecar) |
| of which JRE, Karate jar, licenses (zipped) | 60.3 MB | 60.9 MB | 60.9 MB |
| Unpacked (= installed) | 538.5 MB | **167.8 MB** | 259.7 MB |
| NSIS installer | — | 42.0 MB (LZMA) | not built |
| MSI | — | fails on version `1.0.0-rc.3` | — |

Electron's zip share by file:

| File | Zipped | Unpacked |
|---|---|---|
| `Grace.exe` | 108.7 MB | 249.8 MB |
| `resources.pak` | 13.3 MB | 13.4 MB |
| `locales/` (55 files; `en-US.pak` alone is 0.6 MB) | 12.8 MB | 51.2 MB |
| `dxcompiler.dll` | 10.2 MB | 25.8 MB |
| `icudtl.dat` | 4.6 MB | 10.8 MB |

### Memory, start time, IPC

| | Electron | Tauri, Rust | Tauri, Node sidecar |
|---|---|---|---|
| First paint, fresh / reused profile | **437 / 414 ms** | 571 / 538 ms | — / 535 ms |
| Tree shown, fresh / reused profile | 767 / 741 ms | **710 / 713 ms** | — / 758 ms |
| Idle private working set, fresh / reused | **87.8 / 84.8 MB** | 95.5 / 94.5 MB | — / 109.6 MB |
| Idle working set, fresh / reused | **333.8 / 327.1 MB** | 372.7 / 376.8 MB | — / 418.5 MB |
| Processes | 4 | 7 (`Grace.exe` 4 MB + 6 `msedgewebview2.exe`) | 8 (+ sidecar 17 MB) |
| No-op call from the page, median / p95 (500 calls) | **0.10 / 0.20 ms** | 1.10 / 1.50 ms | 1.20 / 1.80 ms |

- **Window before `setup`.** Tauri builds the windows of `tauri.conf.json` before the `setup` hook runs (`tauri` 2.12.5 `src/app.rs`). The first Tauri build started the dry run after the WebView2 window was built.
  - With that order, Karate wrote its first event about 790 ms after spawn (Electron: about 540 ms), and the tree showed at 961 ms (fresh profile) and 969 ms (reused).
  - The spike now sets `"create": false` and builds the window in `setup` after starting the dry run. The first event then lands at about 445 ms, which gives the 710 ms above.
  - Grace would need the same order.
- **Window shows before first paint.** Tauri's window shows at once, with WebView2's white background, before the page paints. Electron's Grace stays hidden until `ready-to-show`.
  - First paint compares the same page event in both, but in Tauri the person sees a white window first.
  - The fix is `visible: false` plus a show on page load, or a background colour chosen per theme.
- **IPC cost.** Tauri's `invoke` goes through a custom protocol request, about 1 ms. Grace batches run events every 100 ms, so this does not matter.

## The three backends

| | 1. Rust port | 2. Node sidecar | 3. Plugins from the webview |
|---|---|---|---|
| Zip | 62.5 MB | 97.8 MB | about 62.5 MB |
| Idle private memory | +8–10 MB vs Electron | +22–25 MB | about as option 1 |
| `src/main` | rewritten in Rust (about 2,900 lines, 5,900 lines of tests) | kept as is; `main.ts`, `preload.ts` and `secrets.ts` change | rewritten in page TypeScript against plugin APIs (no `node:fs`, no `child_process`) |
| `src/shared` | split: logic main uses (tags, env, history, threads, flaky) in Rust and TypeScript | one home | one home |
| Trust boundary | kept | kept | **lost** |
| Rust to own | all of main | a shell of about 250 lines | the shell and capability files |

**Option 2** ran Grace's real `Runner` as a 91.9 MB Node single executable application, both alone and inside Tauri:
- Open, live progress, cancel (outcome `cancelled`, no Java left) and the request validation of `src/main.ts` all work.
- **Signing.** The injection breaks node.exe's Authenticode signature (`postject`: "The signature seems corrupted!"), so a release would have to re-sign the sidecar.
- **Secrets.** `safeStorage` does not exist in Node: secrets would have to be encrypted in Rust and passed over the pipe.
- **Size and memory.** The size gain shrinks by a Node runtime (35.3 MB zipped), and memory is the worst of the three.

**Option 3** was probed with [e2e/shell-scope.mjs](e2e/shell-scope.mjs). It calls `plugin:shell|execute` from the page, as any script there could. The capability allows the bundled `java.exe` with Karate's exact argument shape (validators on the paths), and `taskkill /PID <n> /T /F`:

| Call from the page | Result |
|---|---|
| Karate dry run of the open project | allowed, exit 0 |
| Same shape, `-w` any folder, `-cp C:\Users\Public\karate.jar;C:\Users\Public` | **allowed** (exit 1: no such jar) |
| `java -version` | denied: `program not allowed on the configured shell scope: karate` |
| `cmd /c echo hi` | denied |
| `taskkill` of any PID | passed the scope; the call then failed decoding taskkill's localized output as UTF-8 |

A scope can fix the argument shape but not which folder or jar is meant. Running Karate in a folder runs that folder's code (`karate-config.js`, Java classes), so a script in the page could run any code it can place on disk. Today the renderer cannot name a folder at all: `project:open` shows the native picker in main, and `recent:open` takes only a folder from the saved list. Option 3 loses that boundary.

## Electron API map

| Electron, where used | Tauri 2 |
|---|---|
| `app.requestSingleInstanceLock`, `second-instance` (`main.ts`) | `tauri-plugin-single-instance`. It locks per app identifier, not per data folder, so e2e tests that start several apps at once need a seam. |
| `app.setPath/getPath("userData")`, `GRACE_USER_DATA` | `app.path().app_data_dir()`, read-only. The spike uses its own variable (`SPIKE_DATA`). |
| `app.isPackaged`, `process.resourcesPath` | `tauri::is_dev()`, `app.path().resource_dir()`. It returns a `\\?\D:\…` path, which Java's `-cp` cannot read: strip it (`dunce` crate). |
| `app.on("before-quit")` + `runner.dispose()` | `RunEvent::ExitRequested` (`api.prevent_exit()`) or `RunEvent::Exit` |
| `BrowserWindow({ show: false })` + `ready-to-show` | `"visible": false`, then `show()` on `PageLoadEvent::Finished` |
| `backgroundColor` by `nativeTheme.shouldUseDarkColors` | `WebviewWindowBuilder::background_color`, `Window::theme()` |
| `getBounds/setBounds/getNormalBounds/isMaximized`, `moved`/`resized` (`window.ts`, settings) | `outer_position`, `inner_size`, `set_position`, `set_size`, `is_maximized`, `WindowEvent::Moved/Resized`. No normal-bounds getter: `tauri-plugin-window-state` keeps it. |
| `screen.getAllDisplays()[].workArea` (`placeWindow`) | `app.available_monitors()`, `Monitor::work_area()` |
| `webPreferences: contextIsolation, nodeIntegration: false, sandbox` | Nothing to set: WebView2 has no Node, and its renderers run in Chromium's sandbox. The page reaches Rust only through commands its capabilities allow. |
| `setWindowOpenHandler(deny)`, `will-navigate` | `WebviewWindowBuilder::on_new_window`, `on_navigation` |
| `Menu.setApplicationMenu(null)` (no Reload) | No menu on Windows, but F5, Ctrl+R and the right-click menu still reload. wry has `browser_accelerator_keys` and `default_context_menus` switches, but Tauri 2.12.5 never passes them. Block the keys in the page, or rely on `replay` after a reload. |
| `ipcMain.handle` per `CH` channel | `#[tauri::command]`. The spike uses one `api(channel, args)` command, to keep `CH` as the only list of channels. |
| `webContents.send(CH.event, e)` | `Emitter::emit_to("main", "app:event", e)` |
| `contextBridge.exposeInMainWorld("api", api)` (`preload.ts`) | None. The page imports `@tauri-apps/api` ([src/api.ts](src/api.ts)). The isolation pattern can filter IPC in an iframe if wanted. |
| `dialog.showOpenDialog(win, { openDirectory, defaultPath })` | `tauri-plugin-dialog`: `app.dialog().file().set_parent(&w).set_directory(p).blocking_pick_folder()` (from Rust) |
| `shell.openPath(report)` | `tauri-plugin-opener`: `app.opener().open_path(p, None)` (from Rust) |
| `safeStorage` (secrets, ADR 0008) | Not in Tauri. Windows DPAPI (`CryptProtectData`, `windows` crate) is the closest to Electron's, or the `keyring` crate. Secrets saved by Electron would be lost. |
| Fuses (`RunAsNode`, asar integrity, …) | None: there is no Node and no asar. |

## Typed API and `src/shared`

- **Option 2: types survive.** The `Api` interface stays the contract. [src/api.ts](src/api.ts) implements it with `invoke`/`listen` and typechecks against `src/shared/ipc.ts` (`npm run typecheck` here). `src/shared` keeps one home.
- **Option 1: the contract is not checked.** The Rust side builds events with `json!`, and nothing checks them against `src/shared/events.ts`. The spike shows the cost:
  - The Rust `replay` left out the `opening` event, which the renderer waits for before it drops the startup screen.
  - With no project open, the page stayed on "Loading Grace".
  - It compiled cleanly and was found only by an e2e test.
- **Option 1 fix.** Define the events and commands as Rust types and generate TypeScript from them (`tauri-specta`, `ts-rs`). The logic main shares with the renderer (tag filters, environment and property parsing, run ids, thread limits, flaky detection) would then exist twice.
- **`listen()` race.** `listen()` registers asynchronously, so `ready()` must wait for it. Otherwise main's replay can reach the page before the listener exists. The shim chains them.

## Tests

| Driver | Result |
|---|---|
| Playwright over CDP ([e2e/cdp.spec.mjs](e2e/cdp.spec.mjs)) | 3 passed: home with no project, run with live progress (`9 / 9`, Finished), cancel (Cancelled) with no Java left from the app's `jre`. Also passes from the unzipped portable build. |
| tauri-driver 2.1.0 + msedgedriver 156.0.4314.50 ([e2e/webdriver.mjs](e2e/webdriver.mjs)) | Passed: session in about 0.5 s, run to Finished. Plain `fetch` on W3C WebDriver; no WebdriverIO or Selenium. msedgedriver must match the WebView2 version exactly. |
| `cargo test` | 2 passed (tree from the recorded `dryrun-clean.jsonl`, run id shape) |

- **Main-process stubs.** Playwright's `_electron` does not drive Tauri, and neither CDP nor WebDriver reaches the Rust side. All 18 spec files in `e2e/` use `app.evaluate` or helpers built on it (62 uses), to stub the folder picker and `shell.openPath` and to read window state. Each needs a test seam in the app instead, such as an environment variable that answers the picker.
- **Leak checks.** The Java leak checks carry over unchanged: they look at `java.exe` under the app's `jre`.
- **CDP is the cheaper path.** It keeps Playwright and the existing locators. WebDriver adds tauri-driver and a matching msedgedriver on every machine.
- **WebView2 updates under the tests.** WebView2 is the system's evergreen runtime: the rendering engine updates without a Grace release, and the test driver has to follow it.

## Packaging and CI

- **No zip target.** Tauri's Windows bundler makes only NSIS and MSI installers.
  - A portable zip means `tauri build --no-bundle` plus a script that zips `Grace.exe`, `jre/`, `karate.jar` and `licenses/`.
  - `resource_dir()` is the exe's folder, and the zip ran from another folder (e2e above).
  - The JRE is a `bundle.resources` entry; `tauri build` copies it beside the exe.
- **MSI rejects the version.** The bundler refuses `1.0.0-rc.3` for MSI: "optional pre-release identifier in app version must be numeric-only". ADR 0015's versions would need a separate MSI version.
- **Tools downloaded at build time.** The bundler downloads NSIS 3.13, `nsis_tauri_utils.dll` and WiX 3.14 from GitHub at build time (hash-checked).
- **Build times** (8 cores; GitHub's Windows runners have 4 vCPUs):

  | Build | Time |
  |---|---|
  | Cold release build (LTO, `opt-level = "s"`) | 131 s |
  | Rebuild after a one-file change | 75 s |
  | `cargo test --release` | 57 s |
  | NSIS bundle on top | about 60 s, with the first download of NSIS and WiX |

- **Cache.** `target/` is 1.4 GB, and the cargo registry 1.0 GB. The Windows dependency tree has 269 crates (468 in `Cargo.lock`, all platforms).
- **Third-party actions not allowed.** Workflows may use only GitHub-owned actions (ADR 0007), so `dtolnay/rust-toolchain` and `Swatinem/rust-cache` are out: the runner's preinstalled rustup and `actions/cache` would have to do.
- **CI time grows.**
  - The `unit` job would add `cargo test`.
  - Each `e2e` shard would add a release build, or download one built once.
  - `release` would add the Rust build and the zip script.
  - Minutes are free on the public repo (ADR 0018); the cost is wall time.
- **Lint and audit.** `checks` would add `cargo clippy`, `cargo fmt --check` and `cargo audit` next to oxlint and `npm audit`.

## Rust in an AI-built repo

- **New rules to write.** Like Solid 2, Tauri 2 differs from what most training data shows (Tauri 1's `allowlist`, `tauri::api`, v1 config). AGENTS.md would need a Tauri 2 rule block, plus Rust conventions (error handling, `unwrap` policy, clippy level).
- **Slower checks.** One changed line costs a 60–75 s release rebuild, against Vite's sub-second rebuild. Every subagent iteration on main-process code gets slower.
- **More review load.** Every main-process PR would be reviewed in Rust as well as TypeScript, by people and by review agents.
- **What the spike showed.** About 800 lines of Rust compiled on the first try. The two real bugs were the `\\?\` resource path and the missing `opening` event. Both compiled cleanly and showed up only when the app ran. A shared TypeScript type would have caught the second.

## Versions

- **Tauri:**
  - Rust crates: `tauri` 2.12.5, `tauri-build` 2.7.4, `tauri-plugin-dialog` 2.8.4, `tauri-plugin-opener` 2.7.2, `tauri-plugin-shell` 2.6.0 (probe only).
  - npm packages: `@tauri-apps/cli` and `@tauri-apps/api` 2.12.4.
  - wry 0.57.1.
- **Rust toolchain:** Rust 1.99.1 (MSVC), VS 2022 Build Tools (MSVC 14.44, Windows SDK 10.0.26100).
- **Test drivers:** tauri-driver 2.1.0, msedgedriver 156.0.4314.50.
- **Node sidecar:** Node 24.15.0 single executable application, injected with `postject` from the repo's `node_modules`.

## Sources

- Tauri 2 docs: [WebDriver testing](https://v2.tauri.app/develop/tests/webdriver/), [Windows installer](https://v2.tauri.app/distribute/windows-installer/), [resources](https://v2.tauri.app/develop/resources/), [sidecar](https://v2.tauri.app/develop/sidecar/), [capabilities](https://v2.tauri.app/security/capabilities/), [shell plugin](https://v2.tauri.app/plugin/shell/), [isolation pattern](https://v2.tauri.app/concept/inter-process-communication/isolation/), [app size](https://v2.tauri.app/concept/size/).
- API reference: [`WebviewBuilder::on_new_window` / `on_navigation`](https://docs.rs/tauri/latest/tauri/webview/struct.WebviewBuilder.html).
- Source read in `~/.cargo/registry`:
  - `tauri` 2.12.5 `src/app.rs` (windows built before `setup`).
  - `tauri-utils` 2.10.2 `src/platform.rs` (`resource_dir` canonicalizes `current_exe`).
  - wry 0.57.1 `src/lib.rs` (`browser_accelerator_keys`, `default_context_menus` default on).
- Node: [single executable applications](https://nodejs.org/api/single-executable-applications.html).
- WebView2: [environment variables](https://learn.microsoft.com/microsoft-edge/webview2/reference/win32/webview2-idl#createcorewebview2environmentwithoptions) (`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS`, `WEBVIEW2_USER_DATA_FOLDER`).

## Not verified

- **What no one looked at:** the windows were never looked at by a person, and the white flash and F5 reload were not tried by hand.
- **Not built or measured:**
  - The MSI with a numeric version.
  - The NSIS installer's installed size: it should be the unpacked size plus the uninstaller.
  - A build on a GitHub runner.
  - Option 2's sidecar with secrets, and option 2's fresh-profile numbers.
- **Not tested:** the `en-US`-only locale cut for Electron.
- **No people:** no test with people of the start-up feel, where Tauri's window shows earlier but paints later.
