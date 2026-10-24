# Walking Skeleton Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first usable karate-kid: open a Karate project, see its tests from a dry run, run any node, watch live status, read step details and the console, cancel, open the report.

**Architecture:** The Electron main process owns everything Karate: it spawns the bundled Java without a shell, polls `karate-events.jsonl` every 100 ms, and turns Karate events into a small set of typed `AppEvent`s sent to the window. The window holds a Solid store fed only by those events and renders one screen (toolbar, progress, tree, details, console).

**Tech Stack:** Electron 45 + Forge 8 (Vite plugin), Solid 2 RC, Tailwind v4, TypeScript 6, Vitest 5, Playwright 1.64 (`_electron`), Temurin JRE 21.0.12.1+1, Karate 2.1.4.

**Spec:** [docs/specs/2026-10-24-walking-skeleton.md](../specs/2026-10-24-walking-skeleton.md). Read it first; requirement ids (R1–R8) below refer to it.

## Global Constraints

- Branch: `feature/walking-skeleton`, cut from `develop` after PR #3 is merged. Conventional Commits, one task per commit or more. Rules in [AGENTS.md](../../AGENTS.md), including the Solid 2 rules.
- Exact versions, no `^`/`~`. Start from the [stack spike package.json](../research/2026-10-24-stack-spike/package.json): `electron` 45.0.0, `@electron-forge/*` 8.0.1, `vite` 8.3.5, `typescript` 6.0.3, `solid-js` and `@solidjs/web` 2.0.0-rc.18, `@solidjs/vite-plugin` 3.0.0-next.51, `tailwindcss` and `@tailwindcss/vite` 4.3.3, `vitest` 5.0.5, `jsdom` 30.1.3, `@solidjs/testing-library` 1.0.0-rc.2, `@playwright/test` 1.64.1.
- Do not add `@tanstack/solid-router` or `@kobalte/core` yet: the MVP has one screen and native buttons. They come with the second screen (ADR 0001 stays in force).
- Windows 11 x64 only. JRE `21.0.12.1+1`, `karate-2.1.4.jar`, URLs and SHA-256 values exactly as in the spec's "Bundled Java and Karate" section.
- Karate is spawned only via `src/main/karate/process.ts`, never with `shell: true`, with `cwd` = project root, flags exactly as the spec's "Spawning Karate" block, and `KARATE_OPTIONS`, `JAVA_TOOL_OPTIONS`, `JDK_JAVA_OPTIONS`, `_JAVA_OPTIONS` removed from the env.
- Cancel and shutdown kill with `taskkill /PID <pid> /T /F`.
- `contextIsolation: true`, `nodeIntegration: false`, `sandbox: true`. Keep the Forge fuses from the stack spike unchanged.
- Event-parsing tests use real Karate output (spike samples or fixtures captured by `scripts/capture-fixtures.mjs`), never hand-written JSON.
- Paths inside the app model use `/` and are relative to the project root. Only `command.ts` and `process.ts` turn them into absolute Windows paths.

## Review Focus

1. **Project path with spaces and non-ASCII** (`D:\Mes tests\équipe`): discovery and runs must work. Test in Task 7 (`opens a project whose path has spaces and accents`).
2. **Very chatty run** (thousands of console lines): the window must stay responsive and keep only the last 10,000 lines. Test in Task 9 (`console keeps the last 10000 lines`); batching per poll in Task 7.
3. **Second Run click while a run or discovery is going**: must not start a second Java. Test in Task 7 (`rejects a run while another is active`).
4. **Last project folder deleted or moved**: app starts with no project, no crash; opening a missing folder shows an error. Tests in Task 7 (`open on a missing folder emits an error`) and Task 8 (`loadLastProject ignores a missing folder`).
5. **Duplicate scenario names in one feature**: the name fallback after an edit must refuse rather than guess. Test in Task 4 (`resolveNode returns null when the name is ambiguous`).

---

### Task 1: Scaffold the app

**Files:**
- Create: `package.json`, `package-lock.json`, `forge.config.mts`, `vite.main.config.mts`, `vite.preload.config.mts`, `vite.renderer.config.mts`, `tsconfig.json`, `index.html`, `vitest.config.mts`, `playwright.config.ts`, `.gitignore`, `.gitattributes`, `src/main.ts`, `src/preload.ts`, `src/renderer.tsx`, `src/index.css`, `src/declarations.d.ts`, `src/shared/sanity.test.ts`
- Copy from: [stack spike](../research/2026-10-24-stack-spike/), [Vitest spike config](../research/2026-10-24-vitest-spike/vitest.config.mts), [Playwright spike config](../research/2026-10-24-playwright-spike/playwright.config.ts)

**Interfaces:**
- Produces: npm scripts `start`, `package`, `typecheck`, `lint`, `test` (`vitest run`), `e2e` (`npm run package && npx electron --version && playwright test`), `vendor` (`node scripts/fetch-vendor.mjs`), `fixtures` (`node scripts/capture-fixtures.mjs`). Entry points stay `src/main.ts`, `src/preload.ts`, `src/renderer.tsx`.

