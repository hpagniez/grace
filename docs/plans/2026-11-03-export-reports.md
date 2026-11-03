# Export a run's report Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Runs write JUnit XML when `karate-pom.json` asks, and each history row can save its run's report as a zip.

**Architecture:** `readPom` reads `output.junitXml`; `detectLayout` carries it; `spawn` adds `junit:xml` to `-f` for runs only. A new `zipReport` in main writes the zip with Windows' `tar.exe`. A new IPC call takes a run id; main shows the Save dialog, zips the run's folder and sends a notice. History rows get an Export button.

**Tech Stack:** Electron main (Node `child_process.execFile`), SolidJS 2 renderer, Vitest, Playwright.

**Spec:** `docs/specs/2026-11-03-export-reports.md`

## Global Constraints

- No new dependency. The zip is made by `TAR` from `src/main/updates/fetchZip.ts` (`%SystemRoot%\System32\tar.exe`), with `execFile`, no shell, `windowsHide: true`.
- tar arguments, exactly: `-c --format zip --options hdrcharset=UTF-8 -f <dest> -C <run folder> <entries…>`, where the entries are the run folder's own names except `run.json` and `karate-json`.
- The window sends a run id only; main checks it with `handleRunId` (`RUN_ID`).
- Copy: dialog title `Export report`; filter `{ name: "Zip archive", extensions: ["zip"] }`; proposed name `<project folder name>-report-<run id>.zip` in `app.getPath("downloads")`; success notice `Report saved to <full path>` with `success: true`; failure notice `The report could not be saved: <reason>`; pom error `output.junitXml is not true or false`; button label `Export`.
- Solid 2 rules and UI conventions of `AGENTS.md`: icons through `Icon` with `lucide` data, row buttons use `ghostSmall`.
- Idle costs nothing: no timer, listener or poll.
- Keep the change to `src/main/karate/command.ts` to `buildArgs`'s `-f` and the new `junitXml` option; another branch (#223) edits `locate` and `selectionTargets`.

## Review Focus

- The person types a name with another extension, or none: the file is still a zip, under the name they typed (Task 2 test "writes a zip whatever the name").
- The person replaces an existing file: it is overwritten, not appended to (Task 2 test "replaces an existing file").
- The run's folder is gone by the time the person saves (history cleared, run pruned): a notice, and no file left (Task 2 test "leaves nothing when the folder is gone").
- A destination path with spaces and accents: passed as one argument, no shell (Task 2 test uses `out dir é/r é.zip`).
- A run starts while a row is exported: every Export button is disabled while busy (Task 4 History test).

---

### Task 1: JUnit XML when karate-pom.json asks

**Files:**
- Modify: `src/main/karate/pom.ts`, `src/main/karate/command.ts` (`ProjectLayout`, `detectLayout`, `buildArgs`), `src/main/runner.ts` (`spawn`)
- Test: `src/main/karate/pom.test.ts`, `src/main/karate/command.test.ts`, `src/main/runner.setup.test.ts`

**Interfaces:**
- Produces: `Pom.junitXml: boolean` (`NO_POM.junitXml === false`); `ProjectLayout.junitXml: boolean`; `buildArgs` option `junitXml: boolean` (required, so no caller forgets it).

- [ ] **Step 1: Write the failing tests**

`pom.test.ts`: the `broken` helper gains `junitXml: false`. New cases:
- `{"output":{"junitXml":true,"dir":"x","html":false}}` → `junitXml: true`, `unused: ["output.dir", "output.html"]`.
- `{"output":{"junitXml":false}}` → `junitXml: false`, `unused: []`.
- `{"output":"x"}` → `junitXml: false`, `unused: ["output"]`.
- `{"output":{"junitXml":"yes"}}` → `broken("output.junitXml is not true or false")`.
- The existing "keeps paths and configDir…" case expects `junitXml: false`.

`command.test.ts`: `detectLayout` `toEqual` cases gain `junitXml: false`; one new case with a pom setting it true gives `junitXml: true`. `buildArgs`: the shared `o` gains `junitXml: false`; new case `buildArgs({ ...o, dryRun: false, junitXml: true })` equals `base.with(19, "html,karate:jsonl,junit:xml")` plus `-T 1 t1`.

