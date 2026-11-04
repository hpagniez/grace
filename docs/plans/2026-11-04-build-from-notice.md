# Build from the notice Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Build button in the "Java code is not built" notice runs `test-compile` (Maven) or `testClasses` (Gradle), then refreshes.

**Architecture:** The library listing's spawn block in `Runner.listLibraries` becomes one private `runTool`, used by the listing and the new `Runner.build`. The `listing` event and the `notice` event each get an optional `build` flag; the window turns the first into "Building…" text and the second into a Build button. A successful Build writes its start time to `<project data folder>/built.json`, which the Java code check reads.

**Tech Stack:** Electron main (Node), SolidJS 2 renderer, Vitest, Playwright.

**Spec:** [docs/specs/2026-11-04-build-from-notice.md](../specs/2026-11-04-build-from-notice.md). Read it with this plan.

## Global Constraints

- Follow `AGENTS.md`: Solid 2 rules, UI conventions (semantic tokens only, shared classes from `src/renderer/buttons.ts`, no `title=`), Conventional Commits (subject ≤ 72 characters, lower case), no backward compatibility.
- Spawn only through `spawnKarate(CMD, ["/d", "/s", "/c", `"<command>"`], root, …, { env, windowsVerbatimArguments: true })` as `listLibraries` does; every path goes in through an environment variable. Cancel stays `this.proc.kill()` (`taskkill /T /F`).
- Commands, verbatim: Maven `"%GRACE_TOOL%" -B -q test-compile`; Gradle `"%GRACE_TOOL%" -q --no-daemon --console=plain testClasses`.
- The build's environment is `{ ...process.env, GRACE_TOOL: tool }`: never Grace's JRE, no `GRACE_CLASSPATH_FILE`.
- Texts, verbatim: `Building the project's Java code with Maven…` / `…with Gradle…`; `Build cancelled.`; `Grace could not build the project's Java code with Maven: <reason>.` (no full stop after a reason ending in `?`); ` Building needs a JDK: set JAVA_HOME to one.`
- Idle costs nothing: no timer, poll or watcher.
- Run single unit test files freely. Full `npm test` and `npm run e2e` only after the parent session's go.

## Review Focus

