---
paths:
  - "src/main/karate/**"
  - "src/main/runner*"
  - "crates/grace-core/**"
  - "listener/**"
  - "test/fixtures/**"
  - "scripts/capture-fixtures.mjs"
  - "scripts/vendor.json"
  - "scripts/bump-vendor.mjs"
---

# Karate integration rules

Full detail: [docs/inventory/karate-core.md §19](../../docs/inventory/karate-core.md#19-integration-surface-for-a-gui). Where they disagree, the [CLI spike](../../docs/research/2026-10-24-karate-cli-spike/README.md) wins: it ran Karate 2.1.4 for real. Its `samples/` are real event-stream fixtures.

- Before changing behaviour, read the indexes [docs/specs/README.md](../../docs/specs/README.md) and [docs/decisions/README.md](../../docs/decisions/README.md), and open only the docs whose line matches.
- Follow the Karate CLI first: for every feature and every kind of project, do what the Karate CLI does, and build Grace's own way only where it has none ([ADR 0026](../../docs/decisions/0026-follow-the-karate-cli-first.md)).
- Spawn from the Electron main process only, without a shell: `java -Dstdout.encoding=UTF-8 -Dstderr.encoding=UTF-8 -cp "<karate.jar>;<project classpath roots>" io.karatelabs.Main run --no-color -w <root> -o <absolute> -f html,karate:jsonl ...` with `cwd` = project root. Relative paths resolve from the process cwd, not `-w`.
- Remove `KARATE_OPTIONS`, `JAVA_TOOL_OPTIONS`, `JDK_JAVA_OPTIONS` and `_JAVA_OPTIONS` from the child environment; they override the app's values.
- Tail `<output>/karate-json/karate-events.jsonl` for live progress. There is no `PROGRESS` event: count depth-0 `SCENARIO_EXIT` against the dry-run total. Filter `callDepth == 0` for the test tree. Key nodes by feature + `refId` (slugs collide). Check `skipped` before `passed` (dry run sets both). Ignore `OUTLINE_ENTER.numExamples`.
- `-D` is dry run, `-p` is the pom file, `-n` is the scenario-name filter. Do not invent flags; check `RunCommand.java`.
- Tags: `@a`, `~@a`, `@a,@b` (OR), repeated `-t` (AND), or Karate JS `anyOf('@a','@b') && not('@c')`. `(@a or @b) and not @c` fails with exit 3.
- Exit codes: `0` pass, `1` test failures (also after `taskkill`), `2` usage error, `3` parse or runtime error (one broken feature aborts the whole run). A missing path, unknown `-e` or unmatched `-n` exit `0` silently: preflight with a dry run and warn on 0 scenarios.
- On Windows, cancel with `taskkill /PID <pid> /T /F` and record "cancelled" in the app. `child.kill()` leaves the JVM running when spawned through a shell or launcher.
- HTTP request/response detail exists only as text in step logs inside `FEATURE_EXIT`. OSS Karate v2 has no debug server. Live step and HTTP events need our own Java `RunListener` passed with `--listener`.
- The step debugger is that listener: `listener/grace/DebugListener.java`, on the classpath and passed with `--listener grace.DebugListener` only in a debug run, on one thread. It talks to the main process over a loopback socket (spec `docs/specs/2026-11-02-step-debugger.md`). It may use only Karate's public listener interface, never `karate.debug.port` or the paid `io.karatelabs.debug` classes.
- The HTTP inspector (F11) parses that step-log text in `src/main/karate/http.ts`; the text is Karate's, not a contract. After any Karate version change, run `npm run fixtures` to re-record `test/fixtures/events/` (including `run-http*.jsonl`), check the recordings still hold the http sample project's calls (spec `docs/specs/2026-10-25-http-inspector.md`, Acceptance criteria), and keep `src/main/karate/http.test.ts` green. The `vendor-updates` bump PR re-records them on its own.