`runner.setup.test.ts`, new test "writes JUnit XML when karate-pom.json asks" (`T`): a project `{ "tests/a.feature": "Feature: a\nScenario: a1\n  * print 'a'\n", "karate-pom.json": '{"output":{"junitXml":true}}' }`; open, `run("d:")`; the only run folder under `<dataDir>/projects/*/runs/` holds exactly one `junit-xml/*.xml` file; `<dataDir>/projects/*/dryrun/junit-xml` does not exist; no `notice` names `output`.

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/main/karate/pom.test.ts src/main/karate/command.test.ts src/main/runner.setup.test.ts`
Expected: FAIL on the new cases (missing `junitXml`, no `junit-xml` folder).

- [ ] **Step 3: Implement**

- `readPom`: destructure `output` with `paths` and `configDir`. When `output` is a plain object (not null, not an array): `junitXml` is its `junitXml` (`undefined` → false; not a boolean → `broken("output.junitXml is not true or false")`), and its other keys go into `unused` as `output.<key>`, in place of `output`, keeping the file's key order. Otherwise `output` (if present) stays in `unused` as today.
- `detectLayout`: `junitXml: pom.junitXml`, with a one-line JSDoc on `ProjectLayout.junitXml` (Karate's `output.junitXml`, export spec R1).
- `buildArgs`: `"-f", o.junitXml ? "html,karate:jsonl,junit:xml" : "html,karate:jsonl"`.
- `spawn`: take `junitXml` from the same `detectLayout(root)` call and pass `junitXml: !dryRun && junitXml`.

- [ ] **Step 4: Run them to see them pass**

Same command. Expected: PASS. Then `npm run typecheck`: no error.

- [ ] **Step 5: Commit**

`feat(runner): write JUnit XML when karate-pom.json asks` with `Part of #70`.

---

### Task 2: Zip a run's report

**Files:**
- Create: `src/main/report-zip.ts`
- Test: `src/main/report-zip.test.ts`

**Interfaces:**
- Consumes: `TAR` from `src/main/updates/fetchZip.ts`.
- Produces: `zipReport(dir: string, dest: string): Promise<void>`: resolves once `dest` is written; rejects with an `Error` whose message is tar's stderr (trimmed) or the system error, after removing `dest`.

- [ ] **Step 1: Write the failing tests**

Each test builds a run folder in a temp dir: `karate-summary.html`, `feature-html/café.html`, `junit-xml/a.xml`, `karate-json/karate-events.jsonl`, `run.json`. A helper unpacks a zip with `TAR -xf` into a fresh temp dir and returns its files as sorted relative paths with `/`.
- "holds the report and leaves out run.json and karate-json": `zipReport(run, "<tmp>/out dir é/r é.zip")` (folder created first); the unpacked files equal `["feature-html/café.html", "junit-xml/a.xml", "karate-summary.html"]`.
- "writes a zip whatever the name": dest `<tmp>/r.bin`; it unpacks to the same three files.
- "replaces an existing file": write `junk` to the dest first; after `zipReport`, it unpacks to the three files.
- "leaves nothing when the folder is gone": `zipReport("<tmp>/missing", dest)` rejects, and `existsSync(dest)` is false.

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/main/report-zip.test.ts`
Expected: FAIL, module not found.

- [ ] **Step 3: Implement `zipReport` in `src/main/report-zip.ts`**

`readdirSync(dir)` minus `run.json` and `karate-json` gives the entries; `promisify(execFile)(TAR, [Global Constraints arguments], { windowsHide: true })`. The whole body is in one `try`; the `catch` runs `rmSync(dest, { force: true })` and throws `new Error(stderr?.trim() || message)`, as `unpack` in `fetchZip.ts` does. One-line JSDoc citing export spec R3.

- [ ] **Step 4: Run them to see them pass**

Same command. Expected: PASS.

- [ ] **Step 5: Commit**

`feat(runner): zip a run's report with Windows' tar` with `Part of #70`.

---

### Task 3: Export over IPC

**Files:**
- Modify: `src/shared/ipc.ts`, `src/preload.ts`, `src/main/runner.ts`, `src/main.ts`, and every test file whose `Api` object no longer typechecks
- Test: `src/main/runner.history.test.ts`

