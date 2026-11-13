---
status: built
issue: 65
summary: "Breakpoints on steps, and a debug run that pauses, steps and shows the paused scenario's variables"
---

# Step Debugger Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Breakpoints on steps, a debug run that pauses, steps and shows the paused scenario's variables.

**Architecture:** A Java `RunListener` (`listener/grace/DebugListener.java`, compiled by `npm run vendor` with ECJ into `vendor/listener.jar`) connects to a loopback TCP server that the main process opens per debug run (`src/main/karate/debug.ts`). The Runner runs a debug run like any run, with one thread and the listener, stops tailing the JSONL while paused, and emits `debugPaused`/`debugResumed`/`breakpoints` events. The window shows steps (from the run, or read from the feature file), breakpoint toggles, a Debug button per row, toolbar controls and a Variables tab.

**Tech Stack:** Java 21 (Karate 2.1.4 API, ECJ 3.46.100), Node 24 `net`, Electron IPC, SolidJS 2, Vitest, Playwright.

**Spec:** [docs/specs/2026-11-02-step-debugger.md](../specs/2026-11-02-step-debugger.md). Research: [docs/research/2026-11-02-step-debugger/](../research/2026-11-02-step-debugger/README.md). ADR 0025.

## Global Constraints

- Listener class `grace.DebugListener`; system properties `grace.debug.port`, `grace.debug.token`; jar `listener.jar`.
- Values cut at 65,536 characters (`VALUE_LIMIT`).
- A debug run: `-T 1`, `listener.jar` right after `karate.jar` on the classpath, `--listener grace.DebugListener`. Normal runs and dry runs unchanged.
- Server on `127.0.0.1`, port 0, one accepted connection (first line = token), then no longer listening.
- No timer runs while paused; nothing spins while paused (spec R7.2).
- Keys: F5 continue, F10 step over, F11 step into, Ctrl+Shift+Enter debug the selected row.
- Solid 2 and UI rules of `AGENTS.md` (tokens, `Icon`, `Tooltip`, `StatusMark`, no `title=`, shared class strings).
- Machine: run nothing heavy (Java, Karate, `npm test`, e2e, bench, make) until the parent session says go.

## Review Focus

1. A pause line Node cannot parse (Karate value with odd characters, a cut surrogate pair): `parsePause` throws, the session drops the socket, the run goes on without pausing (Task 2 test `drops a bad line`).
2. A feature path with spaces or non-ASCII, or a Maven project where Karate's path is relative to `src/test/java`: breakpoints match because both sides use the tree's `file` (Task 3 test on the `maven` fixture).
3. Cancel or app quit while paused: Java is killed, the session and server close, no `setInterval` survives (Task 3 test `cancel while paused`).
4. Breakpoint toggled while Java runs between pauses: the new set reaches the listener and fires (Task 3 test `breakpoint added mid-run`).
5. Window reload during a debug run: the breakpoints come back (Task 3 test on `replay()`).

---

## Slice 1 (PR 1, `Part of #65`): listener, toolchain, main process

### Task 1: Listener source, ECJ pin, compile in `npm run vendor`, package

**Files:**
- Create: `listener/grace/DebugListener.java`
- Modify: `scripts/vendor.json` (add `ecj`), `scripts/fetch-vendor.mjs`, `forge.config.mts:21`, `src/main/vendor.ts`, `AGENTS.md` (Commands table: `npm run vendor` also compiles the debug listener)
- Test: `scripts/fetch-vendor.test.ts` (`zipArgs`), the listener itself through Task 3

**Interfaces:**
- Produces: `vendor/listener.jar`; `VendorPaths.listener: string` (`join(base, "listener.jar")`).
- Wire format (Task 2 relies on it). Listener → app: line 1 the token; then one JSON line per pause: `{"file","line","depth","prefix","keyword","text","test":{"file","refId"},"vars":[{"name","kind","type"?,"value","cut"?}]}`. App → listener: line 1 `{"breakpoints":[{"file","line"}],"pause":bool}`; later `{"breakpoints":[...]}` or `{"command":"continue"|"over"|"into"|"pause"}`.

