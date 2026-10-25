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
| F10 | Run history: list of past runs (date, environment, tags, counts, duration), open a past run's results and report, re-run it, re-run failed only | should | idea | keep each run's output folder (`-B` timestamped `karate-reports_yyyyMMdd_HHmmss`, or our own per-run `-o`) plus a small run summary; failed scenarios → `file:LINE` args. See [notes](#f10-run-history) | — |
| F11 | HTTP inspector per step: method, URL, status, request and response headers and bodies, timing; pretty JSON/XML, copy as cURL | should | idea | after the run: parse the HTTP text in `FEATURE_EXIT.data` step logs; live: our own `RunListener` (shared with F14). See [notes](#f11-http-inspector) | — |
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
| F31 | Open… starts in the last opened folder | should | idea | none (`dialog.showOpenDialog` `defaultPath` from `lastProject`) | — |
| F32 | Cancel a run or a discovery dry run | must | done | kill the process tree (Windows: `taskkill /PID <pid> /T /F`, exits 1, so the app records "cancelled") | [spec](specs/2026-10-24-walking-skeleton.md) |

## Idea notes

Starting points for the larger ideas. The spec, written just before building, decides.

### F10 Run history

- Store each run under the app's data folder: its output folder (HTML report, `karate-events.jsonl`) and a summary file (start time, duration, environment, tag filter, paths run, counts, exit code or "cancelled").
- A history list per project, newest first. Selecting a run shows its tree results, failures and console from the stored files, without running Karate.
- Re-run repeats the same paths, environment and tags. Re-run failed only passes the failed scenarios as `file:LINE` arguments (outline examples by their example line).
- Keep the last N runs per project (a setting) and delete older folders, since reports grow fast.
- Feeds F15 (trends and flaky tests).

### F11 HTTP inspector

- First version, after the run: parse the request and response text that Karate writes into step logs in `FEATURE_EXIT.data`. Show a list of calls per scenario, then a detail pane with headers, body (pretty JSON or XML) and timing.
- Copy a request as cURL. Mask values the environment switcher (F08) marks as secret.
- Live version: the `RunListener` from F14 streams `HTTP_*` events while the run goes.

### F14 Step debugger

- A small Java `RunListener` jar, built and bundled with the app like the Karate jar (ADR 0006), added to the classpath and passed with `--listener`.
- It connects to a local socket opened by the main process and sends `STEP_*`, `HTTP_*` and `MATCH_EXIT` events. Returning from `onEvent` only when the app says "continue" makes it pause on a breakpoint.
- Breakpoints are set on step lines in the tree or in a feature view (F28). While paused, show the scenario's variables, read through the listener.
- Building Java needs its own toolchain in the repo and a decision record. Run one thread (`-T 1`) while debugging.
