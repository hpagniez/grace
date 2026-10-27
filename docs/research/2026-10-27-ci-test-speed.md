# CI test speed: why the `test` workflow takes 20 minutes, and how to cut it

Checked on 2026-10-27 against the `test` runs of PR #31 ([run 40584412059](https://github.com/Tusquito/karate-kid/actions/runs/40584412059/job/122471971018)) and PR #30 ([run 40582544583](https://github.com/Tusquito/karate-kid/actions/runs/40582544583)), the test code on `develop` (9c45f75), GitHub's runner docs, the Vitest 5.0.5 and `@solidjs/signals` 2.0.0-rc.18 sources in `node_modules/`, and a local timing run pinned to 2 CPUs. Backlog rows: F44 (CI speed) and F45 (Solid warnings) in [features.md](../features.md).

**Bottom line:** one file, `src/main/runner.test.ts`, takes 422 s of the 468 s `npm test` step, and the e2e suite then runs for 11 minutes after it, in the same job. The Java tests are CPU-bound: on the 2-CPU runner, two of them at once are only 15 % faster than one after the other. More VMs help; more workers on one VM barely do. Run unit and e2e tests as separate jobs, then split each suite over two jobs. A ready PR goes from about 20 to about 7 minutes, a draft PR from about 8.5 to about 5, for about 3 more billed Windows minutes per ready run, and no test is dropped.

---

## 1. Measurements

### The CI job

The repo is private, so `windows-latest` is GitHub's private-repo runner: **2 CPUs, 8 GB RAM** (public repos get 4 CPUs and 16 GB) ([GitHub-hosted runners](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)). `node-version: 24` resolved to Node 24.22.1 on 2026-10-27.

| Step | PR #31 (743 unit, 114 e2e tests) | PR #30 (718 unit, 103 e2e tests) |
|---|---|---|
| Setup: checkout, Node, caches, `npm ci`, `npm run vendor` | 46 s | 49 s |
| `npm test` | 7 min 50 s | 7 min 39 s |
| Draft check | 7 s | 8 s |
| `e2e` (package, then Playwright) | 11 min 10 s | 10 min 59 s |
| **Whole job** | **20 min 13 s** | **19 min 48 s** |

A draft PR runs only setup and `npm test`: about 8.5 minutes.

### `npm test` by file (PR #31)

| File | Tests | Time |
|---|---|---|
| `src/main/runner.test.ts` | 120 | 422 s |
| `src/main/karate/process.test.ts` | 7 | 6.9 s |
| `src/main/history.test.ts` | 21 | 4.7 s |
| `src/main/karate/argfile.test.ts` | 10 | 2.2 s |
| 13 jsdom files (`*.test.tsx`) | about 160 | about 25 s together |
| 28 other Node files | about 425 | about 3 s together |

`runner.test.ts` drives the real `Runner` class: every test opens a fixture project (a Karate dry run) and most of them run it, so each test starts one to three JVMs. By `describe` block:

| Block | Time |
|---|---|
| Runner history | 162 s |
| Runner runs | 72 s |
| Runner properties | 50 s |
| Runner tag filter | 43 s |
| Runner threads | 31 s |
| Runner environment | 31 s |
| Runner discovery | 15 s |
| Runner home screen | about 13 s |
| Runner flaky | 5 s |

Vitest's footer says `isolate: false` would save "at least ~16.76 s" (45 workers at about 381 ms each): 3.6 % of the step. See [rejected options](#rejected).

### `e2e` by spec (PR #30)

The app is packaged first (about 25 s), then 103 tests run on one worker in about 10.5 minutes. The longest specs: `threads.spec.ts` 94 s, `history.spec.ts` 92 s, `env.spec.ts` 74 s, `console.spec.ts` 59 s (one test of 57 s), `tags.spec.ts` 46 s, `run.spec.ts` 44 s, `http.spec.ts` 42 s, `cancel.spec.ts` 41 s, `flaky.spec.ts` 37 s. The other 8 specs take 94 s together.

### Two Java test processes on 2 CPUs

`runner.test.ts` was run on a 16-core Windows 11 machine with each process tree pinned to the same 2 CPUs (`start /affinity 3`; child processes, the JVMs included, inherit the mask), as on the runner:

