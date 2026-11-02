# Light and Fast Guard Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Measure the packaged app on every PR and before every release, lock in today's numbers as limits, and settle the rule in the merge and release workflow.

**Architecture:** A plain-Node bench (`bench/*.mjs`) starts any Grace exe with `--remote-debugging-port`, drives its page through Playwright's `chromium.connectOverCDP`, and samples the process tree through PowerShell. The results go to a JSON report and a Markdown table. A pure `judge` module compares a report with `bench/limits.json`. The bench uses it to write the table and the release `Result:` line, and `scripts/check-pr.mjs` uses it to re-check a release record and to refuse raised limits that come without a decision record. A `perf` workflow runs the bench on the PR's base and head and posts one comment, and `merge-when-green` waits for that run.

**Tech Stack:** Node 24 ES modules with JSDoc (like `scripts/*.mjs`), `@playwright/test`'s `chromium` (already installed), PowerShell CIM queries, Vitest for the pure parts, GitHub Actions.

**Spec:** [docs/specs/2026-10-31-light-and-fast.md](../specs/2026-10-31-light-and-fast.md)

## Global Constraints

- Measure the packaged app on Windows 11: `out/Grace-win32-x64/Grace.exe` from `npm run package`, or the exe of an unpacked release zip (`Grace-win32-x64-<version>.zip`).
- No new dependency: CDP through `@playwright/test`, sampling through `powershell.exe`, zip size from the file on disk.
- Workflows use only GitHub-owned actions pinned to a full commit SHA. Reuse the SHAs in `.github/workflows/test.yml`.
- Never run the bench at the same time as `npm test` or `npm run e2e`.
- Before each round, wait until system CPU load reads under 35%, and log the load seen.
- Rounds: one warm-up, then 3 in PR mode or 5 in release mode. Two builds alternate `a, b, b, a, a, b, ...`.
- Projects come from `writeBigProject`: 5 features per module (2,000 tests) always, plus 25 (10,000 tests) in release mode.
- Aims: click to paint under 100 ms and no frame over 50 ms, the same idle and during a run.
- The first limits come from `develop` at `e3cfe3b7`, measured in release mode on the reference machine (Windows 11, Ryzen 7 5800X, 32 GB: the machine this repo's checks ran on).
- A worse number never fails the bench or the `perf` job. Only a failure to measure does.
- Commits follow AGENTS.md (Conventional Commits, `bench` is a new scope, `ci`, `docs`). Every PR says `Part of #147`, and the last one says `Closes #147`.

## Review Focus

1. **A build the bench cannot fully drive** (the base or the previous release has an older test id). The affected measures read `null` and show `n/a`. The bench neither crashes nor waits forever. Pinned in Task 1 (`judge` with `null` medians) and Task 4 (probe timeout returns `null`).
2. **A machine that never gets quiet** (a busy CI VM). `waitForQuiet` gives up after 5 minutes, logs the load, and the round goes ahead. Pinned in Task 3.
3. **Processes left behind after a round that crashed.** Every Grace and Java process the bench started is gone when it exits, failed rounds included. Pinned in Task 4 (`stop` kills the tree) and Task 5 (verification counts processes).
4. **A stale comment after a cancelled run.** The PR comment names the two commits it measured, so a comment from an older head can never pass for the current one. Pinned in Task 7.
5. **A measure with a limit that the current build did not produce** (a probe renamed or broken). It reads `not measured` and counts as a miss in release mode, never as a silent pass. Pinned in Task 1.

---

### Task 1: Measure catalogue and judge

**Files:**
- Create: `bench/measures.mjs`, `bench/judge.mjs`
- Test: `bench/judge.test.ts`

**Interfaces:**
- Produces:
  - `MEASURES: Record<string, { unit: "MB" | "ms" | "count" | "fps", release?: true }>` in `bench/measures.mjs`. The ids, in table order:
    - `size.unpacked`, `size.zip`
    - `start.paint.fresh`†, `start.tree.fresh`†, `start.paint.reused`, `start.tree.reused`
    - `idle.memory`, `idle.cpu.shown`, `idle.cpu.minimised`†
    - `run.memory.app`, `run.memory.java`, `run.cpu.app`, `run.cpu.java`, `run.finishedLag`
    - `smooth.{select,expand,http}.{idle,run}.paint`, `smooth.{select,expand,http,scroll}.{idle,run}.frame`, `smooth.run.fps`, `smooth.run.frame`
    - `page.nodes.open`, `page.nodes.run`, `page.heap.open`, `page.heap.run`
    - Every `run.*`, `smooth.*` and `page.*` id again with the suffix `@10k`†

    † means `release: true`. Units: memory and size in MB, CPU time and times in ms, nodes as a count, `smooth.run.fps` in fps.
  - Report shape (JSDoc `@typedef Report`): `{ mode: "pr" | "release", machine: string, builds: { a: { label: string, exe: string } | null, b: { label: string, exe: string } }, rounds: { build: "a" | "b", warmup: boolean, load: number, values: Record<string, number | null> }[], medians: { a: Record<string, number | null> | null, b: Record<string, number | null> } }`.
  - Limits shape (`@typedef Limits`), stored in `bench/limits.json`: `{ machine: string, commit: string, measures: Record<string, { unit: string, limit: number, noise: number, aim?: number, better?: "higher", machine: boolean }> }`. `better` is absent for lower-is-better measures.
  - `judge(report: Report, limits: Limits): { result: "pass" | "miss", rows: Row[] }`, with `Row = { id, a: number | null, b: number | null, change: number | null, limit: number | null, aim: number | null, verdict: "pass" | "better" | "worse" | "over limit" | "not measured" | "n/a" }`.
  - `makeLimits(report: Report, meta: { machine: string, commit: string }): Limits`, built from build `b`'s non-warm-up rounds.
  - `raisedLimits(base: Limits | null, head: Limits | null): string[]`: the ids whose limit is looser in `head` or missing from it.

- [ ] **Step 1: Write the failing tests** in `bench/judge.test.ts`, with small hand-built reports:
  - `judge` in release mode: `idle.memory` b 96, limit 95 gives `"over limit"` and result `"miss"`.
  - In PR mode, the same row with `machine: true` gives `limit: null` and verdict `"pass"` (machine measures are not judged off the reference machine). With `machine: false` (`page.nodes.open`) it gives `"over limit"`.
  - a 80, b 90, noise 5 gives `"worse"` and result `"miss"` in release mode. b 84 gives `"pass"`, and b 70 gives `"better"`.
  - `better: "higher"` (`smooth.run.fps`, limit 60): b 55 gives `"over limit"`.
  - a `null`, b 90, limit 95 gives `"n/a"` with `change: null`, judged on the limit only.
  - b `null` with a limit gives `"not measured"`, a miss in release mode and `"n/a"` in PR mode.
  - An id in the medians but not in the limits gives a row with `limit: null`, compared with a 5% noise.
  - `makeLimits`: rounds b = 100, 104, 98 for `idle.memory` give noise `ceil(max(6, 5))` = 6 and limit 106 (median 100). The `smooth.select.idle.paint` aim is 100 and the `smooth.scroll.run.frame` aim is 50. `size.*` and `page.*` get `machine: false`, everything else `machine: true`. `smooth.run.fps` gets `better: "higher"` and limit median − noise. Warm-up rounds are ignored.
  - `raisedLimits`: a lower-is-better limit 95 → 96 lists the id, 95 → 90 lists nothing, a removed id is listed, and `base` `null` gives `[]`.

- [ ] **Step 2: Run them to verify they fail**

Run: `npx vitest run bench/judge.test.ts`
Expected: FAIL, cannot resolve `./judge.mjs`.

- [ ] **Step 3: Implement `bench/measures.mjs` and `bench/judge.mjs`**

Rules the tests do not spell out:
- `noise = ceil(max(range of rounds, 5% of median, floor))`, with a floor of 1 for MB and count and 5 for ms, rounded to the unit.
- A row is judged against its limit when `report.mode === "release"` or `!limit.machine`.
- `"worse"` and `"better"` need `a` and `b` both non-null, with a move larger than the noise.
- The result is `"miss"` when any row is `"over limit"`, `"worse"` or `"not measured"`. Only release mode prints that result (Task 5).

- [ ] **Step 4: Run the tests to verify they pass**

Run: `npx vitest run bench/judge.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add bench/measures.mjs bench/judge.mjs bench/judge.test.ts
git commit -m "feat(bench): judge a bench report against its limits"
```

### Task 2: Gates in check-pr

**Files:**
- Modify: `scripts/check-pr.mjs` (two new exports, wired into `main()`), the header comment
- Test: `scripts/check-pr.test.ts`

**Interfaces:**
- Consumes: `judge`, `raisedLimits` (Task 1).
- Produces:
  - `limitRaiseError(base: string | null, head: string | null, added: string[]): string | null`. `base` and `head` are the `bench/limits.json` texts (`null` when the file is missing), and `added` lists the files the PR adds. The function returns `null` when nothing is raised, or when some path in `added` starts with `docs/decisions/`. Otherwise it returns `` `bench/limits.json raises ${ids.join(", ")} without a new docs/decisions/ record (AGENTS.md)` ``.
  - `performanceError(head: string, base: string, files: string[], read: (f: string) => string, limits: string | null): string | null`. Same scope and record lookup as `publicDocsError`. The JSON is the record path with `.md` replaced by `.bench.json`.

- [ ] **Step 1: Write the failing tests** (the spec's Gherkin):
  - `limitRaiseError`: a raised limit with no decision file fails and names the id. The same with `docs/decisions/0024-x.md` in `added` passes. A lowered limit passes. A missing base file passes.
  - `performanceError` returns `null` for `feature/x` into `develop`.
  - For `release/v1.0.0-rc.7` into `main`, it fails when:
    - the record has no `## Performance` section
    - there is no `.bench.json` beside the record
    - there is no `Result:` line
    - the JSON judges a miss but the record says `Result: pass` (the message names the measure)
    - the record says `Result: miss` with no `Decision: go`
    - the record says `Decision: no-go`
  - It passes for `Result: miss` with an `### Analysis` heading and `Decision: go`, and for a JSON that judges a pass with `Result: pass`.
  - In the existing temp-repo `main()` test, a release record with a `## Public docs` section but no `## Performance` section now fails.

- [ ] **Step 2: Run them to verify they fail**

Run: `npx vitest run scripts/check-pr.test.ts`
Expected: FAIL, `limitRaiseError` is not exported.

- [ ] **Step 3: Implement both functions and wire them into `main()`**
  - Base limits: `git show ${BASE_SHA}:bench/limits.json` (`null` on error). Head limits: `git show ${HEAD_SHA}:bench/limits.json`.
  - Added files: `git diff --name-only --diff-filter=A ${BASE_SHA} ${HEAD_SHA}`.
  - Gate rule: fail when the JSON judges `"miss"` and the record says `pass`, or when the record says `miss` without `Decision: go`. A record that says `miss` while the JSON judges `pass` is allowed: it is the conservative side.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `npx vitest run scripts/check-pr.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add scripts/check-pr.mjs scripts/check-pr.test.ts
git commit -m "ci(checks): gate release performance and raised limits"
```

### Task 3: Process sampling on Windows

**Files:**
- Create: `bench/system.mjs`
- Test: `bench/system.test.ts`

**Interfaces:**
- Produces:
  - `sample(rootPid: number): Promise<{ app: { privateMB: number, cpuMs: number }, java: { privateMB: number, cpuMs: number } }>`. It sums the whole process tree under `rootPid`. `java.exe` and its `conhost.exe` count as `java`, everything else as `app`. Private MB is the `WorkingSetPrivate` the spike's `treeMemory` reads. CPU is `KernelModeTime + UserModeTime` from `Win32_Process`, in ms.
  - `startSampler(rootPid: number, everyMs = 500): { stop(): Promise<{ appPeakMB: number, javaPeakMB: number }> }`. It runs one long-lived `powershell.exe` loop that prints a JSON line each tick, never one PowerShell per sample.
  - `cpuLoad(): Promise<number>`: the system-wide percentage.
  - `waitForQuiet(opts?: { under?: number, maxMs?: number, everyMs?: number, read?: () => Promise<number> }): Promise<{ load: number, waitedMs: number }>`, with defaults `under: 35`, `maxMs: 300_000`, `everyMs: 2_000`.
  - `minimise(pid: number): Promise<void>`: a `ShowWindow(MainWindowHandle, 6)` call through PowerShell `Add-Type`.
  - `folderMB(dir: string): number`.

- [ ] **Step 1: Write the failing tests:**
  - `sample` on a spawned `node -e` child that holds a 200 MB `Buffer.alloc` and spins for 1 s reports `app.privateMB >= 190` and `app.cpuMs >= 500`, with `java` at 0.
  - `startSampler` on the same child reports `appPeakMB >= 190`.
  - `waitForQuiet({ read: fake([80, 80, 20]), everyMs: 1 })` resolves `{ load: 20 }`.
  - `waitForQuiet({ read: () => 90, maxMs: 50, everyMs: 10 })` resolves with `load: 90` and `waitedMs >= 50`.
  - `folderMB` on a temp folder holding a 2 MiB file returns 2.1 (MB = 10⁶ bytes, one decimal place).
  - Every test kills its child in `finally`.

- [ ] **Step 2: Run them to verify they fail**

Run: `npx vitest run bench/system.test.ts`
Expected: FAIL, cannot resolve `./system.mjs`.

- [ ] **Step 3: Implement `bench/system.mjs`**, porting `treeMemory` from `docs/research/2026-10-29-tauri-spike/measure/measure.mjs`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `npx vitest run bench/system.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add bench/system.mjs bench/system.test.ts
git commit -m "feat(bench): sample a process tree's memory and cpu"
```

### Task 4: App driver and page probes

**Files:**
- Create: `bench/app.mjs`, `bench/probes.mjs`
- Port from: `docs/checks/2026-10-30-virtual-lists-perf/perf.spec.ts` (`instrument`, `clickToPaint`, `busy`, the frame log, the "Finished after Karate" method) and `scroll.spec.ts` (`frames`, the wheel loop). Leave those dated files as they are.

**Interfaces:**
- Consumes: `startSampler`, `sample` (Task 3) for the callers.
- Produces:
  - `startApp(exe: string, opts: { project: string, dataDir?: string, cdp: boolean, port?: number }): Promise<App>`, with `App = { pid: number, t0: number, dataDir: string, page: Page | null, cdp: CDPSession | null, stop(opts?: { keepData?: boolean }): Promise<void> }`.
    - Before the launch, it writes `settings.json` to `dataDir`: `{ lastProject: project, updates: { checkAtStartup: false }, window: { x: 40, y: 40, width: 1400, height: 900, maximized: false } }`.
    - It sets `GRACE_USER_DATA=dataDir` and, when `cdp`, `--remote-debugging-port`, then retries `chromium.connectOverCDP` for up to 60 s.
    - `stop` runs `taskkill /PID <pid> /T /F`, waits until no process of the tree is left, and removes `dataDir` unless `keepData`.
  - `startTimes(app: App): Promise<{ paintMs: number, treeMs: number }>`, the spike's `timingRun` method, measured from `t0`.
  - `instrument(page: Page): Promise<void>`.
  - `clickToPaint(page: Page, target: Target, done: Done): Promise<number | null>`, with the harness's `Target` and `Done`. It returns `null` when the target is not found within 5 s.
  - `scrollTree(page: Page, notches = 60): Promise<{ frameMs: number | null }>`.
  - `framesDuring(page: Page, fn: () => Promise<void>): Promise<{ fps: number, frameMs: number }>`.
  - `pageStats(cdp: CDPSession): Promise<{ nodes: number, heapMB: number }>`.
  - `finishedLagMs(page: Page, eventsFile: string): Promise<number | null>`.

- [ ] **Step 1: Implement both files.** Probes find rows by the selectors in the harness (`[data-testid=tree]` with `data-rows`, `[data-testid=tree-node]`, `[data-key="d:tests"]`, `tab-http`, `http-list`, `run-all-button`, `run-outcome`). A selector that is missing means `null`, never a thrown error.

- [ ] **Step 2: Smoke-check the driver** with a throwaway script from the scratchpad (not committed). It runs `npm run package`, then calls `startApp` on `out/Grace-win32-x64/Grace.exe` with a `writeBigProject(dir, 5)` project and `cdp: true`, waits for `[data-testid=tree]`, prints `startTimes`, and calls `stop()`.
Expected: paint and tree times print. Afterwards, `Get-Process Grace,java -ErrorAction SilentlyContinue | Where-Object Path -like "*out\Grace-win32-x64*"` prints nothing.

- [ ] **Step 3: Commit**

```bash
git add bench/app.mjs bench/probes.mjs
git commit -m "feat(bench): drive a grace exe over cdp and probe its page"
```

### Task 5: The bench command

**Files:**
- Create: `bench/stats.mjs`, `bench/report.mjs`, `bench/bench.mjs`
- Test: `bench/stats.test.ts`, `bench/report.test.ts`
- Modify: `package.json` (script `"bench": "npm run package && node bench/bench.mjs"`), `.gitignore` (`bench-out/`)

**Interfaces:**
- Consumes: Tasks 1, 3 and 4, and `writeBigProject(dir, featuresPerModule)` imported straight from `../e2e/big-project.ts`. Node 24.15 strips its types; the import was checked to work.
- Produces:
  - `median(xs: number[]): number | null` (ignores `null`s; `null` when none are left).
  - `order(rounds: number, builds: ("a" | "b")[]): ("a" | "b")[]`: one warm-up per build first, then `a, b, b, a, a, b, ...`, or `b` alone when only one build is given.
  - `markdown(report: Report, verdict: ReturnType<typeof judge>): string`.
  - CLI: `node bench/bench.mjs [--b <exe>] [--a <exe>] [--a-zip <zip>] [--b-zip <zip>] [--label-a <text>] [--label-b <text>] [--release] [--rounds <n>] [--out <dir>]`. `--b` defaults to `out/Grace-win32-x64/Grace.exe`, `--out` to `bench-out`, and `--rounds` to 3 (5 with `--release`). It writes `<out>/bench.json` (a `Report`) and `<out>/bench.md`.

- [ ] **Step 1: Write the failing tests:**
  - `median([3, null, 1, 2])` is 2, and `median([null])` is `null`.
  - `order(3, ["a", "b"])` is `["a", "b", "a", "b", "b", "a", "a", "b"]` (warm-ups `a, b`, then three rounds each, alternating).
  - `order(2, ["b"])` is `["b", "b", "b"]`.
  - `markdown` puts the two labels and the machine in the header, has one row per measure in `MEASURES` order, writes `n/a` for `null`, shows `–` in the limit column when `row.limit` is `null`, and ends with `Result: miss` in release mode but has no `Result:` line in PR mode.

- [ ] **Step 2: Run them to verify they fail**

Run: `npx vitest run bench/stats.test.ts bench/report.test.ts`
Expected: FAIL, the modules are missing.

- [ ] **Step 3: Implement the three files.** One round of one build, in order:
  1. `waitForQuiet`, recording the load.
  2. Start: a fresh data folder, `startApp` with CDP and `startTimes`, then `stop({ keepData: true })`. Its figures are `start.*.fresh`, kept in release mode only. A second start on the same folder gives `start.*.reused`.
  3. Idle: `startApp` without CDP on that folder. Wait 20 s, read `idle.memory` from `sample`, then take the `idle.cpu.shown` delta over 60 s. In release mode, `minimise`, then the `idle.cpu.minimised` delta over 60 s. Then `stop`.
  4. Interactive, on the 2,000-test project:
     - Start with CDP and `instrument`.
     - Idle probes: select the 5th test row, close then open the top folder, scroll 60 notches, open the HTTP tab, then back to the Console tab.
     - `page.*.open`, then `startSampler`, then click Run all.
     - Once `run-outcome` reads `Running`, run the same probes as `*.run` inside `framesDuring` until `Finished`.
     - Then `run.finishedLag`, `run.memory.*` from the sampler, `run.cpu.*` deltas, `page.*.run`, and `stop`.
  5. In release mode, step 4 again on the 10,000-test project, with ids suffixed `@10k`.

  If a run reaches `Finished` before the during-run probes end, the probes that did not run read `null`. A probe that returns `null` leaves its measure `null`. The command exits 1 only when an app does not start or Run all never finishes (a 30 min timeout). It stops every app in `finally`. Size comes from `folderMB(dirname(exe))` and the zip file's size.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `npx vitest run bench/stats.test.ts bench/report.test.ts`
Expected: PASS

- [ ] **Step 5: Run the real bench once**

Run: `npm run bench -- --rounds 1`
Expected: exit 0. `bench-out/bench.md` has a value for every non-release measure. Afterwards no `Grace.exe` or `java.exe` under `out\Grace-win32-x64` is running.

- [ ] **Step 6: Run a comparison against an older build**, to pin Review Focus 1. Download `Grace-win32-x64-1.0.0-rc.6.zip` with `gh release download v1.0.0-rc.6 -p "*.zip" -D <scratch>` and unzip it to the scratchpad.
Run: `node bench/bench.mjs --a <scratch>/Grace.exe --label-a "1.0.0-rc.6" --label-b "this branch" --rounds 1`
Expected: exit 0. Measures rc.6 cannot give show `n/a` in column A.

- [ ] **Step 7: Commit**

```bash
git add bench/stats.mjs bench/report.mjs bench/bench.mjs bench/stats.test.ts bench/report.test.ts package.json .gitignore
git commit -m "feat(bench): add npm run bench"
```

### Task 6: Lock in today's limits

**Files:**
- Create: `bench/limits.json`, `docs/checks/<date>-bench-baseline.md`
- Modify: `bench/bench.mjs` (a `--limits-from <bench.json> --commit <sha>` mode that writes `bench/limits.json` with `makeLimits`)

**Interfaces:**
- Consumes: `makeLimits` (Task 1).

- [ ] **Step 1: Build `e3cfe3b7`** in a temporary worktree: `git worktree add <scratch>/base e3cfe3b7`, then `npm ci`, `npm run vendor` and `npm run make` there.
- [ ] **Step 2: Measure it in release mode on the reference machine.** Keep the machine quiet: no other agent sessions, no `npm test`.
Run: `node bench/bench.mjs --release --b <scratch>/base/out/Grace-win32-x64/Grace.exe --b-zip <its zip> --label-b "develop e3cfe3b7" --out <scratch>/baseline`
Expected: exit 0, and every measure in `MEASURES` has a value.
- [ ] **Step 3: Write the limits.**
Run: `node bench/bench.mjs --limits-from <scratch>/baseline/bench.json --commit e3cfe3b7`
Expected: `bench/limits.json` lists every measure, with `machine` set to `"Windows 11 Pro, Ryzen 7 5800X, 32 GB"`.
- [ ] **Step 4: Write the check record** `docs/checks/<date>-bench-baseline.md`. Like the other records, it gives the date, commit, machine, what was run, the table, the loads seen per round, and which measures miss their aims today. Remove the worktree with `git worktree remove`.
- [ ] **Step 5: Commit**

```bash
git add bench/limits.json bench/bench.mjs docs/checks/
git commit -m "feat(bench): lock in today's numbers as limits"
```

### Task 7: The perf workflow

**Files:**
- Create: `.github/workflows/perf.yml`

- [ ] **Step 1: Write `perf.yml`.**
  - **Triggers:** the same `on:` block, `paths-ignore`, back-merge `if:` and `concurrency` as `test.yml`, plus `workflow_dispatch`. `runs-on: windows-latest`, `timeout-minutes: 60`, `permissions: contents: read, pull-requests: write`.
  - **Build both:** check out the head into `head/` and `github.event.pull_request.base.sha` into `base/`. In each: `npm ci`, then `npm run vendor` (vendor cache keyed per folder, as in `test.yml`), then `npm run package`.
  - **Measure:** from `head/`, run `node bench/bench.mjs --a ../base/out/Grace-win32-x64/Grace.exe --label-a "base <short base sha>" --label-b "head <short head sha>" --out bench-out`.
  - **Publish:** append `bench-out/bench.md` to `$GITHUB_STEP_SUMMARY`. Then, unless the PR comes from a fork or `github.actor` is `dependabot[bot]`, post or update one comment whose body starts with `<!-- grace-perf -->`. Find it with `gh api repos/$GH_REPO/issues/$PR/comments --paginate`, then PATCH it, or POST when none exists. Upload `bench-out/` as an artifact.
  - The job's result is the bench's exit code. Nothing reads the numbers.
- [ ] **Step 2: Push and check it on this PR.** The `perf` run finishes green, and the comment shows both short SHAs. Push an empty commit: the same comment is updated with the new head SHA, and no second comment appears.
- [ ] **Step 3: Commit**

```bash
git add .github/workflows/perf.yml
git commit -m "ci(perf): post bench numbers on every code pr"
```

### Task 8: merge-when-green waits for perf

**Files:**
- Modify: `.github/workflows/merge-when-green.yml`

- [ ] **Step 1: Add `perf` to `workflow_run.workflows`.** After the `test.yml` block, add a `perf.yml` block of the same shape. It needs `latest perf.yml`, and a missing run is allowed only when every changed file is under `docs/` or is `*.md` (the existing loop). Otherwise the run must be `success`, or `skip "perf has not posted numbers on $sha"`. Back-merges already filter on `status=success` from the PR into `main`. Update the header comment to name `perf` and ADR 0023.
- [ ] **Step 2: Check it.** `merge-when-green` runs from `develop`, so the check comes after this PR merges. On the next labelled code PR, the merge happens only after its `perf` run finishes, and the run log shows the `perf has not posted numbers` notice while it is still running. Record that in the PR that follows.
- [ ] **Step 3: Commit**

```bash
git add .github/workflows/merge-when-green.yml
git commit -m "ci(merge): wait for perf numbers before merging"
```

### Task 9: Settle the rule in the docs

**Files:**
- Create: `docs/decisions/0023-run-lightly-as-a-release-requirement.md`
- Modify: `AGENTS.md`

- [ ] **Step 1: Write ADR 0023** in the MADR minimal shape of ADR 0020: front matter `status: accepted`, `date`, `decision-makers: Tusquito`, then Context, Considered Options (the spec's A, B and C), Decision Outcome (B) and Consequences. It holds the spec's R7 content. Its last paragraph narrows ADR 0011: "This narrows ADR 0011: a labelled PR into `develop` also waits until `perf` has finished with success on its head. Raising a limit in `bench/limits.json` needs a new decision record."
- [ ] **Step 2: Edit AGENTS.md:**
  - **Commands table:** add `npm run bench` (package the app and measure it; `-- --a <exe>` compares with another build; `--release` runs the release benchmark). Extend the no-parallel line to the bench.
  - **Doc rules, the spec line:** specs also have a `## Cost` section.
  - **A new `## Performance` section,** in the spec's R6 words: the order of priorities, the `## Cost` section, idle costing nothing, reading the perf comment and explaining a worse number in the PR body, never writing a release's `Decision:` line, that raising a limit needs a decision record, and that a stack change is judged first on running lightly, measured with the bench against the current build (R1.3, #152).
  - **Git workflow, beside the public-docs rule:** the release benchmark, on the reference machine named in `bench/limits.json`. Run `npm run make`, download and unzip the previous release's zip, then run `node bench/bench.mjs --release --a <prev exe> --a-zip <prev zip> --b-zip out/make/zip/win32/x64/<zip> --label-a <prev version> --label-b <version>`. Copy `bench-out/bench.json` to `docs/checks/<record>.bench.json` and paste `bench.md` under `## Performance`. On a miss, write `### Analysis` and stop for the owner's `Decision:`. `scripts/check-pr.mjs` enforces it.
  - **Workflows table:** add the `perf` row and the `merge-when-green` change.
- [ ] **Step 3: Check the docs.** Run `npm run lint`: it passes. Every path the new text names exists.
- [ ] **Step 4: Commit**

```bash
git add docs/decisions/0023-run-lightly-as-a-release-requirement.md AGENTS.md
git commit -m "docs(agents): make running lightly a release requirement"
```

### Delivery

Two PRs into `develop`, each with `npm run e2e` run locally and its summary pasted:
- Tasks 1 to 6 ("feat(bench)"), `Part of #147`. Move #147's card to `building` when it opens (AGENTS.md).
- Tasks 7 to 9 ("ci: perf numbers on every pr"), `Closes #147`. This one's own `perf` run is the check for Task 7.