- [ ] **Step 1:** `scripts/vendor.json` gets `"ecj": { "version": "3.46.100", "url": "https://repo1.maven.org/maven2/org/eclipse/jdt/ecj/3.46.100/ecj-3.46.100.jar", "sha256": "74c93172475fee0cd4e6eefb44233516dd40525642da35d009146d97ea598229", "file": "ecj-3.46.100.jar" }`.
- [ ] **Step 2:** Write `DebugListener.java` from the spike, changed to the spec:
  - constructor: connect, write token line, read the first line (`Json.parseStrict` → `Map`), set `breakpoints` (a `Set<String>` of `file + ":" + line`) and `pauseAsked`; start a daemon reader thread.
  - reader thread: per line, `breakpoints` replaces the set; `command` `pause` sets `pauseAsked`, any other command goes into a `LinkedBlockingQueue<String>`; at EOF or error sets `dead` and offers `"continue"`.
  - `onEvent`: only `STEP_ENTER`; pause rule of spec R1.3 (`mode` is `"run"`, `"over"` or `"into"`, `overDepth` the paused depth); on pause clear `pauseAsked`, send the pause line, `take()` the command, set `mode`/`overDepth`. The paused test: walk `ScenarioRuntime` → `getFeatureRuntime().getCallerScenario()` until depth 0, then its feature's relative path and `getScenario().getRefId()`.
  - `kind` per spec R1.5 (`JsFunction`/`JsFunctionWrapper` → `function`, `Boolean` → `boolean`, `Number` → `number`); `type` only for `other` (`getClass().getSimpleName()`); a value over 65,536 chars → first 65,536 and `cut` = chars left out.
  - every `onEvent` body in `try { … } catch (Throwable t) { dead = true; }`, return `true`.
- [ ] **Step 3:** `fetch-vendor.mjs`: `ecj` via `fetchVerified` then copied to `vendor/ecj.jar` (as `karate.jar` is, so the CI `vendor/` cache keeps it); `compileListener()` runs `vendor/jre/bin/java.exe -jar vendor/ecj.jar -21 -nowarn -cp vendor/karate.jar -d vendor/.download/listener-classes listener/grace/DebugListener.java` (a compile error fails the script); then `tar.exe` (System32) `zipArgs(jar, classes)` = `["-a", "-c", "-f", jar, "-C", classes, "grace"]`; throws on a non-zero status. Export `zipArgs` and test it returns exactly that array.
- [ ] **Step 4:** `forge.config.mts` `extraResource` adds `"vendor/listener.jar"`; `vendorPaths` adds `listener`.
- [ ] **Step 5 (after go):** `npm run vendor`; expect `vendor/listener.jar` under 10 KB, listing `grace/DebugListener.class`.
- [ ] **Step 6:** Commit `build(vendor): compile the debug listener with ecj` (`Part of #65`).

### Task 2: The debug session (`src/main/karate/debug.ts`)

**Files:**
- Create: `src/main/karate/debug.ts`, `src/main/karate/debug.test.ts`
- Modify: `src/shared/events.ts` (types below)

**Interfaces:**
- Produces in `src/shared/events.ts`:
  - `Breakpoint { file: string; line: number }`
  - `VarKind = "string"|"number"|"boolean"|"null"|"json"|"xml"|"function"|"other"`
  - `DebugVar { name: string; kind: VarKind; type?: string; value: string; cut?: number }`
  - `Paused { key: string | null; file: string; line: number; depth: number; prefix: string; keyword: string; text: string; vars: DebugVar[] }`
  - `FileStep { keyword: string; text: string; line: number; background: boolean }`
  - `DebugCommand = "continue" | "over" | "into" | "pause"`
  - `AppEvent` adds `{ type: "breakpoints"; breakpoints: Breakpoint[] }`, `{ type: "debugPaused"; paused: Paused }`, `{ type: "debugResumed" }`; `runStarted` adds `debug: boolean`.
- Produces in `debug.ts`: `DEBUG_LISTENER = "grace.DebugListener"`, `VALUE_LIMIT = 65_536`, `interface ListenerPause` (wire shape above), `parsePause(line: string): ListenerPause` (throws on a wrong shape), `isDebugCommand(x: unknown): x is DebugCommand`, and

```ts
export class DebugSession {
  static open(breakpoints: Breakpoint[], onPause: (p: ListenerPause) => void): Promise<DebugSession>;
  readonly port: number;
  readonly token: string; // randomBytes(16) hex
  setBreakpoints(b: Breakpoint[]): void; // before connect: kept for the first line
  command(c: DebugCommand): void; // before connect only "pause" counts: sent as "pause": true
  close(): void; // closes server and socket; idempotent
}
```