| Run | Vitest duration |
|---|---|
| A: the whole file in one process, as CI runs it today | 334 s |
| B: two processes at once, one with discovery, runs, tag filter, threads, environment and properties, the other with history, flaky and home screen | 285 s and 241 s, so 285 s wall time |

B is only 15 % faster than A. Run alone, the halves would take about 190 s and 140 s (57 % and 43 % of A): sharing 2 CPUs made them 1.5 and 1.7 times slower. Most of a Java test is JVM CPU time (start-up, class loading, compiling), not waiting. Four tests in B failed as expected: they check that no Java from `vendor/jre` is running at all (section 2).

---

## 2. Why everything runs one at a time

- `vitest.config.mts` sets `fileParallelism: false`, with the comment "real-Java test files must not overlap: process tests scan every java.exe".
- The tests that scan every `java.exe`:
  - `ourJava()` in `src/main/runner.test.ts`, at lines 408, 425, 452 and 465.
  - "kill leaves no java behind" in `src/main/karate/process.test.ts:81`.
  - `ourJavaPids()` in `e2e/helpers.ts`, used by `cancel.spec.ts`, `env.spec.ts`, `history.spec.ts` and `home.spec.ts`. That is why `playwright.config.ts` has `workers: 1`, and why AGENTS.md says never to run `npm test` and `npm run e2e` at the same time.
- [ADR 0007](../decisions/0007-github-actions-ci-cd-on-the-free-private-plan.md) runs both suites in one Windows job, one after the other, "so install and downloads happen once and the suites never overlap".

Separate jobs get separate VMs, so these global checks cannot see another job's Java. That makes splitting over jobs safe with no change to the tests.

---

## 3. Fixes

### 3.1 Unit and e2e tests in two parallel jobs

Split the one job in `.github/workflows/test.yml` into two jobs in the same workflow:

- `unit`: checkout, setup-node, the Electron and `vendor/` caches, `npm ci`, `npm run vendor`, `npm test`.
- `e2e`: the same setup, then the draft check and the step named `e2e`, as today. Keep the draft check inside the job: the draft flag in the event is stale when a push and "ready for review" land together, which is why the job asks the API today.

`merge-when-green.yml` keeps working without a change: it waits for the whole `test` run, then looks for a step named `e2e` in any of the run's jobs (`.jobs[].steps[]`, line 68).

- Ready PR: about 20 minutes becomes about 12 (the longer job).
- Draft PR: unchanged. The `e2e` job still pays about 50 s of setup before its draft check skips the step.
- Billed: about 1 to 2 more Windows minutes per run, because billing rounds each job up to a whole minute.
- It changes what ADR 0007 decided ("one Windows job runs both suites in turn"), so it needs a new decision record that supersedes that part. Update the comments in `test.yml` and the workflow table in AGENTS.md in the same change.

### 3.2 Unit tests in two jobs

1. **Split `runner.test.ts`** (2,445 lines, 9 `describe` blocks) into files by block, with the shared helpers (`setup`, `fixture`, `until`, `javaCommandLines`, `ourJava` and the others) in a test module next to `test/fixtures/copy.ts`. No test changes.
2. **Run the `unit` job as a matrix of two**, each half with an explicit file list: one gets the history, flaky and home screen files plus every other test file (about 226 s on CI), the other gets the remaining runner files (about 242 s). Do not use `vitest --shard`: Vitest 5.0.5 splits by file count after sorting by a hash of the path (`BaseSequencer.shard`), not by duration, so the runner files could land in the same half.
3. Each half keeps `fileParallelism: false`: within a VM the leak checks stay as they are.

- Draft PR: about 8.5 minutes becomes about 5.
- Ready PR: still about 12 minutes; the `e2e` job is the longer one until 3.3.
- Billed: one more VM, about 1 more Windows minute per run.
- Any later file split must keep the two halves balanced. Check the times in the first CI run.

### 3.3 E2E tests in two jobs

- Run the `e2e` job as a matrix of two: `playwright test --shard=1/2` and `--shard=2/2`. Each half packages the app again (about 25 s).
- `merge-when-green.yml:68` reads only the first `e2e` step (`[...][0].conclusion`). With two halves it must require every `e2e` step to pass, or a red second half could be merged. Change it in the same PR.
- Ready PR: about 12 minutes becomes about 7. Billed: about 1 to 2 more Windows minutes per ready run.
- Playwright shards whole files when `fullyParallel` is off, and the spec files range from 2 s to 94 s, so the halves will not be exactly even. Check the times in the first CI run.