- [ ] **Step 1:** Copy the stack spike files listed above to the repo root. In `package.json`: name `karate-kid`, productName `karate-kid`, drop `electron-squirrel-startup`, `@types/electron-squirrel-startup`, `@kobalte/core`, `@tanstack/solid-router`, the `overrides` block, and makers deb/rpm/squirrel; keep `@electron-forge/maker-zip` and set it to `['win32']`. Add the test dev dependencies from Global Constraints and the scripts above.
- [ ] **Step 2:** `src/main.ts`: remove the squirrel import, the tick timer, the autocheck and the logging. Keep window creation with the three security flags. `src/renderer.tsx`: render an empty `<main>` with the text `karate-kid`. `src/preload.ts`: expose an empty `api` object for now.
- [ ] **Step 3:** `.gitignore`: `node_modules/`, `out/`, `.vite/`, `vendor/`, `test-results/`, `playwright-report/`. `.gitattributes`: `* text=auto eol=lf`.
- [ ] **Step 4:** Write `src/shared/sanity.test.ts`: `test('vitest runs', () => expect(1 + 1).toBe(2))`. Run `npm install`, then `npm test` → 1 passed. Run `npm run typecheck` → no errors.
- [ ] **Step 5:** Run `npm start` → a window opens showing `karate-kid`; close it.
- [ ] **Step 6:** Commit: `build: scaffold electron forge app with solid 2, vitest and playwright`.

### Task 2: Bundled Java, Karate and test fixtures

**Files:**
- Create: `scripts/fetch-vendor.mjs`, `scripts/fetch-vendor.test.ts`, `scripts/capture-fixtures.mjs`, `src/main/vendor.ts`, `test/fixtures/projects/clean/**`, `test/fixtures/projects/broken-extra/broken/syntax.feature`, `test/fixtures/projects/maven/**`, `test/fixtures/events/*.jsonl`, `test/fixtures/copy.ts`

**Interfaces:**
- Produces:
  - `vendor/jre/bin/java.exe`, `vendor/karate.jar`, `vendor/licenses/temurin/*`, `vendor/licenses/karate-LICENSE.txt`
  - `src/main/vendor.ts`: `export interface VendorPaths { java: string; jar: string; jreDir: string }` and `export function vendorPaths(base: string): VendorPaths` (base = `<repo>/vendor` in dev and tests, `process.resourcesPath` when packaged)
  - `scripts/fetch-vendor.mjs` exports `verifySha256(file: string, expected: string): Promise<void>` (throws `Error` naming the file and both hashes on mismatch)
  - `test/fixtures/copy.ts`: `export function copyFixture(name: 'clean' | 'broken' | 'maven'): string` copies to a fresh temp dir and returns it; `broken` = `clean` plus `broken-extra/broken/syntax.feature`
  - Event fixtures: `test/fixtures/events/dryrun-clean.jsonl`, `run-clean.jsonl`, `dryrun-outline-sizes.jsonl`, plus copies of the four [spike samples](../research/2026-10-24-karate-cli-spike/samples/)

