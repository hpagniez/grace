# Features

Backlog of what karate-kid does or will do. One row per user-facing capability.

- **Priority**: `must` (first release), `should`, `could`. Reasoning per row lives in [inventory/commercial.md](inventory/commercial.md#user-needs-a-free-runner-must-cover).
- **Status**: `idea` → `specced` → `building` → `done`. A row moves to `specced` only when its file exists in `specs/`.
- **Karate surface**: how the app gets the data. Details in [inventory/karate-core.md §19](inventory/karate-core.md#19-integration-surface-for-a-gui).
- Update the row in the same change that adds its spec or ships the feature.

| ID | Feature | Priority | Status | Karate surface | Spec |
|---|---|---|---|---|---|
| F01 | Browse test tree: folders, features, scenarios, outline examples | must | idea | dry run `-D -f karate:jsonl`, filter `callDepth == 0` | — |
| F02 | Run a feature, scenario or single example row | must | idea | `karate run file.feature:LINE` | — |
| F03 | Run a folder or the whole suite | must | idea | `karate run <paths>` | — |
| F04 | Run by tag: tag browser, include/exclude | must | idea | `-t` (repeat = AND, `,` = OR, `~` = NOT); tags from dry run | — |
| F05 | Live progress: per-node pass/fail, counts | must | idea | tail `karate-events.jsonl` (`SCENARIO_*`, `PROGRESS`) | — |
| F06 | Live console log with ANSI colors, filterable | must | idea | child stdout/stderr, `FORCE_COLOR=1`, `--log-console` | — |
| F07 | Failure details: error, step location, jump to line | must | idea | `SCENARIO_EXIT.error` (starts with `path.feature:line`), `FEATURE_EXIT.data` | — |
| F08 | Environment switcher + extra system properties | must | idea | list `karate-config-*.js`; `-e`; `-D` props via `java -Dk=v` | — |
| F09 | Open the generated HTML report | must | idea | `<output>/karate-summary.html` in a webview | — |
| F10 | Run history: past runs, re-run, re-run failed only | should | idea | `karate-reports_yyyyMMdd_HHmmss` folders (`-B`); failed lines → `file:LINE` | — |
| F11 | HTTP inspector per step: request, response, headers, timing | should | idea | `FEATURE_EXIT.data` step logs after the run; live needs our own `RunListener` | — |
| F12 | Thread count and stop/kill run | should | idea | `-T`; kill process tree (Windows: `taskkill /T /F`) | — |
| F13 | Project setup: Maven, Gradle, standalone jar, karate CLI | should | idea | `karate doctor --json`; `karate-pom.json` as project file | — |
| F14 | Step debugger: breakpoints, step, variables | should | idea | **not in OSS core** (debug server is in the paid `karate-ide` jar); needs our own `RunListener` | — |
| F15 | Trends: pass rate over time, flaky tests | should | idea | aggregate stored run history (F10) | — |
| F16 | Watch mode: re-run on save | could | idea | file watcher in main process | — |
| F17 | Open in external editor at file:line | could | idea | OS shell / editor URL scheme | — |
| F18 | Import Postman, cURL or OpenAPI to `.feature` | could | idea | none in OSS CLI | — |
| F19 | Export and share JUnit XML / HTML reports | could | idea | `-f junit:xml`, report folder | — |
| F20 | Data-driven view: per-row results of Examples | could | idea | `OUTLINE_ENTER`, `isOutlineExample`, `exampleIndex` | — |
| F21 | AI / MCP control of the runner | could | idea | — | — |