- [ ] **Step 1: Failing tests** (`debug.test.ts`, a `net.connect` client stands in for Java):
  - `sends breakpoints and a queued pause after the token`: `open([{file:"a.feature",line:8}])`, `command("pause")`, client writes token → first line `{"breakpoints":[{"file":"a.feature","line":8}],"pause":true}`.
  - `closes a connection with a wrong token and serves the right one`.
  - `stops listening once connected`: a second `connect` to `port` fails with `ECONNREFUSED`.
  - `parses a pause and writes commands`: client sends a pause line → `onPause` gets it; `command("over")` → client reads `{"command":"over"}`; `setBreakpoints([])` → `{"breakpoints":[]}`.
  - `drops a bad line`: client sends `{"file":1}` → no `onPause`, socket closed, no throw.
  - `survives the client resetting`: client `destroy()` → no unhandled `error`.
  - `parsePause` rejects a missing `vars` and accepts `cut`.
- [ ] **Step 2:** Run `npx vitest run src/main/karate/debug.test.ts` (after go); expect failures.
- [ ] **Step 3:** Implement with `net.createServer`, `readline` per socket, a `'error'` handler on server and socket.
- [ ] **Step 4:** Tests pass. Commit `feat(runner): add the debug session socket`.

### Task 3: Debug runs, breakpoints and file steps in the Runner

