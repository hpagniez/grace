# Karate v2 CLI spike on Windows

Date 2026-10-24. Windows 11 (French locale), Node 24, portable Temurin JRE 21.0.12.1+1, `karate-2.1.4.jar` (the only jar in the [v2.1.4 release](https://github.com/karatelabs/karate/releases/tag/v2.1.4)). Checked against [inventory/karate-core.md §19](../../inventory/karate-core.md#19-integration-surface-for-a-gui).

Throwaway spike. Kept here as evidence and as parser fixtures:

- [project/](project/): offline sample project. A Karate mock started from `karate-config.js` via `karate.callSingle` → `karate.start`, plus a passing, a failing, a called, an outline and a slow scenario; tags `@smoke`, `@slow`, `@wip`; `broken/syntax.feature` is invalid on purpose.
- [samples/](samples/): real `karate-events.jsonl` excerpts (normal run with failure, outline rows + called feature, dry run, file after `taskkill`). Absolute paths rewritten to `C:\spike`.
- [scripts/](scripts/): the Node harness. `run-case.mjs` spawns java, captures stdout/stderr/exit code and timestamps each JSONL line arrival. Expects the JRE and jar under `../dl/` (not committed).

## Corrections to the inventory

| Inventory said | Spike showed |
|---|---|
| `PROGRESS` event with counts | Never emitted. Count depth-0 `SCENARIO_EXIT` against the dry-run total. |
| Dry-run step status unclear | Scenarios are `passed:true` **and** `skipped:true`; steps are `skipped`. Check `skipped` first. |
| Colour auto-off without a TTY | ANSI stays on when piped. Use `--no-color` or `NO_COLOR=1`. |
| Boolean tags `(@a or @b) and not @c` maybe supported | Fails, exit 3 (parsed as JS). Use `anyOf('@a','@b') && not('@c')`. |
| `-w` sets the working dir | Relative feature paths and relative `-o` resolve from the **process cwd**, not `-w`. |
| Mocks stop via `GET /__admin/stop` | 404 in OSS. Kill the process. |
| — | `OUTLINE_ENTER.numExamples` is wrong (3 rows → 2). Ignore it. |
| — | Slugs are not unique. Key nodes by feature + `refId`. |
| — | An outline row's `line` is its `Examples:` line. Row line = Examples line + 2 + `exampleIndex`. |
| — | Console text uses the Windows code page and locale decimals. Fix encoding with `-Dstdout.encoding=UTF-8 -Dstderr.encoding=UTF-8`. |
| — | Logback logs go to **stderr**; stdout has only summaries. |
| — | `JAVA_TOOL_OPTIONS` (and `JDK_JAVA_OPTIONS`, `_JAVA_OPTIONS`) can inject `-Dkarate.options` too. |

Confirmed as documented: `KARATE_OPTIONS` overrides CLI flags; exit codes 0/1/2/3; report backups named `<o-dir>_yyyyMMdd_HHmmss`; `:LINE` and `-n` bypass `-t`; mock prints `  URL: http://localhost:<port>` on stdout.

## Results per case

### Dry run: `run -D -f html,karate:jsonl -o <abs> -w <proj> tests`
Exit 0 in about 1.1 s. Config JS is not executed (stderr still logs "karate-config.js processed"). `SUITE_ENTER.env` is null. Summary counts every scenario as both passed and skipped.

### Normal run: `run -e dev -T 2 -f html,karate:jsonl,junit:xml`
Exit 1 (intended failure). Writes `index.html`, `karate-summary.html`, `karate-timeline.html`, `feature-html/`, `junit-xml/`, `karate-json/karate-events.jsonl`, `res/`.

### Tags
| `-t` | Result |
|---|---|
| `@smoke` | works |
| `~@wip` | works |
| `@smoke,@slow` | OR, works |
| `-t @smoke -t @slow` | AND, works |
| `"(@smoke or @slow) and not @wip"` | exit 3, `ParserException expected [R_PAREN]` |
| `"anyOf('@smoke','@slow') && not('@wip')"` | works |

### Line and name selection
- `outline.feature:11` → that row only. `:3` (outline header) → all rows. A line inside a scenario selects the scenario. `:7:15` → two scenarios.
- `-n "failing match"` works. Outline rows need the expanded name (`add 2 and 3`); the template name matches nothing.
- No match → exit 0, no warning.

### Cancel (slow scenario, killed 1.5 s after its `SCENARIO_ENTER`)
| Spawn | Kill | Leftover java? |
|---|---|---|
| `spawn(java)` | `taskkill /PID <pid> /T /F` | no (exit code 1) |
| `spawn(java)` | `child.kill()` | no |
| `spawn(java, {shell:true})` | `child.kill()` | **yes**: java finished the run and wrote a full report |
| `spawn(java, {shell:true})` | `taskkill /T /F` | no |

After a hard kill the JSONL ends on a complete line with no EXIT events, and the report dir has no HTML. The next run rotates the partial dir. Exit 1 after `taskkill` looks like a test failure, so the app must record "cancelled" itself.

### Exit codes
| Case | Exit |
|---|---|
| all pass | 0 |
| test failure | 1 |
| unknown flag, `-T abc` | 2 |
| syntax error in any selected feature (whole run aborts) | 3 |
| bad tag expression | 3 |
| missing path, unknown `-e`, `-n` with no match | **0, silent** |
| missing `-w` dir | 1 |
| killed by `taskkill` | 1 |

## JSONL event stream

- Envelope `{type, timeStamp, threadId, data}`; `threadId` is `thread-NN`, null on `SUITE_*`. `schemaVersion` is `"1"`.
- Seen: `SUITE_ENTER`, `FEATURE_ENTER`, `OUTLINE_ENTER`, `SCENARIO_ENTER`, `SCENARIO_EXIT`, `ERROR`, `FEATURE_EXIT`, `SUITE_EXIT`. Never seen: `PROGRESS`, `OUTLINE_EXIT`, `STEP_*`, `HTTP_*`, `MATCH_EXIT`.
- Each line reaches disk within about 20 ms of its `timeStamp`, so tailing gives live progress.
- Fields:
  - `FEATURE_ENTER`: `path` (relative to `-w`), `slug`, `name`, `description`, `tags` (no `@`), `line`, `sourceHash`, `callDepth`.
  - `SCENARIO_ENTER`/`EXIT`: `feature`, `slug`, `executionIndex`, `name` (null if unnamed), `line`, `refId`, `callDepth`, `tags`; outline rows add `isOutlineExample`, `exampleIndex` (0-based, resets per Examples table), `outlineSlug`. EXIT adds `passed`, `skipped`, `durationMillis`, and on failure `error` (starts with the absolute `path:line`) and `errorReason`.
  - `ERROR`: `feature`, `scenario`, `message`, `type`.
  - `FEATURE_EXIT`: full result tree, `tags` as `{name:"@x", line}`, `scenarioResults[].stepResults[]` with `step`, `result{status, millis}`, `stepLog`, `callResults`.
  - `SUITE_EXIT`: `summary{featureCount, featuresPassed, featuresFailed, scenarioCount, scenariosPassed, scenariosFailed, scenariosSkipped, passedRate, durationMillis, passed}`.
- Called features emit events at `callDepth` 1 inside the caller's scenario; no caller field, `threadId` links them. A `callSingle` from config emits a depth-1 feature with no depth-0 parent. Filtering `callDepth == 0` works.
- HTTP request and response (headers, bodies, timing) exist only as **text** in the step's `stepLog` inside `FEATURE_EXIT`, so HTTP detail is available when the feature ends, not live.

## Impact on the app design

- **Spawn:** `java -Dstdout.encoding=UTF-8 -Dstderr.encoding=UTF-8 -jar karate.jar run --no-color -w <root> -o <abs> -f html,karate:jsonl ...` with `{cwd: root}` and no shell. Remove `KARATE_OPTIONS`, `JAVA_TOOL_OPTIONS`, `JDK_JAVA_OPTIONS`, `_JAVA_OPTIONS` from the child env.
- **Cancel:** always `taskkill /T /F` on Windows; track "cancelled" in the app; mark scenarios with an open `SCENARIO_ENTER` as interrupted.
- **Tree:** build from a dry run (about 1 s). Key by feature + `refId`. Group outline rows by `outlineSlug` + `line`.
- **Preflight:** a dry run surfaces syntax errors (exit 3); warn on 0 scenarios, missing paths and missing `karate-config-<env>.js`.
- **Tag UI:** the four simple forms; an advanced box takes Karate JS (`anyOf`/`allOf`/`not`), validated by a dry run.
- **Console:** merge stdout and stderr; never parse numbers from it.
- **Live step and HTTP detail** still need our own Java `RunListener`.

## Not done

The Rust launcher (not installed; launcher kill behaviour inferred from the `shell:true` case), what triggers `PROGRESS`, `@setup` in dry run, `-C`, `-g`, `--listener`, `karate-pom.json`, the `_n` backup suffix. Windows only.