**Interfaces:**
- Consumes: `zipReport` (Task 2); `Runner.runReportPath(id)`.
- Produces: `CH.exportReport = "history:export"`; `Api.exportReport(id: string): Promise<void>` (JSDoc: sends the run's id only; main asks where to save); `Runner.exportSource(id: string): { dir: string; name: string } | null`.

- [ ] **Step 1: Write the failing test**

In `runner.history.test.ts`, beside "runReportPath is null for a cancelled run and for an unknown id", reuse its finished and cancelled runs: `exportSource(done)` equals `{ dir: path.dirname(runner.runReportPath(done)!), name: \`${path.basename(root)}-report-${done}.zip\` }`; `exportSource(cancelled)`, `exportSource("20200101-000000-000")` and `exportSource(\`${done}\\..\\${done}\`)` are null.

- [ ] **Step 2: Run it to see it fail**

Run: `npx vitest run src/main/runner.history.test.ts -t exportSource`
Expected: FAIL, `exportSource` is not a function.

- [ ] **Step 3: Implement**

- `Runner.exportSource`: built on `runReportPath`; JSDoc "The folder and proposed zip name of a listed run's report (export spec R2.3); null if it has none."
- `ipc.ts` and `preload.ts`: the channel and `exportReport: (id) => ipcRenderer.invoke(CH.exportReport, id)`.
- `main.ts`: `handleRunId(CH.exportReport, …)` after `CH.runReport`. Get `exportSource(id)`; return if null. `dialog.showSaveDialog` (with `win` when it exists and is not destroyed, as `CH.open` does) with the Global Constraints title, filter and `defaultPath`. Return on `canceled` or no `filePath`. `await zipReport(source.dir, filePath)`, then emit the success notice; on a throw emit the failure notice with the error's message.
- Add `exportReport: vi.fn(async () => {})` wherever a test builds an `Api`.

- [ ] **Step 4: Run it to see it pass**

Same command, then `npm run typecheck` and `npm run lint`. Expected: PASS, no error.

- [ ] **Step 5: Commit**

`feat(ipc): save a run's report as a zip where the person picks` with `Part of #70`.

---

### Task 4: The Export button

**Files:**
- Modify: `src/renderer/History.tsx`
- Test: `src/renderer/History.test.tsx`, `e2e/history.spec.ts`, `e2e/helpers.ts`

**Interfaces:**
- Consumes: `Api.exportReport(id)` (Task 3).
- Produces: the row button `data-testid="history-export"`; e2e helper `stubSaveDialog(app: ElectronApplication, file: string | null): Promise<() => Promise<number>>` (replaces `dialog.showSaveDialog` with one that resolves `{ canceled: false, filePath: file }`, or `{ canceled: true, filePath: "" }` for null, and returns how many times it was called).

- [ ] **Step 1: Write the failing tests**

`History.test.tsx`: `rowButtons` becomes `["history-report", "history-export", "history-rerun", "history-rerun-failed"]`, which brings it under the existing checks on the disabled state while busy and `ghostSmall`; the expected labels become `["Report", "Export", "Re-run", "Re-run failed"]`. In "actions call the api with the id": its icon is lucide `Download`, and a click on row 1's Export calls `api.exportReport` once with `"20261024-172000-000"`. In the `reportAvailable: false` case: row 0's `history-export` is disabled, row 1's enabled.

`e2e/history.spec.ts`, new test "Export a run's report": copy `project("clean")`, write `karate-pom.json` `{"output":{"junitXml":true}}` into it, launch, open, run `d:`, wait for `Finished`. Open the History tab. `stubSaveDialog(app, null)`, click the top row's `history-export`: the stub's count reaches 1, the notice stays hidden and the temp dir stays empty. `stubSaveDialog(app, <temp dir>/r.zip)`, click it again. The notice reads `Report saved to <that path>`. Unpacked with `tar -xf`, the zip holds `karate-summary.html` and one `junit-xml/*.xml` per feature of the clean project, and nothing under `karate-json`. Then `window.api.exportReport("..\\x")` (via `win.evaluate`, as `editor.spec.ts` does) makes the notice read `Invalid request` and leaves the stub's count at 1.

- [ ] **Step 2: Run the unit test to see it fail**

Run: `npx vitest run src/renderer/History.test.tsx`
Expected: FAIL, no `history-export`.

- [ ] **Step 3: Implement**

In `History.tsx`, after the Report button: the same markup with `data-testid="history-export"`, `Icon icon={Download} size={14}`, label `Export`, the same `disabled` expression as Report, `onClick={() => void props.api.exportReport(run.id)}`. In `e2e/helpers.ts`, `stubSaveDialog` beside `stubOpenPath`.

- [ ] **Step 4: Run the tests to see them pass**

`npx vitest run src/renderer/History.test.tsx`: PASS. The e2e run waits for the parent session's go (machine sharing), then `npm run e2e`: all pass.

- [ ] **Step 5: Commit**

`feat(ui): export a run's report from its history row` with `Closes #70` in the PR, `Part of #70` in the commit.