- [ ] **Step 1:** Write `scripts/fetch-vendor.test.ts`: `verifySha256` resolves for a temp file whose hash matches, and rejects with a message containing both hashes when it does not.
- [ ] **Step 2:** Run `npx vitest run scripts/fetch-vendor.test.ts` → FAIL (module not found).
- [ ] **Step 3:** Implement `scripts/fetch-vendor.mjs` with Node built-ins only (`fetch`, `node:crypto`, `node:fs`). Download JRE zip and jar to `vendor/.download/`, verify, unzip the JRE with `tar -xf` (bsdtar ships with Windows 11) and rename the single top folder to `vendor/jre`, copy the jar to `vendor/karate.jar`. Copy the JRE's `legal/` and `NOTICE`/`LICENSE` files to `vendor/licenses/temurin/`. Download Karate's `LICENSE` from `https://raw.githubusercontent.com/karatelabs/karate/v2.1.4/LICENSE`, compute its SHA-256 once, and pin it in the script like the other two. Skip downloads whose verified output already exists.
- [ ] **Step 4:** Run the test → PASS. Run `npm run vendor` → `vendor/jre/bin/java.exe -version` prints `21.0.12`.
- [ ] **Step 5:** Build `test/fixtures/projects/clean/` from the [spike project](../research/2026-10-24-karate-cli-spike/project/): `tests/**`, `karate-config.js`, `karate-config-dev.js`, `start-mock.feature`, `mocks/api-mock.feature` (add `@ignore` above its `Feature:` line). `broken-extra/broken/syntax.feature` from the spike's `broken/`. `maven/`: `pom.xml` (`<project/>` is enough), `src/test/java/karate-config.js` returning `{ fromConfig: 'cfg' }`, `src/test/resources/data/user.json` = `{ "name": "kid" }`, `src/test/java/common/helper.feature` (`@ignore`, defines `greeting = 'hello ' + who`), `src/test/java/examples/users.feature` with one scenario asserting `fromConfig == 'cfg'`, `read('classpath:data/user.json').name == 'kid'` and `(call read('classpath:common/helper.feature') { who: 'x' }).greeting == 'hello x'`.
- [ ] **Step 6:** Implement `scripts/capture-fixtures.mjs`: runs the bundled Java per the spec command (classpath = project root, `-B false`) and writes `dryrun-clean.jsonl` (`-D` on clean `tests`), `run-clean.jsonl` (full run on clean), `dryrun-outline-sizes.jsonl` (`-D` on the spike's `extra/outline-sizes.feature`). Replace the absolute fixture root in every line with `C:\\fixture` before writing. Copy the four spike samples next to them. Run `npm run fixtures`.
- [ ] **Step 7:** Verify the spec's `@ignore` question: in `run-clean.jsonl`, the `SUITE_EXIT` summary has `scenarioCount` 9, `scenariosPassed` 8, `scenariosFailed` 1, and no depth-0 events for `start-mock.feature` or `mocks/`. If not, stop and report: the spec's fixture assumption is wrong.
- [ ] **Step 8:** Implement `vendorPaths` and `copyFixture`. Commit: `build: fetch bundled jre and karate, add test fixtures`.

### Task 3: Karate events and the JSONL tail

**Files:**
- Create: `src/main/karate/events.ts`, `src/main/karate/events.test.ts`

**Interfaces:**
- Produces:
  ```ts
  export interface KarateEvent { type: string; timeStamp: number; threadId: string | null; data: Record<string, any> }
  export class JsonlTail {
    constructor(file: string);
    read(): KarateEvent[]; // new complete lines since last call; [] if the file does not exist yet
  }
  export function readAllEvents(file: string): KarateEvent[];
  ```

- [ ] **Step 1:** Tests in `events.test.ts`:
  - `returns [] before the file exists`
  - `returns only complete lines and keeps a partial last line`: write 2 lines + half a line to a temp file → `read()` returns 2; append the rest + `\n` → returns 1; next `read()` → `[]`
  - `parses a real event stream`: `readAllEvents('test/fixtures/events/run-clean.jsonl')` contains exactly one `SUITE_ENTER` and one `SUITE_EXIT`, and 9 depth-0 `SCENARIO_EXIT`
  - `handles a file that ends without EXIT events`: the post-`taskkill` sample parses with no throw and its last event is `SCENARIO_ENTER`
- [ ] **Step 2:** Run `npx vitest run src/main/karate/events.test.ts` → FAIL.
- [ ] **Step 3:** Implement with `fs.openSync`/`fs.readSync` from a stored byte offset; decode as UTF-8 only up to the last `\n` so a split multi-byte character is never decoded early.
- [ ] **Step 4:** Run → PASS. Commit: `feat(runner): tail karate event stream`.

### Task 4: Test tree

**Files:**
- Create: `src/shared/model.ts`, `src/main/karate/tree.ts`, `src/main/karate/tree.test.ts`

**Interfaces:**
- Produces in `src/shared/model.ts` (used by main and renderer):
  ```ts
  export type Layout = 'maven' | 'gradle' | 'standalone';
  export type NodeKind = 'folder' | 'feature' | 'outline' | 'scenario' | 'row';
  export type Status = 'notRun' | 'pending' | 'running' | 'passed' | 'failed' | 'interrupted';
  export interface TestNode {
    key: string; kind: NodeKind; name: string;
    file: string;          // folder or feature path relative to the project, '/' separators; '' for the root
    line: number | null;   // line to run: scenario line, outline line, or row line; null for folders/features
    tags: string[];        // without '@'
    children: TestNode[];
  }
  ```
- Produces in `tree.ts`:
  ```ts
  export function parseRefId(refId: string): { section: number; example: number | null; line: number };
  export function buildTree(events: KarateEvent[], rootName: string): TestNode; // root: kind 'folder', key 'd:', file ''
  export function findNode(tree: TestNode, key: string): TestNode | null;
  export function leafKeys(node: TestNode): string[]; // scenario and row keys under node, in tree order
  export function resolveNode(tree: TestNode, old: TestNode): TestNode | null;
  ```
- Key format: folder `d:<file>`, feature `f:<file>`, outline `o:<file>#<section>`, scenario `s:<file>#<section>`, row `s:<file>#<section>.<example>`.

- [ ] **Step 1:** Tests in `tree.test.ts`, fed by fixtures:
  - `parseRefId('[1.3:7]')` → `{ section: 1, example: 3, line: 7 }`; `parseRefId('[3:22]')` → `{ section: 3, example: null, line: 22 }`
  - `builds the clean project tree`: from `dryrun-clean.jsonl`: root has folder `d:tests` with features `f:tests/basic.feature`, `f:tests/outline.feature`, `f:tests/slow.feature` (sorted by name); `leafKeys(root).length === 9`; no node file starts with `tests/common/`
  - `groups outline rows`: outline `o:tests/outline.feature#1` named `add <a> and <b>` with line 3 and 3 rows; the row named `add 2 and 3` has key `s:tests/outline.feature#1.2` and line 10 (Examples line 7 + 2 + exampleIndex 1)
  - `handles two Examples tables`: from `dryrun-outline-sizes.jsonl`, the second outline has 7 rows and rows of its second table get lines from that table's own `Examples:` line
  - `keeps scenario tags`: `failing match` has tags `['wip']`
  - `resolveNode finds a scenario that moved`: build a second tree from the same events with every `basic.feature` section number +1 and line +5 → `resolveNode(newTree, oldFailingMatch)` returns the node named `failing match`
  - `resolveNode returns null when the name is ambiguous`: two scenarios with the old name in the feature → `null`
  - `resolveNode returns null when the scenario is gone`
- [ ] **Step 2:** Run `npx vitest run src/main/karate/tree.test.ts` → FAIL.
- [ ] **Step 3:** Implement. Use only depth-0 events. A row belongs to the latest `OUTLINE_ENTER` seen earlier in the same feature (spec R2). Row line = row `line` + 2 + `exampleIndex`. Folder nodes come from the feature paths. Within a feature keep file order; sort folders and features by name. `resolveNode`: same key and same name → that node; else exactly one node of the same kind in the same feature with the same name → that node; else `null`.
- [ ] **Step 4:** Run → PASS. Commit: `feat(runner): build test tree from dry run events`.

### Task 5: Run state and step details

**Files:**
- Create: `src/shared/events.ts`, `src/main/karate/run-state.ts`, `src/main/karate/run-state.test.ts`, `src/main/karate/steps.ts`, `src/main/karate/steps.test.ts`

**Interfaces:**
- Consumes: `KarateEvent` (Task 3), `TestNode`, `buildTree`, `leafKeys`, key format (Task 4).
- Produces in `src/shared/events.ts`:
  ```ts
  export interface Step { keyword: string; text: string; line: number; status: string; millis: number; background: boolean; log: string | null }
  export interface ScenarioUpdate { key: string; status: Status; durationMillis?: number; error?: string }
  export interface Summary { scenarioCount: number; scenariosPassed: number; scenariosFailed: number; durationMillis: number }
  export type RunOutcome = 'finished' | 'cancelled' | 'failed';
  export interface Project { root: string; layout: Layout }
  export type AppEvent =
    | { type: 'project'; project: Project | null; tree: TestNode | null; error: string | null }
    | { type: 'runStarted'; selection: string; keys: string[]; total: number; startedAt: number }
    | { type: 'scenarios'; updates: ScenarioUpdate[] }
    | { type: 'steps'; key: string; steps: Step[] }
    | { type: 'console'; lines: string[] }
    | { type: 'runFinished'; outcome: RunOutcome; exitCode: number | null; summary: Summary | null; reportAvailable: boolean }
    | { type: 'notice'; message: string };
  ```
- Produces:
  ```ts
  // steps.ts
  export function extractSteps(featureExitData: Record<string, any>): { key: string; steps: Step[] }[];
  // run-state.ts
  export class RunState {
    constructor(tree: TestNode, selection: TestNode);
    readonly keys: string[]; // leafKeys(selection)
    apply(events: KarateEvent[]): AppEvent[]; // 'scenarios' and 'steps' events, at most one 'scenarios' per call
    finish(exitCode: number | null, cancelled: boolean, reportAvailable: boolean): AppEvent[]; // interrupted updates + 'runFinished'
  }
  ```

- [ ] **Step 1:** Tests in `steps.test.ts` (fixture `run-clean.jsonl`): the `FEATURE_EXIT` of `tests/basic.feature` yields 4 entries keyed with Task 4's format; `failing match` has a step with keyword `And`, text starting `match response.message`, status `failed`; the `method get` step of `passing hello` has a `log` containing `GET` and `200`.
- [ ] **Step 2:** Tests in `run-state.test.ts`:
  - `marks running then passed or failed`: tree from `dryrun-clean.jsonl`, selection = root, apply `run-clean.jsonl` events → final statuses: 8 `passed`, 1 `failed` (`failing match`), its update carries `error` starting with the feature path and `:20`
  - `ignores called features`: no update key contains `common/create-user`
  - `emits steps when a feature ends`
  - `finish after cancel interrupts running scenarios`: apply the post-`taskkill` sample, `finish(1, true, false)` → the open scenario becomes `interrupted`, last event `runFinished` with outcome `cancelled`
  - `finish without SUITE_EXIT is a failed run`: `finish(3, false, false)` → outcome `failed`
  - `finish with SUITE_EXIT carries the summary`: outcome `finished`, summary `{ scenarioCount: 9, scenariosPassed: 8, scenariosFailed: 1, ... }`
- [ ] **Step 3:** Run both test files → FAIL.
- [ ] **Step 4:** Implement. Map an event to a key from `data.feature` + section/example of `data.refId`; skip `callDepth !== 0`. Status: `SCENARIO_ENTER` → `running`; `SCENARIO_EXIT` → `failed` if `!passed`, `passed` otherwise. Outcome per spec R4.
- [ ] **Step 5:** Run → PASS. Commit: `feat(runner): track run status and extract steps`.

### Task 6: Karate command line and process

**Files:**
- Create: `src/main/karate/command.ts`, `src/main/karate/command.test.ts`, `src/main/karate/process.ts`, `src/main/karate/process.test.ts`

**Interfaces:**
- Consumes: `TestNode`, `Layout` (Task 4), `VendorPaths` (Task 2).
- Produces:
  ```ts
  // command.ts
  export interface ProjectLayout { layout: Layout; classpath: string[]; featureRoots: string[] } // absolute paths
  export function detectLayout(root: string): ProjectLayout;
  export function selectionTargets(node: TestNode, root: string, featureRoots: string[]): string[];
  export function buildArgs(o: { jar: string; classpath: string[]; root: string; outDir: string; dryRun: boolean; targets: string[] }): string[];
  export function childEnv(env: NodeJS.ProcessEnv): NodeJS.ProcessEnv;
  // process.ts
  export interface KarateProcess { pid: number; exit: Promise<number | null>; kill(): Promise<void> }
  export function spawnKarate(java: string, args: string[], cwd: string, onLines: (lines: string[]) => void): KarateProcess;
  export function killOrphan(pidFile: string, jreDir: string): Promise<void>;
  ```

- [ ] **Step 1:** Tests in `command.test.ts`:
  - `detectLayout`: temp dir with `pom.xml` + both `src/test/*` folders → `maven`, classpath and featureRoots = both folders; `build.gradle.kts` → `gradle`; only `src/test/java` present → only that folder; no build file → `standalone`, both lists = `[root]`
  - `selectionTargets`: root `d:` → featureRoots; folder → absolute folder; feature → absolute file; scenario, outline, row → `<absolute file>:<line>`
  - `buildArgs` equals exactly: `['-Dstdout.encoding=UTF-8', '-Dstderr.encoding=UTF-8', '-cp', '<jar>;<cp1>;<cp2>', 'io.karatelabs.Main', 'run', '--no-color', '-w', root, '-o', outDir, '-B', 'false', '-f', 'html,karate:jsonl', ...(dryRun ? ['-D'] : []), ...targets]`
  - `childEnv` removes the 4 variables (any letter case) and keeps `PATH`
- [ ] **Step 2:** Tests in `process.test.ts` (real Java, `vendor/` required; 60 s timeout):
  - `runs karate and streams console lines`: dry run on a `copyFixture('clean')` → exit 0, some line contains `DRY RUN`
  - `kill leaves no java behind`: full run of `tests/slow.feature`, call `kill()` 1.5 s after start → `exit` resolves; `tasklist /FI "PID eq <pid>"` no longer lists it
  - `killOrphan kills only our java`: start a run, write its PID to a temp pid file, `killOrphan(pidFile, jreDir)` → process gone and pid file deleted; with a pid file holding a non-java PID (e.g. `process.pid`) → that process untouched, pid file deleted
- [ ] **Step 3:** Run both → FAIL.
- [ ] **Step 4:** Implement. `spawnKarate`: `child_process.spawn(java, args, { cwd, env: childEnv(process.env), shell: false, windowsHide: true })`; read stdout and stderr with `readline`, buffer lines and flush to `onLines` every 100 ms. `kill`: `taskkill /PID <pid> /T /F`, then await `exit`. `killOrphan`: read the PID, get the process executable path with PowerShell `(Get-CimInstance Win32_Process -Filter "ProcessId=<pid>").ExecutablePath`, kill only if it is inside `jreDir`; always delete the pid file.
- [ ] **Step 5:** Run → PASS. Commit: `feat(runner): build karate command and manage java process`.

### Task 7: Runner service

**Files:**
- Create: `src/main/runner.ts`, `src/main/runner.test.ts`

**Interfaces:**
- Consumes: everything in Tasks 2–6.
- Produces:
  ```ts
  export class Runner {
    constructor(o: { vendor: VendorPaths; dataDir: string; emit: (e: AppEvent) => void; pollMs?: number });
    open(root: string): Promise<void>;   // emits 'project'
    refresh(): Promise<void>;            // emits 'project'
    run(key: string): Promise<void>;     // resolves when the run ends; emits per spec R3–R4
    cancel(): Promise<void>;
    reportPath(): string | null;         // karate-summary.html of the last finished run, if it exists
    current(): AppEvent | null;          // last 'project' event, re-sent when the window (re)loads
    dispose(): Promise<void>;            // kills an active run
  }
  ```
- Output dirs `<dataDir>/projects/<first 12 hex of sha1(root)>/run` and `.../dryrun`, emptied before each spawn. Pid file `<dataDir>/running.pid`.

- [ ] **Step 1:** Tests in `runner.test.ts` (real Java, 120 s timeout; collect emitted events in an array):
  - `opens the clean project`: `project` event with layout `standalone`, tree with 9 leaves, `error` null
  - `opens the maven fixture and runs it`: layout `maven`; `run('d:')` → `runFinished` outcome `finished`, summary 1 passed
  - `reports a syntax error`: on `copyFixture('broken')` → `project` event with `tree` null and `error` containing `syntax.feature`
  - `opens an empty folder`: tree with 0 leaves, `error` null
  - `open on a missing folder emits an error`
  - `opens a project whose path has spaces and accents`: copy clean into `<tmp>/Mes tests/équipe` → 9 leaves
  - `runs all with live progress`: `runStarted` total 9; at least one `scenarios` event arrives before `runFinished`; final 8 passed, 1 failed; `reportPath()` exists
  - `runs one outline row`: `run('s:tests/outline.feature#1.2')` → `runStarted` total 1, keys `['s:tests/outline.feature#1.2']`
  - `re-discovers before running`: after open, insert a scenario at the top of `basic.feature` → `run(<old failing match key>)` emits a `project` event with 10 leaves, then runs exactly the scenario named `failing match`
  - `refuses a test that changed`: rename `failing match` in the file → `run(<its old key>)` emits `notice` `This test changed; the tree was refreshed` and no `runStarted`
  - `rejects a run while another is active`: second `run()` during a run emits `notice` and spawns nothing
  - `cancel interrupts and leaves no java`: run `f:tests/slow.feature`, `cancel()` after the first `running` update → outcome `cancelled`, an `interrupted` update, `reportPath()` null, no `java.exe` from `vendor/jre` in `tasklist`, pid file gone
  - `does not modify the project`: hash all files under the fixture before and after a full run; only paths under `karate-temp` may differ
- [ ] **Step 2:** Run `npx vitest run src/main/runner.test.ts` → FAIL.
- [ ] **Step 3:** Implement. Discovery: dry run on all feature roots, wait for exit, `readAllEvents`, `buildTree`. Exit 3 → `error` = console lines containing `Exception` or `feature`, else the last 20 lines. `run(key)`: reject if active; re-discover (emit `project`); `resolveNode`; zero leaves → `notice` `Nothing to run`; write pid file; emit `runStarted` (the window clears its console on it); every `pollMs` (default 100) feed `JsonlTail.read()` to `RunState.apply` and emit; forward console lines batched; on exit do a last `read()`, then `finish` and delete the pid file.
- [ ] **Step 4:** Run → PASS. Commit: `feat(runner): orchestrate discovery, runs and cancel`.

### Task 8: Main process wiring and IPC

**Files:**
- Create: `src/shared/ipc.ts`, `src/main/settings.ts`, `src/main/settings.test.ts`
- Modify: `src/main.ts`, `src/preload.ts`

**Interfaces:**
- Consumes: `Runner` (Task 7), `vendorPaths` (Task 2), `killOrphan` (Task 6).
- Produces:
  ```ts
  // src/shared/ipc.ts
  export const CH = { ready: 'app:ready', open: 'project:open', refresh: 'project:refresh', run: 'run:start', cancel: 'run:cancel', report: 'report:open', event: 'app:event' } as const;
  export interface Api {
    ready(): Promise<void>; openProject(): Promise<void>; refresh(): Promise<void>;
    run(key: string): Promise<void>; cancel(): Promise<void>; openReport(): Promise<void>;
    onEvent(cb: (e: AppEvent) => void): () => void;
  }
  // src/main/settings.ts
  export function loadLastProject(dataDir: string): string | null;
  export function saveLastProject(dataDir: string, root: string): void;
  ```

- [ ] **Step 1:** Tests in `settings.test.ts`: save then load returns the folder; `loadLastProject ignores a missing folder` (saved folder deleted → `null`); corrupt `settings.json` → `null`.
- [ ] **Step 2:** Run → FAIL; implement (`<dataDir>/settings.json`); run → PASS.
- [ ] **Step 3:** `src/main.ts`: on `app.whenReady`, `killOrphan(<userData>/running.pid, vendor.jreDir)`; create the `Runner` with `vendorPaths(app.isPackaged ? process.resourcesPath : path.join(app.getAppPath(), 'vendor'))`, `dataDir = app.getPath('userData')`, and `emit` = `win.webContents.send(CH.event, e)`. Handlers: `ready` re-sends `runner.current()` (idempotent, Forge reloads the renderer); `open` shows `dialog.showOpenDialog({ properties: ['openDirectory'] })`, then `runner.open` and `saveLastProject`; `run` validates that `key` is a string of at most 1,000 characters matching `^[dfos]:`; `report` calls `shell.openPath(runner.reportPath())` only if non-null. Open the last project at start. On `before-quit` during a run: `preventDefault`, `await runner.dispose()`, then `app.quit()`.
- [ ] **Step 4:** `src/preload.ts`: expose `api: Api` with `contextBridge`, `ipcRenderer.invoke` per channel and `onEvent` returning an unsubscribe.
- [ ] **Step 5:** `npm run typecheck` → no errors. `npm start` → window opens, no console errors. Commit: `feat(ipc): wire runner to the window`.

### Task 9: Renderer store

**Files:**
- Create: `src/shared/status.ts`, `src/shared/status.test.ts`, `src/renderer/store.ts`, `src/renderer/reduce.ts`, `src/renderer/reduce.test.ts`

**Interfaces:**
- Consumes: `AppEvent`, `TestNode`, `Status`, `Step` (Tasks 4–5), `Api` (Task 8).
- Produces:
  ```ts
  // status.ts
  export function deriveStatus(node: TestNode, statuses: Record<string, Status>): Status;
  // reduce.ts
  export interface RunView { active: boolean; total: number; done: number; passed: number; failed: number; startedAt: number | null; outcome: RunOutcome | null; summary: Summary | null; reportAvailable: boolean }
  export interface AppState {
    project: Project | null; tree: TestNode | null; error: string | null;
    statuses: Record<string, Status>; results: Record<string, { durationMillis?: number; error?: string }>;
    steps: Record<string, Step[]>; console: string[]; run: RunView; notice: string | null; selected: string | null;
  }
  export const CONSOLE_LIMIT = 10_000;
  export function initialState(): AppState;
  export function reduce(s: AppState, e: AppEvent): void; // mutates a draft
  // store.ts
  export function createAppStore(api: Api): { state: AppState; select(key: string): void };
  ```

- [ ] **Step 1:** Tests in `status.test.ts`: a feature with children passed+failed → `failed`; passed+running → `running`; all passed → `passed`; any `notRun` and none failed or running → `notRun`; `interrupted` child and no failed → `interrupted`.
- [ ] **Step 2:** Tests in `reduce.test.ts` (plain objects, Node): replay `dryrun-clean.jsonl` through `buildTree` into a `project` event, then events from `RunState` on `run-clean.jsonl`:
  - `runStarted` sets selection keys to `pending`, keeps others, clears `console`, sets `total`
  - after all events: `done` 9, `passed` 8, `failed` 1, outcome `finished`
  - a `project` event keeps statuses of keys that still exist and drops the others
  - `console keeps the last 10000 lines`: 10,500 lines in 3 events → length 10,000, last line kept
- [ ] **Step 3:** Run → FAIL. Implement. `createAppStore` wraps `reduce` in a Solid 2 store setter (draft), subscribes with `api.onEvent`, then calls `api.ready()`. Run → PASS.
- [ ] **Step 4:** Commit: `feat(ui): renderer store fed by app events`.

### Task 10: Main screen

**Files:**
- Create: `src/renderer/App.tsx`, `src/renderer/Toolbar.tsx`, `src/renderer/Progress.tsx`, `src/renderer/Tree.tsx`, `src/renderer/Details.tsx`, `src/renderer/Console.tsx`, `src/renderer/Tree.test.tsx`, `src/renderer/Console.test.tsx`
- Modify: `src/renderer.tsx`, `src/index.css`

**Interfaces:**
- Consumes: `createAppStore`, `deriveStatus`, `Api`.
- Produces these `data-testid`s, used by Task 11: `open-button`, `refresh-button`, `run-all-button`, `cancel-button`, `report-button`, `project-path`, `project-layout`, `progress-text` (`<done> / <total>`), `progress-passed`, `progress-failed`, `progress-elapsed`, `run-outcome` (`Running…`, `Finished`, `Cancelled`, `Run failed`), `notice`, `tree`, `tree-message`, `tree-node` (with `data-key`, `data-status`, `data-kind`), `run-node` (inside each `tree-node`), `details`, `details-location`, `details-error`, `step` (with `data-status`), `step-toggle`, `step-log`, `console`.

- [ ] **Step 1:** `Tree.test.tsx` (jsdom): renders the clean tree; a `failed` scenario's `tree-node` has `data-status="failed"` and so does its feature; clicking `run-node` on `add 2 and 3` calls `api.run('s:tests/outline.feature#1.2')`; Run buttons are disabled while `run.active`, and `run-all-button` is disabled when `tree` is null or has no leaves; every `run-node` has an accessible name `Run <node name>`.
- [ ] **Step 2:** `Console.test.tsx`: lines containing ` ERROR ` get the error style class, ` WARN ` the warning class; it stays scrolled to the bottom on new lines unless the user scrolled up.
- [ ] **Step 3:** Run `npx vitest run src/renderer` → FAIL.
- [ ] **Step 4:** Implement layout A from brainstorming: toolbar (path, layout, Open…, Refresh, Run all, Cancel, Report) / progress section (bar split passed and failed, `done / total`, counts, elapsed, outcome) / tree left and details right / console bottom. Native `<button>` elements; tree as nested `<ul role="tree">` with `aria-expanded`. Details per spec R5; steps unavailable text until the feature's `steps` event arrived. Report enabled only when `run.reportAvailable`. Use Solid 2 APIs only (AGENTS.md rules).
- [ ] **Step 5:** Run → PASS. `npm start`, open the clean fixture, Run all, check by eye. Commit: `feat(ui): main screen with tree, progress, details and console`.

### Task 11: End-to-end acceptance suite

**Files:**
- Create: `e2e/helpers.ts`, `e2e/open.spec.ts`, `e2e/run.spec.ts`, `e2e/details.spec.ts`, `e2e/cancel.spec.ts`, `e2e/report.spec.ts`

**Interfaces:**
- Consumes: test ids (Task 10), `copyFixture` (Task 2).
- Produces in `helpers.ts`: `launch(): Promise<{ app: ElectronApplication; win: Page; userData: string }>` (fresh temp folder passed as env `KARATE_KID_USER_DATA`; `main.ts` calls `app.setPath('userData', ...)` when it is set, before anything reads `userData`), `openProject(app, dir)` (stubs `dialog.showOpenDialog` with `app.evaluate` to return `dir`, then clicks `open-button`), `stubOpenPath(app): Promise<() => Promise<string[]>>` (records `shell.openPath` calls), `ourJavaPids(): number[]` (PowerShell `Get-CimInstance Win32_Process -Filter "Name='java.exe'"`, keeping those whose `ExecutablePath` is under `vendor\jre`).

- [ ] **Step 1:** Write one Playwright test per spec Gherkin scenario, with the scenario's name as the test title and its `Then`/`And` lines as assertions on the test ids. Timeouts: 60 s per test. Specifics:
  - "Open a project and see its tests": `tree-node` with `data-kind="row"` under `o:tests/outline.feature#1` count 3; no `data-key` containing `common/create-user`; `project-layout` = `Standalone`; tree visible within 2 s of the click (`expect(...).toBeVisible({ timeout: 2000 })`)
  - "Run all with live progress": `progress-text` shows `0 / 9`; the `slow loop` node reaches `data-status="running"` before `run-outcome` shows `Finished`; final `progress-passed` 8, `progress-failed` 1
  - "A feature edited outside the app": edit the copied fixture file from the test with `fs`
  - "Cancel leaves nothing behind" and "Closing the app during a run": assert `ourJavaPids()` is empty after
  - "Open the HTML report": `stubOpenPath` records one path ending `karate-summary.html`
  - "The project is not modified": file hashes before and after, ignoring `karate-temp`
- [ ] **Step 2:** Run `npm run e2e` → all pass. If one fails, fix the app, not the assertion, unless the assertion contradicts the spec.
- [ ] **Step 3:** Commit: `test(e2e): acceptance suite for the walking skeleton`.

### Task 12: Packaging, manual check and docs

**Files:**
- Modify: `forge.config.mts`, `AGENTS.md`, `README.md`, `docs/features.md`
- Create: `docs/checks/2026-10-24-walking-skeleton-packaged.md`

- [ ] **Step 1:** `forge.config.mts`: `packagerConfig.extraResource: ['vendor/jre', 'vendor/karate.jar', 'vendor/licenses']`; maker-zip for `win32` only. Run `npm run make` → `out/make/zip/win32/x64/karate-kid-win32-x64-*.zip` exists and contains `resources/jre/bin/java.exe`, `resources/karate.jar`, `resources/licenses/`.
- [ ] **Step 2:** Write the manual check list from the spec ("How they are checked"): on Windows 11 without Java (`where java` finds nothing), unzip, start, open the clean fixture, Run all → `Finished` 8/1, run and Cancel `slow.feature` → no `java.exe` left (Task Manager), Report opens in the browser. Record date, machine and result in the file. Run it.
- [ ] **Step 3:** `AGENTS.md`: replace "documentation only … no build or test commands yet" with the commands (`npm run vendor` once, `npm start`, `npm test`, `npm run e2e`, `npm run typecheck`, `npm run lint`, `npm run make`) and add `docs/checks/` to the docs map. `README.md`: the same setup lines. `docs/features.md`: F01, F02, F03, F05, F06, F07, F09 → `done`.
- [ ] **Step 4:** Run `npm test`, `npm run e2e`, `npm run typecheck`, `npm run lint` → all pass. Commit: `build: package windows zip with bundled java and karate`, then open the PR to `develop` with the template filled in.
