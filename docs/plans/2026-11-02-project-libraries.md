# Project Libraries Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Put a project's libraries on Karate's classpath: the Karate CLI's `.karate/ext` jars for every project, and a Maven or Gradle project's test libraries listed by its own build.

**Architecture:** `extensionJars` (command.ts) adds the CLI's two folders as `<folder>\*` entries. A new `src/main/karate/libraries.ts` holds the pure parts of listing (which tool, the key, the kept file, the command, parsing, the failure reason). The Runner runs the tool through `cmd.exe` on open and Refresh when the kept list is stale, as its one process at a time (so Cancel kills it), and every spawn reads the kept list. A `listing` event tells the window to say what it is doing.

**Tech Stack:** Electron main (Node 24, TypeScript), SolidJS 2 renderer, Vitest, Playwright.

**Spec:** `docs/specs/2026-11-02-project-libraries.md` (ADRs 0026, 0027).

## Global Constraints

- Every path the tool needs reaches `cmd.exe` through environment variables (`GRACE_TOOL`, `GRACE_CLASSPATH_FILE`, `GRACE_INIT_SCRIPT`), expanded by `cmd.exe` itself inside quotes: no path is ever written into the command line, so no quoting rule applies.
- Classpath order (R5.1): `karate.jar`, the debug listener jar in a debug run, the project's folders, `<home>\.karate\ext\*`, `<root>\.karate\ext\*`, then the kept libraries.
- Exact texts: `Listing the project's libraries with Maven…` (`Gradle`); `Grace could not list the project's libraries with <Maven|Gradle>: <reason>.` + ` Grace uses the libraries it listed before.` or ` Tests that use them will fail.`; reasons `the project has no mvnw.cmd and mvn is not on PATH`, `the project has no gradlew.bat and gradle is not on PATH`.
- No test reaches a real Maven or Gradle or the developer's own `~/.karate/ext`: the Runner takes `home` and `toolPath` options, `test/runner.ts` passes an empty temp folder and `""`; e2e's `launch()` drops the PATH folders that hold `mvn.cmd`, `mvn.bat` or `gradle.bat`.
- Solid 2 and UI rules of `AGENTS.md`; Conventional Commits, scope `runner`, `ui`, `docs` or `test`.
- Run single unit test files only (`npx vitest run <file>`). Full `npm test`, `npm run e2e` and anything packaged need the parent session's go (see the session brief).

## Review Focus

1. A project folder or home folder path with spaces, `&` or `(` in it: the tool must still run (env-variable expansion, Task 3 test uses a root under a folder named `a b&c`).
2. A kept list whose jar was deleted (a cleaned `~/.m2`): the next open re-lists (Task 2 `isStale` test).
3. A build file that changes while Grace is open, then a run: the run keeps the old list and starts no tool (Task 3 test).
4. A failed listing on open joined with the `karate-pom.json` notice: one notice, sentences each ending once with a full stop (Task 3 test).
5. A window reloaded while the tool runs: `replay()` includes the `listing` event (Task 3 test).

---

### Task 1: The Karate CLI's extension folders

**Files:**
- Modify: `src/main/karate/command.ts` (add `extensionJars`)
- Modify: `src/main/runner.ts` (options `home?: string`; `spawn` builds the classpath)
- Modify: `test/runner.ts` (`setup` passes `home`)
- Test: `src/main/karate/command.test.ts`, `src/main/runner.setup.test.ts`

**Interfaces:**
- Produces: `extensionJars(home: string, root: string): string[]` — `path.join(dir, "*")` for each of `path.join(home, ".karate", "ext")` then `path.join(root, ".karate", "ext")` that is an existing folder.
- Produces: Runner option `home?: string`, default `os.homedir()`; field `private readonly home: string`.
- Produces in `test/runner.ts`: `setup(..., opts?: { home?: string; toolPath?: string })` — defaults: a fresh empty temp folder (pushed to `temps`) and `""` (`toolPath` is used from Task 3; add it to the options type now, passed through).
- Produces in `test/runner.ts`: `makeJar(dir: string, files: Record<string, string>): string` — writes the files into a temp folder and zips them with `C:\Windows\System32\tar.exe --format zip -c -f <dir>\lib.jar -C <temp> <names…>`; returns the jar path.