**Files:**
- Modify: `src/main/karate/command.ts` (`buildArgs` option `debug`), `src/main/runner.ts`, `src/main/karate/steps.ts` (`fileSteps`), `src/main/karate/tree.ts` (`parentOf`), `src/main/mask.ts`, `src/shared/history.ts` (`debug?: true`), `src/main/flaky.ts`, `src/main.ts` and `src/preload.ts` and `src/shared/ipc.ts`
- Create: `test/fixtures/projects/debug/debug.feature` and `called.feature` (exactly the spec's Background, so the lines match), `big.feature` (a 102,400-char variable, then `* print 'after'`); `src/main/runner.debug.test.ts`
- Modify tests: `command.test.ts`, `steps.test.ts`, `mask.test.ts`, `flaky.test.ts`, `history.test.ts`; `test/fixtures/copy.ts` and `test/runner.ts` add `"debug"` to the fixture names

**Interfaces:**
- Consumes: Task 2's `DebugSession`, `DEBUG_LISTENER`, types.
- Produces:
  - `buildArgs({..., debug?: { jar: string; port: number; token: string } | null })`: adds `-Dgrace.debug.port=<port>` and `-Dgrace.debug.token=<token>` after `-Dstderr.encoding=UTF-8`, `debug.jar` after `o.jar` in `-cp`, and `--listener grace.DebugListener` after `-f html,karate:jsonl`.
  - `fileSteps(featureText: string, sectionLine: number): FileStep[]`: background steps (`background: true`) then the steps of the section whose header is on `sectionLine`, per spec R4.2.
  - `parentOf(tree: TestNode, key: string): TestNode | null`.
  - `Runner.debug(key: string): Promise<void>`, `Runner.debugCommand(c: DebugCommand): void`, `Runner.toggleBreakpoint(file: string, line: number): void` (refused unless `file` is a feature node's file of the tree and `line` an integer ≥ 1), `Runner.steps(key: string): FileStep[] | null` (scenario: its line; row: `parentOf` outline's line; file read with `locate`).
  - `secretForms` adds the XML-escaped form; `maskCutEnd(text: string, values: string[]): string` replaces the longest end of `text` that starts a form of a value with `****`.
  - `RunRecord.debug?: true`, parsed (`true` or absent; anything else rejects the record); `findFlaky` skips records with `debug`.
  - IPC: `CH.debug = "run:debug"`, `CH.debugCommand = "debug:command"`, `CH.breakpoint = "debug:breakpoint"`, `CH.steps = "steps:get"`; `Api.debug(key)`, `Api.debugCommand(c)`, `Api.toggleBreakpoint(file, line)`, `Api.steps(key): Promise<FileStep[] | null>`. `main.ts` checks types (`isDebugCommand`, string, integer) before calling the Runner.
- Runner details: `breakpoints` cleared on open and close, sent as `breakpoints` on each change and in `replay()`. `runTarget(target, filter, debug = false)`; `runBody` gets `debug`: threads 1, opens the session before `spawn`, `runStarted.debug`, record `threads: 1` and `debug: true`, closes the session in `finally`. The 100 ms tail becomes `startPoll()`/`stopPoll()`; on a pause: `stopPoll()`, `poll()`, key from `leafIndex(tree).get(`${file}|${refId}`)`, cut values through `maskCutEnd`, `emit({type:"debugPaused"})`. `debugCommand`: `pause` goes to the session; the others only while paused: send, `emit({type:"debugResumed"})`, `startPoll()`.

- [ ] **Step 1: Failing unit tests**
  - `command.test.ts` `a debug run adds the listener`: args hold `-Dgrace.debug.port=5005`, `-Dgrace.debug.token=t`, cp `karate.jar;listener.jar;<classpath>`, `--listener`, `grace.DebugListener`; without `debug` none of them.
  - `steps.test.ts` `fileSteps`: on the fixture's `debug.feature` with section 6 → lines `[4, 7, 8, 9, 10]`, line 4 `background: true`, line 7 `{keyword: "*", text: "def name = 'grace'"}`; a `"""` doc string holding `* not a step` lists nothing inside it; an outline section stops at `Examples:`.
  - `mask.test.ts`: `secretForms("a&b")` holds `a&amp;b`; `maskCutEnd("…tok", ["token"])` ends in `****`; a text whose end starts no form is unchanged.
  - `flaky.test.ts`: a failing debug run beside a passing run with the same fingerprint gives no flaky test.
  - `history.test.ts`: `debug: true` parses; `debug: false` rejects.
- [ ] **Step 2: Failing runner tests** (`runner.debug.test.ts`, real Java, `T` timeout), the spec's main-process scenarios: breakpoint pauses at line 8 with vars `base` and `name` (both `string`) and the paused key of `variables`; step over → line 10 depth 0, `called` is `json` holding `"inner":"in!"`; step into from 9 → `called.feature` line 5 depth 1; pause asked before start → line 4; `threads` setting 4 → the run's record `threads: 1`, `debug: true`; breakpoint added mid-run (pause at 8, add 10, continue → pauses at 10); cancel while paused → outcome `cancelled`, no `java.exe` of the project left; app end of the socket closed while paused (test seam: `runner.debugSocketForTests()?.destroy()`) → run finishes `finished`; secret property `token`=`s3cr3t` and `* def auth = karate.properties['token']` → `auth` value `****`; `big.feature` → `big` value of 65,536 chars, `cut` 36,864; `replay()` holds the `breakpoints` event; the `maven` fixture: a breakpoint on a step of its feature pauses.
- [ ] **Step 3:** Implement. **Step 4 (after go):** `npx vitest run src/main` passes. **Step 5:** Commit per area: `feat(runner): run a test under the debugger`, `feat(runner): read a test's steps from its feature file`, `fix(runner): mask a secret cut at the end of a value`.

### Slice 1 close

- [ ] `npm run lint`, `npm run typecheck`, `npm test` (after go, ask the parent), `npm run e2e`; draft PR into `develop`, `Part of #65`, e2e summary in the body; move #65 to `building`; mark ready; read the `perf` comment.

## Slice 2 (PR 2, `Closes #65`): the window

### Task 4: State and events in the window

**Files:** Modify `src/renderer/reduce.ts`, `src/renderer/reduce.test.ts`.

**Interfaces:** `AppState` adds `breakpoints: Breakpoint[]`, `paused: Paused | null`, `vars: DebugVar[]` (last pause's, kept between pauses); `RunView.debug: boolean`; `panel` adds `"variables"`.

- [ ] Tests: `runStarted` with `debug` sets `run.debug`, clears `paused` and `vars`; the first `debugPaused` of a run sets `panel = "variables"` and `selected = paused.key`, a later one keeps the panel the user chose; `debugResumed` clears `paused`, keeps `vars`; `runFinished` clears `paused`, `run.debug` false, `panel` back to `"console"` if it was `"variables"`; `breakpoints` replaces the list. Implement; commit `feat(ui): hold the debugger's state`.

### Task 5: Steps and breakpoints in the details pane

**Files:** Modify `src/renderer/Details.tsx`, `Details.test.tsx`, `src/renderer/nodes.ts` (`breakpointKeys`).

**Interfaces:** `breakpointKeys(tree: TestNode, b: Breakpoint[]): ReadonlySet<string>`: feature keys with a breakpoint in their file; scenario and outline keys with one on a line from theirs up to the next sibling's.

- [ ] Tests (spec R4, R7.1, R7.3): before a run a leaf calls `api.steps(key)` once and lists the steps without status; the toggle is a button named `Add breakpoint at line 8`, calls `api.toggleBreakpoint("debug.feature", 8)`, shows the dot when `breakpoints` holds it and is then named `Remove breakpoint at line 8`; run steps get the same toggle; a background line's dot shows in `second`; paused at depth 0 marks the row and shows `Paused`; depth 1 shows `Paused in called.feature:5`. The file steps load in a `createEffect` that calls `api.steps` when the selected leaf has no run steps. Commit `feat(ui): set breakpoints on a test's steps`.

### Task 6: Debug button, breakpoint dot and paused mark in the tree

**Files:** Modify `src/renderer/Tree.tsx`, `Tree.test.tsx`, `src/renderer/StatusMark.tsx` (`paused`: `Pause` icon, `text-running`, label `Paused`, never spins).

- [ ] Tests (spec R5, R7.1, R7.2): Debug button `Debug <name>` before Run, `Bug` icon, tooltip `Debug <name> (Ctrl+Shift+Enter)`, disabled as Run; Ctrl+Shift+Enter calls `api.debug`, Ctrl+Enter alone still calls `api.run` (the handler checks `shiftKey`); a row in `breakpointKeys` shows a `failed` dot with an accessible name `Has breakpoints`; the paused key's row shows the `paused` mark; on a new `paused.key` the tree opens its parents and calls the list's `reveal`. Commit `feat(ui): debug a test from the tree`.

### Task 7: Toolbar controls, keys, progress and frozen spinners

**Files:** Modify `src/renderer/Toolbar.tsx`, `Toolbar.test.tsx`, `src/renderer/Progress.tsx`, `Progress.test.tsx`, `src/renderer/App.tsx` (`data-paused` on the root, keydown while paused), `src/index.css` (`[data-paused] .animate-spin { animation-play-state: paused; }`).

- [ ] Tests (spec R6, R7.2): `run.debug` hides `Run all`; running shows `Pause` (`api.debugCommand("pause")`); paused shows `Continue` (`Play`), `Step over` (`RedoDot`), `Step into` (`ArrowDownToDot`), each icon only below 1280 px with its label as tooltip, test ids `debug-continue`, `debug-over`, `debug-into`, `debug-pause`; F5, F10, F11 call `continue`, `over`, `into` only while paused; the progress line says `Paused` with the `Pause` icon and its elapsed time does not change while paused (fake timers: no interval), then counts from `startedAt` after `debugResumed`. Commit `feat(ui): step through a paused test from the toolbar`.

### Task 8: The Variables tab

**Files:** Create `src/renderer/Variables.tsx`, `Variables.test.tsx`; modify `src/renderer/BottomPanel.tsx`, `BottomPanel.test.tsx`.

- [ ] Tests (spec R7.4, R7.5): the tab (`tab-variables`, `Braces` icon) shows only while `run.debug`; rows sorted by name: name, kind, value cut with `Cut`; a `json` value opens into a box with `JSON.stringify(JSON.parse(value), null, 2)` (raw when `cut`), an `xml` value as is; a cut value says how many characters were left out; between pauses the list is dimmed with `Running…`; before the first pause the empty state `Variables show when the run pauses`. Commit `feat(ui): show a paused test's variables`.

### Task 9: End to end, docs, PR

**Files:** Create `e2e/debug.spec.ts`; modify `e2e` fixture names if they mirror `copyFixture`; the spec's status to `approved`.

- [ ] `debug.spec.ts`: the spec's two **(e2e)** scenarios through `launch()`. After go: `npm run lint`, `typecheck`, `npm test`, `npm run e2e`. PR 2 into `develop`, `Closes #65`, e2e summary, the `perf` comment read. After merge of the spec: #65's `Spec:` line points at `docs/specs/2026-11-02-step-debugger.md` on `develop`.
