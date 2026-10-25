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
| F10 | Run history: past runs, re-run, re-run failed only | should | idea | `karate-reports_yyyyMMdd_HHmmss` folders (`-B`); failed lines → `file:LINE` | — |
| F11 | HTTP inspector per step: request, response, headers, timing | should | idea | `FEATURE_EXIT.data` step logs after the run; live needs our own `RunListener` | — |
| F12 | Thread count and stop/kill run | should | idea | `-T`; kill process tree (Windows: `taskkill /T /F`, exits 1, so the app tracks "cancelled") | — |
| F13 | Project setup: Maven, Gradle, standalone jar, karate CLI | should | idea | `karate doctor --json`; `karate-pom.json` as project file | — |
| F14 | Step debugger: breakpoints, step, variables | should | idea | **not in OSS core** (debug server is in the paid `karate-ide` jar); needs our own `RunListener` | — |
| F15 | Trends: pass rate over time, flaky tests | should | idea | aggregate stored run history (F10) | — |
| F16 | Watch mode: re-run on save | could | idea | file watcher in main process | — |
| F17 | Open in external editor at file:line | could | idea | OS shell / editor URL scheme | — |
| F18 | Import Postman, cURL or OpenAPI to `.feature` | could | idea | none in OSS CLI | — |
| F19 | Export and share JUnit XML / HTML reports | could | idea | `-f junit:xml`, report folder | — |
| F20 | Data-driven view: per-row results of Examples | could | idea | `OUTLINE_ENTER`, `isOutlineExample`, `exampleIndex` | — |
| F21 | AI / MCP control of the runner | could | idea | — | — |
| F22 | Console filter and search | could | idea | console lines only; Karate log lines carry no scenario id | — |
| F23 | Tag filter AND and typed Karate tag expressions | could | idea | repeated `-t` (AND); JS `anyOf`/`allOf`/`not`, validated by a `-t` dry run | — |