- [ ] **Step 1: Write the failing tests**

In `command.test.ts`, `describe("extensionJars")`:
- `it("lists the home folder, then the project's, as wildcards")`: both `.karate/ext` folders exist → `[path.join(home, ".karate", "ext", "*"), path.join(root, ".karate", "ext", "*")]`.
- `it("skips a folder that does not exist")`: only the project's → one entry; neither → `[]`.

In `runner.setup.test.ts`:
- `it("puts the jars of the project's .karate/ext on the classpath")`: a standalone project with `t.feature` = `Feature: t\nScenario: ext\n  * match read('classpath:data.json').ext == true\n` and `makeJar(path.join(root, ".karate", "ext"), { "data.json": '{ "ext": true }' })`; open, `run("d:")`, `last("runFinished").summary` matches `{ scenarioCount: 1, scenariosPassed: 1 }`.
- `it("puts the jars of the home .karate/ext on the classpath")`: same feature, jar under `<home>/.karate/ext` with `setup(undefined, undefined, undefined, undefined, { home })`; passes.

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/main/karate/command.test.ts src/main/runner.setup.test.ts`
Expected: FAIL (`extensionJars` not exported; the runs fail on `read`).

- [ ] **Step 3: Implement**

`extensionJars` in `command.ts` (use `statSync(...).isDirectory()` in a try, as `isDir` in runner.ts does). In `Runner.spawn`, pass `classpath: [...classpath, ...extensionJars(this.home, root)]` to `buildArgs`. Do not change `detectLayout`: its classpath is also the bases of `locate`, which must never search jars.

- [ ] **Step 4: Run them to see them pass**

Run: `npx vitest run src/main/karate/command.test.ts src/main/runner.setup.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

`feat(runner): add the Karate CLI's .karate/ext jars to the classpath` (body: `Part of #64`).

---

### Task 2: The pure parts of a listing

**Files:**
- Create: `src/main/karate/libraries.ts`
- Test: `src/main/karate/libraries.test.ts`

**Interfaces:**
- Produces:
  - `type BuildLayout = "maven" | "gradle"`
  - `const TOOL_NAME: Record<BuildLayout, "Maven" | "Gradle">`
  - `interface KeptLibraries { tool: BuildLayout; key: string; jars: string[] }`
  - `findTool(root: string, layout: BuildLayout, searchPath: string): string | null` — R2.1/R2.2: `<root>\mvnw.cmd`, else the first `mvn.cmd` or `mvn.bat` in a `;`-separated `searchPath` folder (empty entries skipped); Gradle `gradlew.bat`, else `gradle.bat`.
  - `noToolReason(layout: BuildLayout): string` — the two reasons in Global Constraints.
  - `libraryKey(root: string, layout: BuildLayout): string` — R4.2: SHA-256 hex over, for each listed file in the spec's order, `<name>\0<content>\0`, or `<name>\0-\0` when absent (`-` cannot be told from a file whose content is `-`; acceptable).
  - `readKept(dir: string): KeptLibraries | null` — `<dir>\libraries.json`; null when missing, not JSON, or not that shape (`tool` one of the two, `key` a string, `jars` strings).
  - `writeKept(dir: string, kept: KeptLibraries): void` — `mkdirSync(dir, { recursive: true })` then write.
  - `isStale(kept: KeptLibraries | null, layout: BuildLayout, key: string): boolean` — R4.3.
  - `keptJars(dir: string, layout: Layout): string[]` — the kept jars when `kept.tool === layout`, else `[]`; never throws.
  - `TOOL_COMMAND: Record<BuildLayout, string>` — exactly:
    - maven: `"%GRACE_TOOL%" -q -B dependency:build-classpath "-Dmdep.outputFile=%GRACE_CLASSPATH_FILE%"`
    - gradle: `"%GRACE_TOOL%" -q --no-daemon --console=plain --no-configuration-cache --init-script "%GRACE_INIT_SCRIPT%" graceClasspath`
  - `toolEnv(base: NodeJS.ProcessEnv, o: { tool: string; file: string; initScript: string; searchPath: string; jreDir: string }): NodeJS.ProcessEnv` — `base` unchanged plus the three `GRACE_` variables; adds `JAVA_HOME = jreDir` only when `base.JAVA_HOME` is unset or empty and no `java.exe` is in a `searchPath` folder (R2.4).
  - `parseLibraries(text: string, classpath: string[]): string[]` — split on `;` and line breaks, trim, drop empties, keep entries that exist, drop those equal to a `classpath` entry (compare `path.resolve(x).toLowerCase()`), keep order, drop duplicates.
  - `failureReason(lines: string[]): string` — R6.3: first line starting with `[ERROR]` with that prefix and following spaces removed; else the first non-empty line after a line equal to `* What went wrong:` (trimmed); else the last non-empty line; cut to 300 characters; `"it stopped with no message"` when there is none.
  - `INIT_SCRIPT: string` — the Gradle init script:

```groovy
// Grace: the test runtime classpath of the root project (docs/specs/2026-11-02-project-libraries.md R3.2).
rootProject {
    tasks.register('graceClasspath') {
        def files = project.extensions.getByName('sourceSets').getByName('test').runtimeClasspath
        def out = new File(System.getenv('GRACE_CLASSPATH_FILE'))
        doLast { out.setText(files.files.join(File.pathSeparator), 'UTF-8') }
    }
}
```

- [ ] **Step 1: Write the failing tests** (`libraries.test.ts`, Node environment, temp folders)

- `findTool`: wrapper wins over PATH; `mvn.cmd` then `mvn.bat` on PATH in folder order; `gradlew.bat`, then `gradle.bat`; none → `null`; a `searchPath` of `""` finds nothing.
- `libraryKey`: same files → same key; changing `pom.xml`, adding `.mvn/maven.config`, or changing `gradle/libs.versions.toml` (Gradle) changes it; a file not in the layout's list (`README.md`, or `build.gradle` for Maven) does not.
- `readKept`/`writeKept`: round trip; `"nope"`, `{}`, `{ tool: "ant", key: "k", jars: [] }` → null.
- `isStale`: null → true; other tool → true; other key → true; a jar that does not exist → true; all match and exist → false.
- `keptJars`: `standalone` → `[]`; other tool → `[]`; same tool → the jars.
- `toolEnv`: keeps `base` keys; sets the three `GRACE_` keys; sets `JAVA_HOME` to `jreDir` only when unset and no `java.exe` on `searchPath`; never replaces a set `JAVA_HOME`.
- `parseLibraries`: `"a.jar;b.jar\r\n"` with both existing → both; a missing entry dropped; an entry equal to a classpath folder (other case) dropped; duplicates once; `""` → `[]`.
- `failureReason`: `["[INFO] x", "[ERROR] Could not resolve dependencies", "[ERROR] other"]` → `"Could not resolve dependencies"`; Gradle `["FAILURE: Build failed with an exception.", "", "* What went wrong:", "Task 'graceClasspath' not found in root project 'x'."]` → `"Task 'graceClasspath' not found in root project 'x'."`; `["a", "b", ""]` → `"b"`; a 400-character line → 300 characters; `[]` → `"it stopped with no message"`.

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/main/karate/libraries.test.ts`
Expected: FAIL (module not found).

- [ ] **Step 3: Implement `libraries.ts`** to the interfaces above.

- [ ] **Step 4: Run them to see them pass**

Run: `npx vitest run src/main/karate/libraries.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

`feat(runner): add the pure parts of listing a build's libraries` (body: `Part of #64`).

---

### Task 3: The Runner lists on open and Refresh

**Files:**
- Modify: `src/main/karate/process.ts` (`spawnKarate` gains `extra`; export `CMD`)
- Modify: `src/shared/events.ts` (the `listing` event)
- Modify: `src/main/runner.ts`
- Modify: `test/runner.ts` (`toolPath`; `stubTool`)
- Test: `src/main/runner.libraries.test.ts` (new), `src/main/karate/process.test.ts`

