# Light and fast: numbers on every PR, a benchmark before every release

Date 2026-10-31. Status: approved. Covers [#147](https://github.com/hpagniez/grace/issues/147).

## Intent

Grace should be known for running lightly. A developer keeps it open all day beside IDEs, browsers, containers and chat, and Grace must never be the reason the machine feels slow. Speed comes second: a quick start and an instant response. Download size comes third: smaller is better, but never at the cost of running lightly, because a download happens once and daily use happens every day.

Nothing guards this today. The [big-project note](../research/2026-10-29-big-project-perf/README.md) and the [virtual-lists check](../checks/2026-10-30-virtual-lists-perf.md) measured Grace by hand, once. There is no limit, and no CI job measures anything, so a PR can double Grace's memory or start time and still pass. This spec makes the promise part of how Grace is built: numbers on every PR, a benchmark before every release, and a release that waits when a number gets worse.

## Who and what success looks like

- **People who run Grace beside heavy tools all day.** Idle, Grace uses close to no CPU and little memory. During a run, the screen responds as it does when idle, and their other apps do not stutter.
- **Agents and reviewers who build Grace.** Every code PR shows what it costs before it merges. No release ships a regression that nobody explained and the owner did not accept.

## Goals, non-goals, anti-goals

**Goals:** the requirements below.

**Non-goals:**
- Making Grace lighter. This spec measures and guards, and locks today's numbers in. Fixes get their own issues ([Follow-ups](#follow-ups)).
- A stack change. The [Tauri spike](../research/2026-10-29-tauri-spike/README.md) (#106) found no gain in memory or start time on Windows. The question reopens only with new evidence from this benchmark (R1.3).
- A history chart of every `develop` build (approach C). It can come later.
- macOS and Linux: Grace ships for Windows only.

**Anti-goals:**
- A merge held back because a number is worse. The owner's rule: a merge waits for the numbers to be posted, never for them to be good.
- Limits that drift up quietly.
- A release that ships a worse number with no written analysis and no decision from the owner.
- A benchmark so noisy that people stop reading it.
- An agent approving its own regression.

## Constraints

- Measure what people run: the packaged, minified app on Windows 11.
- GitHub's Windows runners have 4 shared vCPUs, so timings swing from run to run. Two builds are compared on the same VM, in turns, as in the big-project note.
- The packaged exe's `EnableNodeCliInspectArguments: false` fuse blocks Playwright's `_electron` (AGENTS.md). The Tauri spike drove the packaged exe over CDP instead (`--remote-debugging-port`) and opened a project through `GRACE_USER_DATA` and `settings.json`'s `lastProject` (`measure/measure.mjs`). That is the only way to measure a release zip the bench did not build.
- Never run the bench at the same time as `npm test` or `npm run e2e`: the Java leak checks scan every `java.exe` under `vendor/jre`.
- The packaged app checks GitHub for updates at start, and `GRACE_UPDATE_FEED` works only unpackaged. The bench writes `updates.checkAtStartup: false` and the `window` size into the data folder's `settings.json`. CDP gives no access to the main process, so minimising the window takes a Win32 call.
- The bench drives the app through test ids, data attributes and settings keys, and the no-backward-compatibility rule lets any PR rename them. A PR that renames one the bench uses updates the bench in the same PR.
- Workflows use only GitHub-owned actions pinned to a full commit SHA. PRs from forks get a read-only token.

### What exists today

- `docs/research/2026-10-29-big-project-perf/`: `generate.mjs`, plus `perf.spec.ts` and `scroll.spec.ts`. These measure click to paint, long tasks, a frame log, `Performance.getMetrics`, and how far "Finished" lags behind Karate's event file.
- `docs/checks/2026-10-30-virtual-lists-perf/`: the same probes updated for virtual lists, plus `bar.spec.ts`. They launch the built app through `e2e/helpers.ts`, not the exe.
- `docs/research/2026-10-29-tauri-spike/measure/measure.mjs`: first paint, tree shown and idle memory of the whole process tree, over CDP on the exe, with a fresh or reused profile, medians after a warm-up.
- `e2e/big-project.ts`: `writeBigProject(dir, featuresPerModule)`, ported from `generate.mjs`.
- `scripts/check-pr.mjs` `publicDocsError`: fails a release or hotfix PR into `main` whose check record has no `## Public docs` section. R5 extends it the same way.
- `src/main/karate/process.ts` spawns Java with no memory limit, at normal priority.
- The virtual-lists check waited for system CPU load under 35% before each round.
- The latest `develop` is `e3cfe3b7`, with virtual lists merged (#141, #142, #143, #146).

## Approaches considered

- **A. Rules and a release check only.** A decision record, rules in AGENTS.md, `npm run bench`, a limits file, and the release gate. On normal PRs, agents paste numbers from their own machines. Cheapest, but those numbers come from busy machines, and only when someone remembers.
- **B. A, plus a measurement on every PR (chosen).** A CI job builds `develop` and the PR on the same runner, benchmarks both in turns, and posts one comment. It never fails on a number. It costs about 20 minutes of Windows runner per code PR, which is free on a public repo. The owner chose B: the numbers show up without relying on anyone, and measuring both builds on one VM cancels most of the noise.
- **C. B, plus a history chart.** Stores every `develop` result and charts the trend. It needs more infrastructure; it can be added later.

## Requirements

### R1. The promise

1. Grace's priorities, in order:
   1. **Run lightly:** memory, CPU and a smooth screen, idle and during a run, Java included.
   2. **Be fast:** start and response.
   3. **Be small:** download and install size.

   A lower priority never wins over a higher one.
2. The promise is recorded in a decision record (R7) and in AGENTS.md (R6).
3. Any proposal to change the stack (shell, UI framework, runtime) is judged first on running lightly, measured with this benchmark against the current build.

### R2. The benchmark: `npm run bench`

1. One command packages the app and measures it. It can also measure any Grace exe it is given, such as an unpacked release zip or another build. It drives the exe over CDP, so it can compare two builds.
2. It measures the packaged app:

| Group | Measure |
|---|---|
| Size | Unpacked folder size. Zip size, when a zip is given or made. |
| Start | First paint and tree shown, measured from process spawn, with a fresh profile and a reused one. |
| Idle | Private working set (Task Manager's Memory column) of the app's whole process tree, with a project open, after 20 s idle. |
| Idle | CPU time used by the app's process tree over 60 s idle with the window shown, and over 60 s minimised. |
| Run | During Run all on the 2,000-test project: peak private working set of the app (Java excluded) and of Java, and CPU time of each. |
| Run | "Finished" shown after Karate's last test, by the big-project note's method. |
| Smooth | Click to paint and longest frame for four actions: select a test, expand all, open the HTTP tab, scroll the tree 60 notches. Each action is measured idle and during Run all. |
| Smooth | Frame rate and longest frame over the whole Run all. |
| Page | DOM nodes and JS heap after open and at the end of Run all. |

3. The projects come from `writeBigProject`: 2,000 tests on every bench run, with 10,000 tests added in release mode.
4. Each figure is the median of N rounds after one warm-up round: 3 on a PR, 5 at a release. When two builds are compared, rounds alternate (A, B, B, A, A, B, and so on).
   - **Release mode** measures everything in the table.
   - **PR mode** measures a subset, to keep the CI job near 20 minutes: no fresh-profile start, no minimised CPU, and no 10,000-test project. One Run all per round feeds both the Run group and the during-run Smooth probes.
5. Before each round, the bench waits until system CPU load reads under 35%, and logs the load it saw.
6. Output is a JSON file with every round, plus a Markdown table with the columns measure, A, B, change, limit and result. R4 and R5 read the JSON.
7. The bench exits non-zero only when it cannot measure, for example when the app does not start or a probe times out. A worse number never fails it.

### R3. Limits

1. A limits file in the repo gives each R2 measure a limit (a ceiling it must stay under) and, where one exists, an aim.
2. The first limits are today's numbers: `develop` at `e3cfe3b7`, measured in release mode on the reference machine (R5.2). Every limit, Smooth ones included, is that median plus a noise margin, taken from the spread of its rounds and written next to it. A margin keeps a release measured within noise from missing half the time.
3. The aims for the Smooth group: click to paint under 100 ms and no frame over 50 ms, the same idle and during a run. Today misses them in places (the [baseline check](../checks/2026-10-31-bench-baseline.md) records which, at 10,000 tests). A limit is never capped at its aim; limits move toward the aims as fixes land, through ordinary commits (R3.4).
4. Lowering a limit is an ordinary commit. Raising one needs a new decision record: `scripts/check-pr.mjs` fails a PR that raises any limit without adding a file under `docs/decisions/`.
5. Measures that depend on the machine (times, CPU, memory: Java's default heap scales with the machine's RAM) are judged against limits only on the reference machine. Measures that do not (sizes, DOM nodes, JS heap) are judged anywhere.

### R4. Every PR

1. A `perf` workflow runs on PRs into `develop` and `main` that change more than `docs/**` and `*.md`. It skips release and hotfix merges back into `develop`, as `test` does, and it can also be run by hand.
2. On one Windows runner, it builds the PR's base and head and runs `npm run bench` comparing them in PR mode (R2.4).
3. It posts the Markdown table as one PR comment, updates that same comment on later pushes, and also writes it to the job summary. PRs from forks get only the job summary, because their token is read-only.
4. It never fails because a number is worse. It fails only when the bench cannot measure.
5. `merge-when-green` waits for the numbers: it merges a PR only once `perf` has finished with success on its head commit, as well as `checks` and `test`. A worse number never holds a merge. A run that could not measure does, until it is re-run or the bench is fixed, because no numbers were shown. A docs-only PR has no `perf` run and does not wait for one, as with `test`. A release or hotfix merge back into `develop` uses the `perf` run of its PR into `main`, as it does for `checks` and `test`.
6. Time measures show only the change against base. Measures that do not depend on the machine also show their limit (R3.5).
7. When the comment shows a measure worse than base beyond its noise margin, the PR body says why (R6.4).

### R5. Before every release

1. Before a release or hotfix PR into `main`, an agent runs `npm run bench` in release mode on the reference machine, comparing the release build with the previous release's zip from GitHub Releases. Both are measured side by side on the day; the old record's numbers are not reused.
2. The reference machine is the owner's desktop: Windows 11, Ryzen 7 5800X, 32 GB, the machine of the spikes and the checks. If it changes, the next release says so; the side-by-side comparison already re-measures the previous release on the new machine.
3. The packaged-app check record (`docs/checks/<date>-<release|hotfix>-<version>-packaged.md`) gets a `## Performance` section: the machine, the commit, the table, and a line that reads either `Result: pass` or `Result: miss`. The bench writes that line, not an agent, and its JSON is committed beside the record (`<record name>.bench.json`).
4. A miss is a measure over its limit, or a measure worse than the previous release beyond its noise margin. If the bench cannot take a measure on the previous release (for example a test id that has since changed), that measure shows `n/a` against the previous release and is judged on its limit alone.
5. On a miss, the release stops, and the section gets a `### Analysis` subsection. For each measure that missed, it says how much the measure moved, which PR or change caused it, why, and whether it can be fixed. The owner then writes `Decision: go` or `Decision: no-go`. An agent writes the analysis, never the decision.
6. `scripts/check-pr.mjs` fails a release or hotfix PR into `main` when its check record has no `## Performance` section, has no bench JSON beside it, or says `Result: miss` without `Decision: go`. It works out pass or miss itself from the JSON and the limits file, and fails when the record's `Result:` line says otherwise. An agent cannot turn a miss into a pass by typing.
   - Nothing can check who wrote `Decision: go`: agents commit under the owner's git identity and use the owner's `gh` token. That part rests on the AGENTS.md rule (R6.6).
7. A `Decision: go` on a miss names the limits that change. Raising a limit still needs its decision record (R3.4).

### R6. Rules for agents (AGENTS.md)

A short `## Performance` section in AGENTS.md says:
1. The promise and its order (R1).
2. Every design spec has a `## Cost` section. It says what the feature adds to memory, CPU, start time or size, idle and during a run, and how idle stays at zero CPU. The Doc rules line about specs says so too. A PR that adds a dependency states its size.
3. Idle costs nothing: no timer, poll or animation runs while no run is going, or while the window is minimised.
4. Read the PR's perf comment, and explain a worse number in the PR body.
5. Before a release, run the release benchmark (R5). This also goes in the Git workflow section, next to the public-docs check.
6. Never write a release's `Decision:` line.

`npm run bench` is added to the Commands table, and the note against running `npm test` and `npm run e2e` together is extended to cover it.

### R7. Decision record

A new record, `docs/decisions/0023-<title>.md`, says that running lightly is a release requirement. It records the priority order, numbers on every PR that a merge waits for but never judges, the release stop on a miss with an analysis and the owner's decision, and limits that rise only through a decision record. It narrows ADR 0011: a labelled PR into `develop` also waits for `perf` (R4.5). It belongs in a record because it changes the merge and release process, as ADR 0015, ADR 0017 and ADR 0019 did.

## Cost

The bench adds no code to the app. CI adds one Windows job of about 20 minutes per code PR, which is free on a public repo. A labelled merge then takes about 20 minutes after a push instead of about 6. A release adds about an hour on the reference machine: two builds, 5 rounds each, at 2,000 and 10,000 tests.

## Design decisions already made

The owner's:
- Running lightly beats download size.
- CI never blocks a merge on a number. Numbers show on every PR, and a merge waits until they are posted.
- A benchmark runs before each release. A miss stops the release until an analysis is written and the owner decides go or no-go.
- The first limits lock in today's numbers.
- The screen responds the same way whether tests run or not.
- Approach B.

Mine, open to correction:
- Java's memory and CPU are measured as part of Grace.
- The reference machine is the owner's desktop.
- The release gate lives in `check-pr.mjs`, like `## Public docs`. So does the check on raised limits (R3.4).
- 2,000 tests on PRs, with 10,000 added at release.
- The aims are 100 ms and 50 ms: RAIL's response budget and the Long Tasks API's threshold.
- The release baseline is the previous release's zip, re-measured beside the new build.
- A `perf` run that could not measure holds the merge, since no numbers were posted.
- Every limit gets a noise margin, and limits are never capped at their aims.
- PR mode drops the fresh-profile start, the minimised CPU and the 10,000-test project.
- `check-pr.mjs` works out the release result from the committed bench JSON.
- A measure the previous release cannot give shows `n/a` and is judged on its limit.
- The issue's priority label is `must`: the release gate applies before the first stable release.
- The maintained bench replaces `docs/checks/2026-10-30-virtual-lists-perf/` as the way to measure speed. The dated research and check folders stay as they are, because they are records.

## Left to the builder

- Where the bench lives (a `bench/` folder beside `e2e/`, for example), its Playwright config, and the JSON shape.
- How memory and CPU are sampled on Windows, and how often during a run.
- The noise margin for each measure, from the spread of the first rounds.
- How the workflow finds and updates its comment (a marker in the body), and its token permissions (`pull-requests: write` for the comment step only).
- How the base build is made on CI (a second checkout or worktree, `npm ci` in each).
- Whether zip size comes from `npm run make` or from zipping the packaged folder.
- The workflow's timeout.

## Follow-ups

These have their own issues and are not part of this spec:
- [#148](https://github.com/hpagniez/grace/issues/148): Java at below-normal priority, so a run never makes other apps stutter (`os.setPriority` after the spawn in `process.ts`).
- [#149](https://github.com/hpagniez/grace/issues/149): a memory limit for Java, sized from the Java peak this bench records.
- [#150](https://github.com/hpagniez/grace/issues/150): ship only Electron's `en-US` locale. The Tauri spike estimated about −12.6 MB zipped and −50.6 MB unpacked; this was not tested.
- [#126](https://github.com/hpagniez/grace/issues/126): the folder status roll-up, the likely cause of the 333 to 350 ms frames during a 10,000-test run, and the next step toward the 100 ms / 50 ms aims.

## Acceptance criteria

```gherkin
Feature: Light and fast guard

  Scenario: The bench measures the packaged app
    Given the app is packaged
    When I run "npm run bench"
    Then it writes a JSON file and a Markdown table with every measure of R2
    And each figure is the median of its rounds after one warm-up round

  Scenario: The bench compares two builds in turns
    Given two Grace builds A and B
    When I run the bench comparing them
    Then the rounds alternate between A and B
    And the table shows A, B and the change for each measure

  Scenario: The bench measures a release zip it did not build
    Given the previous release's zip, unpacked
    When I run the bench on its exe
    Then it measures that exe over CDP

  Scenario: The bench waits for a quiet machine
    Given the system CPU load reads over 35%
    When a round is due
    Then the bench waits until the load reads under 35%
    And it logs the load it saw

  Scenario: A worse number does not fail the bench
    Given build B uses more idle memory than build A
    When the bench compares them
    Then it exits 0
    And the table marks idle memory as worse

  Scenario: A bench that cannot measure fails
    Given the app shows no window within the timeout
    When the bench runs
    Then it exits non-zero and says what failed

  Scenario: Smoothness is measured idle and during a run
    When the bench measures the Smooth group on the 2,000-test project
    Then each of the four actions is measured idle and during Run all
    And the table shows both figures beside the aims

  Scenario: Idle CPU is measured shown and minimised
    Given Grace is open on a project, idle
    When the bench measures 60 s with the window shown and 60 s minimised
    Then it reports the CPU time of the app's process tree for each

  Scenario: Numbers on a PR
    Given a PR into develop that changes src/
    When the perf workflow runs
    Then it posts one comment with the table comparing the PR to its base
    And a later push updates that comment instead of adding another

  Scenario: A merge waits for the numbers
    Given a PR into develop with the merge-when-green label
    And checks and test passed on its head
    And perf is still running on its head
    Then the PR does not merge yet
    When perf finishes with success
    Then the PR merges

  Scenario: A worse number never blocks the merge
    Given the perf comment shows a measure worse than base
    And checks, test and perf passed on the head
    And the PR has the merge-when-green label
    Then the PR merges

  Scenario: A perf run that could not measure holds the merge
    Given checks and test passed on the head
    And perf failed because the bench could not measure
    Then the PR does not merge

  Scenario: A docs-only PR
    Given a PR that changes only docs/** and *.md
    Then the perf workflow does not run

  Scenario: A PR from a fork
    Given a PR from a fork
    When the perf workflow runs
    Then the table is in the job summary
    And no comment is posted

  Scenario: Raising a limit without a decision record
    Given a PR raises a limit in the limits file
    And it adds no file under docs/decisions/
    Then check-pr fails and names the raised limit

  Scenario: Lowering a limit
    Given a PR lowers a limit in the limits file
    Then check-pr passes

  Scenario: A release with no performance section
    Given a release/v1.0.0-rc.7 PR into main
    And its check record has no "## Performance" section
    Then check-pr fails and names the missing section

  Scenario: A release with no bench JSON
    Given a release/v1.0.0-rc.7 PR into main
    And its check record has a "## Performance" section but no bench JSON beside it
    Then check-pr fails

  Scenario: A release result typed over a miss
    Given the bench JSON shows a measure over its limit
    And the check record says "Result: pass"
    Then check-pr fails and names the measure

  Scenario: A measure the previous release cannot give
    Given the bench cannot find a test id in the previous release
    When it compares the release build with it
    Then that measure shows "n/a" against the previous release
    And it is judged against its limit alone

  Scenario: A release miss with no decision
    Given the release check record says "Result: miss"
    And it has no "Decision: go" line
    Then check-pr fails

  Scenario: A release miss with a no-go
    Given the release check record says "Result: miss"
    And it says "Decision: no-go"
    Then check-pr fails

  Scenario: A release miss, analysed and accepted
    Given the release check record says "Result: miss"
    And it has an "### Analysis" section and "Decision: go"
    Then check-pr passes

  Scenario: A release that passes
    Given the release check record says "Result: pass"
    Then check-pr passes

  Scenario: Feature PRs skip the release gate
    Given a feature/* PR into develop
    Then check-pr does not look for a Performance section
```