1. A reason ending with `?` (Maven's "…rather than a JDK?") gets no second stop: pinned in Task 1's `buildFailure` test.
2. A second click while the first build runs is refused with the busy notice and starts no second tool: pinned in Task 2.
3. A not-built notice joined with other notices (karate-pom.json settings, the libraries' failure) still carries the button when a tool exists: pinned in Task 2.
4. A project folder whose path holds spaces and `&` builds: pinned in Task 2 (reuse `project("pom.xml", parent)`).
5. Gradle "changed" notice with no class rewritten clears after Build, and comes back when a source changes after it (spec R4.3): pinned in Task 2.

---

### Task 1: Pure helpers: build command, JDK lookup, failure text, kept build time

**Files:**
- Modify: `src/main/karate/libraries.ts`, `src/main/karate/compiled.ts`
- Test: `src/main/karate/libraries.test.ts`, `src/main/karate/compiled.test.ts`

**Interfaces:**
- Produces, in `libraries.ts`:
  - `export const BUILD_COMMAND: Record<BuildLayout, string>` (the two commands above).
  - `export function hasJdk(env: NodeJS.ProcessEnv): boolean`: `javac.exe` in `<JAVA_HOME without quotes>\bin` or in any `folders(env.PATH ?? "")` entry.
  - `export function buildFailure(tool: "Maven" | "Gradle", lines: string[], jdk: boolean): string`: the full notice text of spec R5.1 and R5.2. Reason = the first line matching `/\.java:\d+: error: /` (trimmed), else `failureReason(lines)`; then `.` unless the reason ends with `?`; then the JDK sentence when `!jdk`.
- Produces, in `compiled.ts`:
  - `export function readBuilt(dir: string): number | null` (reads `built.json` `{ "at": <ms> }`; null when missing or not a finite number; never throws).
  - `export function writeBuilt(dir: string, at: number): void` (creates `dir`).
  - `testCodeNotice(root, layout, builtAt: number | null = null)`: R8.3 fires only when the newest source is newer than both the newest class and `builtAt`. R8.2 unchanged.

- [ ] **Step 1: Write the failing tests**
  - `libraries.test.ts`, `describe("building")`:
    - `BUILD_COMMAND.maven` is `"%GRACE_TOOL%" -B -q test-compile`; `.gradle` contains `--no-daemon` and ends with `testClasses`.
    - `hasJdk({})` false; true with `JAVA_HOME` = a temp folder holding `bin/javac.exe`; true with `JAVA_HOME` quoted (`"<dir>"`); true with `PATH` = `x;<dir>\bin`; false with `JAVA_HOME` holding only `bin/java.exe`.
    - `buildFailure("Gradle", ["C:\\p\\Helper.java:3: error: cannot find symbol", "* What went wrong:", "Execution failed for task ':compileTestJava'.", "> Compilation failed; see the compiler error output for details."], true)` is `Grace could not build the project's Java code with Gradle: C:\p\Helper.java:3: error: cannot find symbol.`
    - `buildFailure("Maven", ["[ERROR] COMPILATION ERROR : ", "[ERROR] No compiler is provided in this environment. Perhaps you are running on a JRE rather than a JDK?"], false)` is `Grace could not build the project's Java code with Maven: No compiler is provided in this environment. Perhaps you are running on a JRE rather than a JDK? Building needs a JDK: set JAVA_HOME to one.`
  - `compiled.test.ts`: with a source at t=200 s and a class at t=100 s, `testCodeNotice(root, "maven", 300_000)` is null and `testCodeNotice(root, "maven", 150_000)` is the CHANGED text; with no class, `testCodeNotice(root, "maven", 300_000)` is the NOT_BUILT text; `readBuilt` is null for a missing file and for `{"at":"x"}`, and returns what `writeBuilt` wrote.
- [ ] **Step 2: Run** `npx vitest run src/main/karate/libraries.test.ts src/main/karate/compiled.test.ts`. Expected: the new tests fail.
- [ ] **Step 3: Implement** the interfaces above. Update `compiled.ts`'s header comment (Grace builds only on the person's click).
- [ ] **Step 4: Run** the same command. Expected: all pass.
- [ ] **Step 5: Commit** `feat(runner): add the build command, jdk lookup and build time`

### Task 2: Runner.build, the shared tool run, the IPC call

**Files:**
- Modify: `src/main/runner.ts`, `src/shared/events.ts`, `src/shared/ipc.ts`, `src/preload.ts`, `src/main.ts`
- Test: `src/main/runner.libraries.test.ts` (new `describe("Build")` inside `describe("the project's Java code")`, reusing `withHelper`, `at`, `NOT_BUILT`, `CHANGED`)

**Interfaces:**
- Consumes: Task 1's `BUILD_COMMAND`, `hasJdk`, `buildFailure`, `readBuilt`, `writeBuilt`, `testCodeNotice(root, layout, builtAt)`.
- Produces:
  - `events.ts`: `{ type: "notice"; message: string; success?: boolean; build?: true }` (the Build button) and `{ type: "listing"; tool: "Maven" | "Gradle" | null; build?: true }` (the tool is building). Emit `build` only when true, so existing `toEqual` checks hold.
  - `ipc.ts`: `CH.build = "project:build"`; `Api.build(): Promise<void>`. `preload.ts` and `main.ts` (`handle(CH.build, () => runner.build())`) wire it.
  - `Runner.build(): Promise<void>`: returns at once with no project or a standalone one; otherwise `guarded`. No tool: notice `Grace could not build the project's Java code with <Tool>: <noToolReason(layout)>.` without `build`. Else keep `started = Date.now()`, `runTool`; cancelled → notice `Build cancelled.` with `build: true`; exit ≠ 0 → notice `buildFailure(tool, lines, hasJdk(lookup))` with `build: true`; exit 0 → `writeBuilt(projectDir, started)` then the refresh body.
  - `private runTool(root: string, layout: BuildLayout, command: string, env: NodeJS.ProcessEnv, build: boolean): Promise<{ code: number | null; lines: string[] }>`: the spawn, `this.proc`, `this.listing` and the `listing` events, moved out of `listLibraries` unchanged; keeps `this.building = build` for `replay()`, which sends `{ type: "listing", tool, build: true }` while building.
  - A private refresh body shared by `refresh()` and `build()`; `open` and the refresh body pass `readBuilt(projectDirOf(...))` to `testCodeNotice`, and add `build: true` to their joined notice when the Java code text is in it and `findTool(root, layout, this.o.lookupEnv ?? process.env)` is not null.

- [ ] **Step 1: Write the failing tests** (each `T`; `lookupEnv` is `{}` unless said):
  - "offers Build with the not-built notice": open `withHelper()` → `last("notice")` equals `{ type: "notice", message: NOT_BUILT, build: true }`.
  - "offers no Build without a tool": the existing "comes after the libraries' notice" project → `last("notice").build` is undefined.
  - "offers Build in a joined notice": `withHelper()` plus `karate-pom.json` `{ env: "qa" }` → the notice has `build: true`.
  - "builds with test-compile, then refreshes": open `withHelper()`; replace the stub with `mkdir "%~dp0target\test-classes" 2>nul\r\ntype nul>"%~dp0target\test-classes\Helper.class"\r\necho JAVA_HOME=%JAVA_HOME%>>"%~dp0calls.log"`; `await runner.build()` → last call is `-B -q test-compile`; no log line contains `vendor.jreDir`; a `project` event follows the build; no notice after it; `readBuilt(projectDir(dataDir, root))` is a number.
  - "builds in a folder whose path has spaces and &": as the listing's test, then `runner.build()` runs the stub once more.
  - "says the build's first error and keeps Build": stub `echo [ERROR] COMPILATION ERROR\r\nexit /b 1` → notice `{ message: "Grace could not build the project's Java code with Maven: COMPILATION ERROR. Building needs a JDK: set JAVA_HOME to one.", build: true }`; no `project` event after the click.
  - "says nothing of a JDK when JAVA_HOME has javac.exe": `lookupEnv = { JAVA_HOME: <dir with bin/javac.exe> }` → message ends with `COMPILATION ERROR.`
  - "stops the build on Cancel": stub `ping -n 4 127.0.0.1 >nul\r\necho late>>"%~dp0calls.log"`; cancel on the `listing` event with `build: true` → notice `{ message: "Build cancelled.", build: true }`; after 5 s no `late`; `readBuilt` null.
  - "says it is building, also in a replay": a `ping -n 2` stub → `all("listing")` is `[{ type: "listing", tool: "Maven", build: true }, { type: "listing", tool: null }]`; a replay taken during it contains the first.
  - "a Gradle build that rewrites no class still counts": `project("build.gradle")`, a `gradlew.bat` listing stub, `Helper.java` at 1_000_000_200 s, `build/classes/java/test/Helper.class` at 1_000_000_100 s → open says the Gradle CHANGED text; replace the stub with `rem`; `runner.build()` → call contains `--no-daemon` and `testClasses`; no notice after; set `Helper.java` to now + 10 s and `refresh()` → the CHANGED text again.
  - "refuses a second build while one runs": a `ping -n 2` stub; call `runner.build()` twice without awaiting the first → one `-B -q test-compile` call in the log, and a notice `A run is already in progress`.
  - "says when the tool is gone at the click": open `withHelper()`, delete `mvnw.cmd`, `runner.build()` → notice `Grace could not build the project's Java code with Maven: the project has no mvnw.cmd, and mvn is not on PATH, in MAVEN_HOME, in M2_HOME or in IntelliJ IDEA.` with no `build`.
  - "runs nothing for a standalone project or with no project": `runner.build()` on a standalone folder and on a fresh runner → stub log empty, no `busy` event.
- [ ] **Step 2: Run** `npx vitest run src/main/runner.libraries.test.ts`. Expected: the new tests fail (no `build`).
- [ ] **Step 3: Implement** the interfaces above. `listLibraries` keeps its behaviour and its tests.
- [ ] **Step 4: Run** the same file, then `npm run typecheck`. Expected: all pass.
- [ ] **Step 5: Commit** `feat(runner): build the project's java code from the notice`

### Task 3: The window: Build button and building text

**Files:**
- Modify: `src/renderer/reduce.ts`, `src/renderer/bannerText.ts`, `src/renderer/Notice.tsx`, `src/renderer/App.tsx`
- Test: `src/renderer/Notice.test.tsx`, `src/renderer/Banner.test.tsx`, `src/renderer/Progress.test.tsx`, `src/renderer/Tree.test.tsx`

**Interfaces:**
- Consumes: Task 2's event fields and `Api.build`.
- Produces:
  - `AppState.notice: { text: string; success: boolean; build: boolean } | null`; `AppState.building: boolean` (set from `listing` events, `e.build ?? false`).
  - `findingText`: `Building the project's Java code with <tool>…` when `s.listing && s.building`.
  - `Notice` takes optional `children`, shown after the text (a flex row that wraps).
  - `App`'s notice box shows `<button class={btnSmall}>Build</button>` when `notice().build`, disabled while `state.busy || state.run.active`, calling `props.api.build()`.

- [ ] **Step 1: Write the failing tests**
  - `Notice.test.tsx`: children render after the text, inside the same `role="status"` box.
  - `Banner.test.tsx`: `opened({ busy: true, listing: "Maven", building: true })` → `console-state` is `Building the project's Java code with Maven…`.
  - `Progress.test.tsx`: `{ type: "listing", tool: "Gradle", build: true }` → `run-outcome` is `Building the project's Java code with Gradle…`.
  - `Tree.test.tsx` (add `build: vi.fn(async () => {})` to its api): a loaded project plus `{ type: "notice", message: "x", build: true }` → a `Build` button inside `notice`; click → `api.build` called once; after `{ type: "busy", busy: true }` it is disabled; `{ type: "notice", message: "y" }` → no Build button.
- [ ] **Step 2: Run** `npx vitest run src/renderer/Notice.test.tsx src/renderer/Banner.test.tsx src/renderer/Progress.test.tsx src/renderer/Tree.test.tsx`. Expected: the new tests fail.
- [ ] **Step 3: Implement** the interfaces above.
- [ ] **Step 4: Run** the same command, plus `src/renderer/palette.test.ts`, then `npm run lint`. Expected: all pass.
- [ ] **Step 5: Commit** `feat(ui): show a build button in the not-built notice`

### Task 4: End-to-end test and README

**Files:**
- Modify: `e2e/project-setup.spec.ts`, `README.md`

- [ ] **Step 1: Write the e2e test** "the not-built notice builds the project's Java code": `write(project("maven"), { "src/test/java/examples/lib.feature": LIB_FEATURE, "src/test/java/examples/Helper.java": "class Helper {}", "mvnw.cmd": <stub> })` where the stub, after `@echo off`, writes an empty `%GRACE_CLASSPATH_FILE%` and exits when that variable is defined, else creates `%~dp0target\test-classes\Helper.class`. Launch, open; expect `notice` to contain `is not built`; click its `Build` button; expect `notice` to be hidden within 30 s and `target/test-classes/Helper.class` to exist.
- [ ] **Step 2: README** (spec R6.1): the project line adds that the not-built notice's Build button runs `test-compile` or `testClasses`; the "Nothing else to install" line says Grace runs the tests on its own Java, while the project's build tool, which lists libraries and builds Java code, uses the machine's.
- [ ] **Step 3: Run** `npm run lint` and `npm run typecheck`. Expected: pass. Ask the parent session for the go, then `npm run e2e`. Expected: all pass but the known #252 failure, if not fixed on `develop` yet.
- [ ] **Step 4: Commit** `test(e2e): build the project's java code from the notice` and `docs(readme): say the notice builds the project's java code`