**Interfaces:**
- Consumes: everything Task 2 produces; `extensionJars` (Task 1).
- Produces:
  - `spawnKarate(java, args, cwd, onLines, killWaitMs = 5000, extra: { env?: NodeJS.ProcessEnv; windowsVerbatimArguments?: boolean } = {})` — `env` defaults to `childEnv(process.env)`; `windowsVerbatimArguments` passed to `spawn`.
  - `export const CMD = path.join(system32, "cmd.exe")` in `process.ts`.
  - `AppEvent` gains `{ type: "listing"; tool: "Maven" | "Gradle" | null }` — "Grace is running the project's build tool to list its libraries, or null." Sent around each listing and in every `replay()` (after `busy`).
  - Runner option `toolPath?: string`, default `process.env.PATH ?? ""`.
  - Runner `private async listLibraries(root: string, layout: Layout): Promise<string | null>` — the failure notice, or null (listed, kept list still good, standalone, or cancelled).
  - Module helper `projectDirOf(dataDir: string, root: string): string`; `projectDir()` returns `projectDirOf(this.o.dataDir, this.project!.root)`.
  - Module helper `joinNotices(parts: string[]): string` — the expression at today's line 933 (`said.map((m) => (said.length > 1 && !m.endsWith(".") ? `${m}.` : m)).join(" ")`), used there and by `open`.
  - In `test/runner.ts`: `stubTool(root: string, name: "mvnw.cmd" | "gradlew.bat", body: string): string` — writes `@echo off\r\necho %*>>"%~dp0calls.log"\r\n<body>` (CRLF lines) into `<root>\<name>`; returns the path of `calls.log`. And `calls(log: string): string[]` — its lines, `[]` when missing.

`listLibraries` flow:
1. `standalone` → null. `dir = projectDirOf(...)`, `key = libraryKey(...)`, `kept = readKept(dir)`; not `isStale` → null.
2. `said(reason)` builds the notice of Global Constraints; the second sentence depends on `kept?.tool === layout`.
3. `findTool(root, layout, this.toolPath)`; null → `said(noToolReason(layout))`.
4. `mkdirSync(dir)`; delete `libraries.txt`; for Gradle write `INIT_SCRIPT` to `<dir>\libraries.gradle`.
5. Emit `listing` with `TOOL_NAME[layout]`; `this.proc = spawnKarate(CMD, ["/d", "/s", "/c", `"${TOOL_COMMAND[layout]}"`], root, collect lines, 5000, { env: toolEnv(process.env, …), windowsVerbatimArguments: true })`; await `exit`; `this.proc = null`; emit `listing` null (in a `finally`).
6. Cancelled → null. Read and delete `libraries.txt`; exit code not 0 or no file → `said(failureReason(lines))`. Else `writeKept(dir, { tool: layout, key, jars: parseLibraries(text, detectLayout(root).classpath) })`, null.

