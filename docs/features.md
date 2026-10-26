# Features

Backlog of what karate-kid does or will do. One row per user-facing capability.

- **Priority**: `must` (first release), `should`, `could`. Reasoning per row lives in [inventory/commercial.md](inventory/commercial.md#user-needs-a-free-runner-must-cover).
- **Status**: `idea` → `specced` → `building` → `done`. A row moves to `specced` only when its file exists in `specs/`.
- **Karate surface**: how the app gets the data. Details in [inventory/karate-core.md §19](inventory/karate-core.md#19-integration-surface-for-a-gui).
- Update the row in the same change that adds its spec or ships the feature.

| ID | Feature | Priority | Status | Karate surface | Spec |
|---|---|---|---|---|---|
| F01 | Browse test tree: folders, features, scenarios, outline examples | must | done | dry run `-D -f karate:jsonl`, filter `callDepth == 0`, key by feature + `refId` section (no line) | [spec](specs/2026-10-24-walking-skeleton.md) |
| F02 | Run a feature, scenario or single example row | must | done | `karate run file.feature:LINE` | [spec](specs/2026-10-24-walking-skeleton.md) |
| F03 | Run a folder or the whole suite | must | done | `karate run <paths>` | [spec](specs/2026-10-24-walking-skeleton.md) |
| F04 | Run by tag: tag chips, include (OR) / exclude, dim or hide non-matching, saved per project | must | done | tags from dry run (`SCENARIO_ENTER.tags` = full inherited set); app matches, run passes `-t @a,@b -t ~@c` | [spec](specs/2026-10-24-tag-filter.md) |
| F05 | Live progress: per-node pass/fail, counts | must | done | tail `karate-events.jsonl`; count depth-0 `SCENARIO_EXIT` vs dry-run total (no `PROGRESS` event) | [spec](specs/2026-10-24-walking-skeleton.md) |
| F06 | Live console log, ERROR/WARN highlighted | must | done | child stdout + stderr (logs are on stderr); `--no-color`, UTF-8 via `-Dstdout.encoding` | [spec](specs/2026-10-24-walking-skeleton.md) |
| F07 | Failure details: error, location, step list with step logs | must | done | `SCENARIO_EXIT.error` (starts with `path.feature:line`), `FEATURE_EXIT.data` | [spec](specs/2026-10-24-walking-skeleton.md) |
| F08 | Environment switcher + run properties per environment, with encrypted secrets | must | done | `-e` on dry runs and runs; suggestions from `karate-config-<env>.js` and `@env`/`@envnot` tags; `-D` props via a `java @file` argument file; secrets in Electron `safeStorage` | [spec](specs/2026-10-25-environment-switcher.md) |
| F09 | Open the generated HTML report | must | done | `<output>/karate-summary.html` opened in the default browser | [spec](specs/2026-10-24-walking-skeleton.md) |
| F10 | Run history: History tab listing the last 20 runs (date, environment, tags, what ran, counts, duration), open a past run's report, re-run it on its environment and tags, re-run failed only | should | done | own `-o` folder per run plus an app-written `run.json` ([ADR 0010](decisions/0010-run-history-one-folder-per-run.md)); failed tests → `file:L1:L2` args per file | [spec](specs/2026-10-25-run-history.md) |
| F11 | HTTP inspector: every call of the run in an HTTP tab, grouped by status class (1xx to 5xx, no response), narrowed by the tree selection; request and response headers and bodies, timing; pretty JSON/XML, copy as cURL | should | done | after the run: parse the HTTP text in depth-0 `FEATURE_EXIT.data` step logs, nested `callResults` included; live: our own `RunListener` (shared with F14). See [notes](#f11-http-inspector) | [spec](specs/2026-10-25-http-inspector.md) (after-run version) |
| F12 | Thread count: choose how many scenarios run in parallel, saved per project (Cancel already ships, see F32) | should | idea | `-T <n>`; progress already counts depth-0 `SCENARIO_EXIT`, so it holds with parallel runs; console lines interleave across threads | — |
| F13 | Project setup: Maven, Gradle, standalone jar, karate CLI | should | idea | `karate doctor --json`; `karate-pom.json` as project file | — |
| F14 | Step debugger: breakpoints on steps, pause, step over, continue, inspect variables | should | idea | **not in OSS core** (debug server is in the paid `karate-ide` jar); needs our own Java `RunListener` jar passed with `--listener`, talking to the main process over a local socket. See [notes](#f14-step-debugger) | — |
| F15 | Trends: pass rate over time, flaky tests | should | idea | aggregate stored run history (F10) | — |
| F16 | Watch mode: re-run on save | could | idea | file watcher in main process | — |
| F17 | Open in external editor at file:line | could | idea | OS shell / editor URL scheme | — |
| F18 | Import Postman, cURL or OpenAPI to `.feature` | could | idea | none in OSS CLI | — |
| F19 | Export and share JUnit XML / HTML reports | could | idea | `-f junit:xml`, report folder | — |
| F20 | Data-driven view: per-row results of Examples | could | idea | `OUTLINE_ENTER`, `isOutlineExample`, `exampleIndex` | — |
| F21 | AI / MCP control of the runner | could | idea | — | — |
| F22 | Console filter and search | could | idea | console lines only; Karate log lines carry no scenario id | — |
| F23 | Tag filter AND and typed Karate tag expressions | could | idea | repeated `-t` (AND); JS `anyOf`/`allOf`/`not`, validated by a `-t` dry run | — |
| F24 | Console that reads as a terminal: monospace on a dark panel, clearly separated from the rest, legible in dark mode | should | idea | console lines only (as F06) | — |
| F25 | Custom window title bar instead of the default Electron frame | could | idea | none (Electron frameless window with its own drag region and window buttons) | — |
| F26 | App updates: how a new version reaches people who installed the zip (auto-update, or an update notice with a download link) | should | idea | none; needs a decision record (Squirrel/installer vs zip, GitHub Releases on a private repo, signing) | — |
| F27 | Home screen: choose a project folder, with the list of recently opened folders | should | idea | none (recent folders saved in `settings.json`, next to `lastProject`) | — |
| F28 | Feature file editor in the app, with Karate autocompletion | could | idea | Karate keywords and `karate.*` API from [inventory/karate-core.md](inventory/karate-core.md); parse errors from a dry run (exit 3) | — |
| F29 | Loading always visible: a loader or animation while opening, discovering, saving and running | should | idea | none (the Runner's `busy` state and run progress) | — |
| F30 | Toolbar that stays readable with a long project path: less crowded layout for the path and the buttons | should | idea | none | — |
| F31 | Open… starts in the last opened folder | should | done | none (`dialog.showOpenDialog` with the app window as parent and `defaultPath` = the parent folder of `lastProject`, so sibling projects are one click away) | [spec](specs/2026-10-26-ui-polish.md) |
| F32 | Cancel a run or a discovery dry run | must | done | kill the process tree (Windows: `taskkill /PID <pid> /T /F`, exits 1, so the app records "cancelled") | [spec](specs/2026-10-24-walking-skeleton.md) |
| F33 | Update the bundled JRE, Karate jar and other packed dependencies reliably: see that a new version exists, bump it with its checksum, prove the app still works | should | idea | none in the app. Versions, URLs and SHA-256 are pinned in `scripts/fetch-vendor.mjs` (Temurin 21.0.12.1+1, Karate 2.1.4); Dependabot watches only npm and GitHub Actions. See [notes](#f33-bundled-dependency-updates) | — |
| F34 | Usage telemetry: track and monitor how the app is used (features used, runs, errors) | could | idea | none; needs a decision record (opt-in, what is collected, where it is sent, retention). See [notes](#f34-telemetry) | — |
| F35 | In-app feedback: report an issue or propose an improvement, and upvote other people's ideas, from inside the app | could | idea | none; needs a tracker people can reach and a decision record: the repo is private (ADR 0007), so its GitHub Issues are not open to users. See [notes](#f35-in-app-feedback-and-voting) | — |
| F36 | Mouse cursor matches the action: pointer on clickable items, not-allowed on disabled ones, a busy cursor while opening, discovering or running, text cursor in fields | should | done | none (buttons show Chromium's default arrow: Tailwind v4's preflight dropped v3's `cursor: pointer`; today only disabled controls set a cursor. Base rules in `src/index.css`, `progress` on the app root while busy) | [spec](specs/2026-10-26-ui-polish.md) |
| F37 | Resizable bottom panel: drag a splitter between the tree and details area and the bottom panel (Console, History, HTTP) to change its height; resize with the keyboard too; a sensible minimum and maximum; the height is remembered | should | done | none (the panel is a fixed `h-48` in `BottomPanel.tsx`, so the HTTP tab (F11) shows about 5 call rows and 13 calls already need scrolling; height saved per app in the renderer's `localStorage`, read before the first paint so it never jumps) | [spec](specs/2026-10-26-ui-polish.md) |
| F38 | Themed scrollbars: every scroll area (tree, details pane, console, History and HTTP tabs, request and response bodies) gets thin scrollbars styled to match the app, in light and dark mode, instead of the default Windows ones; hover and dragging stay easy, and they stay visible enough to show that an area scrolls | should | done | none (CSS `scrollbar-width: thin` and `scrollbar-color` with the theme tokens, set once in `src/index.css`; Chromium in Electron 45 supports both; fall back to `::-webkit-scrollbar` only if the result looks wrong; no dependency) | [spec](specs/2026-10-26-ui-polish.md) |
| F39 | No white flash at start: the window appears already painted in the app's theme (light or dark), with a startup screen until the app has loaded | should | done | none (create the `BrowserWindow` with `show: false`, show it on `ready-to-show`, and set `backgroundColor` from `nativeTheme.shouldUseDarkColors`: `#18181b` (zinc-900) or `#ffffff`; put a small static startup screen (app name and a spinner, inline CSS that follows `prefers-color-scheme`) in `index.html`, removed once the app is drawn: Solid's `render` appends to a `#root` that has children, it does not replace them). Today the window is created with neither option, so it opens white while the empty `#root` waits for `renderer.tsx`; dev is slower (Vite serves unbundled modules) and Forge reloads the renderer once, a second flash. Related to F29 (loaders) | [spec](specs/2026-10-26-ui-polish.md) |
| F40 | Arrow-key navigation in the tree and the HTTP list: each list is one tab stop; Up/Down move between rows, Left/Right collapse and expand tree nodes, Home/End jump, Enter or Space selects, and the run button stays reachable from the keyboard (for example a shortcut on the focused row) | should | idea | none (follow the WAI-ARIA tree and listbox patterns with a roving `tabindex`. Today Tab and Shift+Tab stop on every button: two per tree row (the name and the ▶ run button) and one per HTTP call row, so long lists are slow to cross. `src/renderer/Tree.tsx` already uses `role="tree"`; the HTTP rows are buttons in `src/renderer/HttpCalls.tsx` (F11)) | — |

## Idea notes

Starting points for the larger ideas. The spec, written just before building, decides.

### F11 HTTP inspector

- First version, after the run: parse the request and response text that Karate writes into step logs in `FEATURE_EXIT.data`. Show a list of calls per scenario, then a detail pane with headers, body (pretty JSON or XML) and timing.
- Copy a request as cURL. Mask values the environment switcher (F08) marks as secret.
- Live version: the `RunListener` from F14 streams `HTTP_*` events while the run goes.

### F14 Step debugger

- A small Java `RunListener` jar, built and bundled with the app like the Karate jar (ADR 0006), added to the classpath and passed with `--listener`.
- It connects to a local socket opened by the main process and sends `STEP_*`, `HTTP_*` and `MATCH_EXIT` events. Returning from `onEvent` only when the app says "continue" makes it pause on a breakpoint.
- Breakpoints are set on step lines in the tree or in a feature view (F28). While paused, show the scenario's variables, read through the listener.
- Building Java needs its own toolchain in the repo and a decision record. Run one thread (`-T 1`) while debugging.

### F33 Bundled dependency updates

- A scheduled CI workflow checks for a newer Temurin 21 JRE (Adoptium API) and Karate release (GitHub releases). When one exists, it opens a PR that bumps the URL, file name and SHA-256 in `scripts/fetch-vendor.mjs`.
- That PR's normal CI (unit and e2e tests with the bundled Java and Karate) is the proof that it still works. A Karate bump also re-checks the CLI flags against `RunCommand.java` and re-records the event fixtures with `npm run fixtures` (AGENTS.md).
- A Karate bump must re-record the HTTP inspector fixtures (`test/fixtures/events/run-http*.jsonl`): F11 parses Karate's step-log text, which can change between versions. Acceptance: the re-recorded http sample project still yields the 13 calls of the F11 spec table, and `src/main/karate/http.test.ts` passes.
- Major versions (a new Java LTS, Karate 3) stay a manual change: they can change the CLI, the event file or the JVM options.
- Showing the bundled versions in the app helps bug reports (F35).

### F34 Telemetry

- Opt-in: off until the person agrees, asked once, and a setting turns it off at any time. The app works the same without it.
- Anonymous: a random install id, app version, OS, features used, run counts and durations, error types. Never project paths, test or environment names, property values (F08 secrets) or console text.
- Sent from the main process in batches, never blocking a run; nothing is sent when offline or opted out.
- The decision record picks the service (hosted or self-hosted, within free plans), the retention and the privacy notice.

### F35 In-app feedback and voting

- A Feedback entry in the app opens a report or idea form, prefilled with the app version and OS (never project data).
- Voting needs identities, or another way to stop repeat votes. Options for the decision record: GitHub Issues or Discussions once the repo is public (people need a GitHub account; votes are reactions), or a hosted feedback board.
- A first step can be a link with prefilled text, before any UI of our own.
- One place for "about, feedback, privacy" ties this to F26 (update notices) and F34 (telemetry consent).