### 3.4 Could: leak checks scoped to each test

This keeps every leak check but lets Java test files run in parallel on one machine. It is worth little on CI (15 % on 2 CPUs, section 1) and more on a developer's machine with many cores.

- `buildArgs` in `src/main/karate/command.ts` always puts `-w <project root>` and `-o <output folder>` on the Java command line, and every test already uses its own temp folders (`copyFixture`, `mkdtempSync`). `javaCommandLines(root)` in `runner.test.ts` already filters by project root.
- Replace the four `ourJava()` checks and the one in `process.test.ts` with checks for the test's own folders.
- Give each Java test file one `afterAll` that fails if any `java.exe` command line still holds one of the file's temp folders. Today only five tests check for leaks, and a leak from an earlier test fails whichever test checks next. After this, every test is covered and a failure names the right file.
- Then remove `fileParallelism: false` and set `maxWorkers` explicitly. Vitest's default in run mode is the CPU count minus one (`getDefaultMaxWorkers`): 1 worker on a 2-CPU runner.
- Scope `ourJavaPids()` in `e2e/helpers.ts` the same way, and the AGENTS.md rule "never run `npm test` and `npm run e2e` at the same time" can go.
- Nothing else is shared between test files: the HTTP mock gets a free port (`karate.start` with no port), `runner.test.ts:1048` changes `process.env.KARATE_ENV` only in its own worker process (Vitest's default `forks` pool), and `vendor/` is only read.

### 3.5 Not measured: cheaper JVM start

Because the Java tests are CPU-bound, starting each JVM with less work would speed up CI and the app alike: the app runs a dry run on every open and refresh. JDK 21 can keep an application class-data archive for the Karate jar (`-XX:+AutoCreateSharedArchive -XX:SharedArchiveFile=<file>`). That changes the app's Java command line and writes a file next to the app's data, so it needs a spike with timings and a decision record first.

### Not now: idle time in the `clean` fixture

`test/fixtures/projects/clean/tests/slow.feature` sleeps 8.5 s, so every "Run all" on the `clean` project waits for it, in unit and e2e tests alike. Cancel tests need a run that is still going, and a shorter sleep makes them race the end of the run. Leave it.

### Rejected

- **`isolate: false`**: saves about 17 s. It shares loaded modules and globals (jsdom, Solid's runtime, `vi.mock`) across test files, so a test could pass or fail depending on which file ran before it.
- **More workers on one runner**: the Java tests are CPU-bound (section 1), and Playwright `workers: 2` would also put two Electron windows and their JVMs on 2 CPUs, which makes the window, focus and timing tests riskier.
- **Replacing the Java tests with mocks**: they are the only unit tests that prove how the app drives the real Karate CLI (AGENTS.md, Karate integration rules).
- **Linux runners**: the tests need `taskkill`, the PowerShell leak checks and the Windows JRE (ADR 0007).
- **Public-repo runners (4 CPUs)**: the repo stays private for now (ADR 0007).

### Expected result

Estimates from CI's own step times; billed minutes are per run, with each job rounded up to a whole minute. ADR 0007 budgets about 1,000 Windows minutes a month.

| | Ready PR, wall time | Draft PR, wall time | Ready PR, billed | Draft PR, billed |
|---|---|---|---|---|
| Today | about 20 min | about 8.5 min | about 21 min | about 9 min |
| 3.1 | about 12 min | about 8.5 min | about 22 min | about 10 min |
| 3.1 and 3.2 | about 12 min | about 5 min | about 23 min | about 11 min |
| 3.1, 3.2 and 3.3 | about 7 min | about 5 min | about 24 min | about 12 min |

---

## 4. Solid warnings in the `npm test` log

The PR #31 log prints 26 `[STRICT_READ_UNTRACKED]` warnings and one `[HUGE_FAN_IN]`. They cost no measurable time: the files that print them run in about 1 s each. They appear on CI (Node 24.22.1) but not locally with Node 24.15.0, with the same `package-lock.json` and the same `develop` commit. The cause of that difference was not traced. `node-version: 24` in `test.yml` follows the newest Node 24 while each machine keeps its own.

| Warning | Where | Cause | Verdict |
|---|---|---|---|
| `STRICT_READ_UNTRACKED` "count" (8) | `src/renderer/Threads.tsx:16` | The effect watches `props.state.threads` but reads `.count` in its second, untracked function | Fix. It works today only because `reduce.ts` puts a fresh object in `threads` for every `threads` event; a change of `count` in place would leave the box stale |
| `STRICT_READ_UNTRACKED` "at", "9999", "setup.lines" and unnamed (18) | `src/renderer/Console.test.tsx`, the fake `scrollHeight` getters | jsdom has no layout, so the test defines `scrollHeight` as a getter that reads a signal or the store. `Console.tsx` reads `el.scrollHeight` in its effect's untracked function, which calls the getter | Fix the test. `Console.tsx` is right: a real `scrollHeight` is not reactive |
| `HUGE_FAN_IN` "errors", 4007 sources (1) | `src/renderer/PropertiesDialog.tsx:74` | Only in the test "a list of more than 500 rows blocks Save" (501 rows). The memo reads every row to find duplicate names | Keep. Finding duplicates needs every name, and `MAX_ROWS = 500` caps the list |

Fixes:

```tsx
// src/renderer/Threads.tsx: read count where the effect tracks it. The compute
// returns a fresh object, so a count main re-sends unchanged still resets the box.
createEffect(
  () => ({ count: props.state.threads?.count ?? 1 }),
  ({ count }) => {
    box.value = String(count);
  },
);
```

```tsx
// src/renderer/Console.test.tsx: a fake layout value is a snapshot, not a reactive read.
// Same change for all three scrollHeight getters (lines 17, 38 and 67 on develop 9c45f75).
import { createSignal, flush, untrack } from "solid-js";
Object.defineProperty(el, "scrollHeight", { get: () => untrack(() => lines().length * 10) });
get: () => untrack(() => (Number(store.state.console.at(-1)?.slice(1) ?? -1) + 1) * 10),
Object.defineProperty(el, "scrollHeight", { get: () => untrack(() => (store.state.console.length + 1) * 10) });
```

`untrack(fn)` closes the strict-read window while `fn` runs (`strictRead = strictReadLabel || false` in `@solidjs/signals` `dev-shared.js`), and a read in an effect's compute function is tracked, so neither fix can warn. The Threads fix and the first two getters were tried on commit 49caf53 (before the third getter existed): `Threads.test.tsx` and `Console.test.tsx` passed, 9 tests. The warnings do not show on Node 24.15.0, so the proof that they are gone is the CI log of the fix's PR: no `[STRICT_READ_UNTRACKED]` line.

Optional, for the spec to decide: fail the jsdom tests on any Solid diagnostic (a `console.warn` spy in `test/jsdom-setup.ts`), with the `HUGE_FAN_IN` of the 500-row test allowed, so new warnings cannot pile up unseen.

---

## Sources

- GitHub Actions runs: [40584412059](https://github.com/Tusquito/karate-kid/actions/runs/40584412059/job/122471971018) (PR #31), [40582544583](https://github.com/Tusquito/karate-kid/actions/runs/40582544583) (PR #30). Step times from `gh api repos/Tusquito/karate-kid/actions/jobs/<id>`, test times from the job logs.
- [GitHub-hosted runners](https://docs.github.com/en/actions/reference/runners/github-hosted-runners): Windows, private repositories: 2 CPUs, 8 GB RAM, 14 GB SSD.
- Vitest 5.0.5, `node_modules/vitest/dist/chunks/index.*.js`: `getDefaultMaxWorkers` (CPU count minus one in run mode) and `BaseSequencer.shard` (split by count after sorting by path hash).
- `@solidjs/signals` 2.0.0-rc.18, `node_modules/@solidjs/signals/dist/dev-shared.js` (`warnStrictReadUntracked`, `untrack`) and `dev.js` (the effect callback's strict-read window); `node_modules/solid-js/skills/reactivity-diagnostics/SKILL.md`.
- Repo: `.github/workflows/test.yml`, `.github/workflows/merge-when-green.yml`, `vitest.config.mts`, `playwright.config.ts`, `src/main/runner.test.ts`, `src/main/karate/process.test.ts`, `src/main/karate/command.ts`, `e2e/helpers.ts`, `test/fixtures/copy.ts`, ADR 0007, ADR 0011.