Wiring:
- `open`: inside `guarded`, before `rediscover`: `const libraries = await this.listLibraries(root, detectLayout(root).layout);`. The notice parts become `[properties text, pomNotice, libraries]`, joined with `joinNotices`.
- `refresh`: inside `guarded`: list, then `rediscover`; if it returned true and a notice came back, emit it after.
- `discover`: first line `if (this.cancelled) return null;` (a Cancel during the listing must not start the dry run; `rediscover`'s cancel path then restores the tree and says `Cancelled`).
- `spawn`: classpath `[...classpath, ...extensionJars(this.home, root), ...keptJars(this.projectDir(), layout)]`.
- `switchEnv`, runs, re-runs and debug runs never call `listLibraries` (R4.4).

- [ ] **Step 1: Write the failing tests** (`runner.libraries.test.ts`; each project is a temp folder with `pom.xml` `<project/>` and `src/test/java/t.feature` reading `classpath:data.json` as in Task 1; the stub's body writes `makeJar(...)`'s path: `>"%GRACE_CLASSPATH_FILE%" echo <jar>`)

- `it("runs mvnw.cmd and puts the listed jar on the classpath")`: open, `calls(log)` has one line containing `dependency:build-classpath` and `-Dmdep.outputFile=` followed by `libraries.txt`'s path; `run("d:")` passes.
- `it("runs gradlew.bat without a daemon")`: `build.gradle` (empty) and a `gradlew.bat` stub; one call containing `--no-daemon`, `--init-script`, the path of `libraries.gradle`, `graceClasspath`; the file holds `INIT_SCRIPT`; the run passes.
- `it("keeps the list until a build file changes")`: open; `refresh()` → still 1 call; change `pom.xml`; `refresh()` → 2 calls.
- `it("never lists on an environment switch or a run")`: open; change `pom.xml`; `setEnv` to `"qa"` (with a `karate-config-qa.js` in `src/test/java`), `run("d:")` → still 1 call.
- `it("says when there is no tool")`: no stub; open → `last("notice").message` is `"Grace could not list the project's libraries with Maven: the project has no mvnw.cmd and mvn is not on PATH. Tests that use them will fail."`; the tree has the feature.
- `it("keeps the last list when the tool fails")`: list once; stub body becomes `echo [ERROR] Could not resolve dependencies\r\nexit /b 1`; change `pom.xml`; `refresh()` → notice `"Grace could not list the project's libraries with Maven: Could not resolve dependencies. Grace uses the libraries it listed before."`; `run("d:")` passes.
- `it("joins the open notice")`: no stub and a `karate-pom.json` `{ "env": "qa" }` → notice `"Grace does not use these karate-pom.json settings: env. Grace could not list the project's libraries with Maven: the project has no mvnw.cmd and mvn is not on PATH. Tests that use them will fail."`
- `it("runs the tool in a folder whose path has spaces and &")`: project under `mkdtemp(...)/a b&c/`; listed and passes.
- `it("says it is listing, also in a replay")`: `onEvent` records; while the stub runs (body `ping -n 3 127.0.0.1 >nul` then the echo), `runner.replay()` contains `{ type: "listing", tool: "Maven" }`; afterwards the events hold `listing` Maven then null.
- `it("stops the tool on Cancel")`: stub body `ping -n 4 127.0.0.1 >nul\r\necho late>>"%~dp0calls.log"`; `cancel()` once `listing` Maven is emitted; `last("notice").message` is `"Cancelled"`; after 5 s `calls(log)` has no `late`; `readKept(...)` is null.
- `it("runs no tool for a standalone project")`: a `mvnw.cmd` stub in a project with no `pom.xml` → no call.

In `process.test.ts`: `it("passes the given environment and verbatim arguments")`: `spawnKarate(CMD, ["/d", "/s", "/c", '"echo "%GRACE_X%""'], tmpdir(), …, 5000, { env: { ...process.env, GRACE_X: "a b&c" }, windowsVerbatimArguments: true })` prints `"a b&c"` (an expanded `&` is an operator unless quoted: every variable in `TOOL_COMMAND` sits inside quotes).

The jar a stub echoes lives in a plain temp folder (no space or `&`): the stub's `echo` is not quoted.

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/main/runner.libraries.test.ts src/main/karate/process.test.ts`
Expected: FAIL.

- [ ] **Step 3: Implement** the interfaces and wiring above. `test/runner.ts` `setup` passes `toolPath: ""` by default.

- [ ] **Step 4: Run them, and the other runner files that open a Maven project, to see them pass**

Run: `npx vitest run src/main/runner.libraries.test.ts src/main/karate/process.test.ts src/main/runner.setup.test.ts src/main/runner.discovery.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

`feat(runner): list a Maven or Gradle project's libraries with its build` (body: `Part of #64`).

---

### Task 4: The window says it is listing

**Files:**
- Modify: `src/renderer/reduce.ts` (state `listing: "Maven" | "Gradle" | null`, initial null, set by the `listing` event)
- Modify: `src/renderer/bannerText.ts` (add `findingText`)
- Modify: `src/renderer/Banner.tsx`, `src/renderer/Progress.tsx`, `src/renderer/Tree.tsx`
- Test: `src/renderer/store.test.tsx` or the reduce tests beside it, `src/renderer/Progress.test.tsx`, `src/renderer/Tree.test.tsx`, the banner tests

**Interfaces:**
- Consumes: the `listing` event (Task 3).
- Produces: `findingText(s: AppState): string` — `Listing the project's libraries with ${s.listing}…` when `s.listing`, else `Finding tests…`.

- [ ] **Step 1: Write the failing tests**: the reduce sets and clears `listing`; with `busy` and `listing: "Maven"`, the progress area, the tree's message (no tree yet) and the banner's state line each read `Listing the project's libraries with Maven…`; with `listing: null` they still read `Finding tests…`.
- [ ] **Step 2: Run them to see them fail** (`npx vitest run` on those files).
- [ ] **Step 3: Implement**: the three places take their text from `findingText`.
- [ ] **Step 4: Run them to see them pass.**
- [ ] **Step 5: Commit** `feat(ui): say when Grace lists the project's libraries` (body: `Part of #64`).

---

### Task 5: End-to-end tests and docs

**Files:**
- Modify: `e2e/helpers.ts` (`launch()` env)
- Modify: `e2e/project-setup.spec.ts`
- Modify: `README.md` (line 44, the project line; line 190, the roadmap)
- Modify: `AGENTS.md` (Karate integration rules)

- [ ] **Step 1: Shield e2e from real build tools.** In `launch()`, build the env from `process.env` with every key matching `/^path$/i` removed, then `PATH` set to the folders of the original that hold none of `mvn.cmd`, `mvn.bat`, `gradle.bat`.
- [ ] **Step 2: Write the e2e tests** in `project-setup.spec.ts`:
  - `test("a Maven project's libraries come from its build")`: `project("maven")` plus a `mvnw.cmd` stub echoing a jar made with `tar.exe` (as `makeJar`) and a feature reading `classpath:data.json`; open; run that scenario; `data-status` is `passed`.
  - `test("jars in .karate/ext are on the classpath")`: a standalone temp project with the jar in `.karate/ext`; same assertion.
- [ ] **Step 3: Docs.** README project line: after "Karate CLI projects with a `karate-pom.json`.", add: "A Maven or Gradle project's libraries come from its own build (`mvnw`, `gradlew`, or `mvn` or `gradle` on PATH), and jars in `.karate/ext/` or `~/.karate/ext/` are on the classpath, as with the Karate CLI." Roadmap line: "**More project setup**: multi-module Maven and Gradle projects ([#64](https://github.com/hpagniez/grace/issues/64))." `AGENTS.md`, first bullet of Karate integration rules: "Follow the Karate CLI first: for every feature and every kind of project, do what the Karate CLI does, and build Grace's own way only where it has none ([ADR 0026](docs/decisions/0026-follow-the-karate-cli-first.md))."
- [ ] **Step 4: Ask the parent session for a go, then run** `npm run vendor`, `npm test` and `npm run e2e`. Expected: all pass. Keep the e2e summary for the PR.
- [ ] **Step 5: Commit** `test(e2e): check the project libraries end to end` and `docs: say where Grace takes a project's libraries from` (bodies: `Part of #64`).

---

### Task 6: Checks with the real tools, then the PR

- [ ] **Step 1:** With the parent's go, `npm run package` (or `electron-forge package`) and, in the packaged app, open the scratch SQLite Maven project (Maven 3.9.16 from the scratch folder on PATH, `-Dmaven.repo.local` through `MAVEN_OPTS` to the scratch repository): the SQLite scenario passes. Note the listing's time and peak memory.
- [ ] **Step 2:** Same with a Gradle 9 project (Gradle downloaded to the scratch folder, `GRADLE_USER_HOME` in the scratch folder): passes; no `java.exe` from Gradle left afterwards.
- [ ] **Step 3:** Write `docs/checks/2026-11-02-project-libraries.md`: what was run, the machine, the results, the numbers, and "Needs the owner: their work suite".
- [ ] **Step 4:** Push `feature/project-libraries`, open a draft PR into `develop` (`Part of #64`, e2e summary, checks), read the `perf` comment and explain any worse measure in the body. After it merges, #64's `Spec:` line gains the new spec.
