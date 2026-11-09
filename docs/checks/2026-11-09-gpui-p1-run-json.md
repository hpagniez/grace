# GPUI Grace check: phase P1's headless run against Electron Grace

Date: 2026-11-09
Commit: `feature/gpui-p1-runner` at 66a6dbb (PRs #292 and #294 stacked under it), and Electron Grace at `develop` 3f8b1d3
Machine: Linux x64 container, 4 vCPU, OpenJDK 21.0.12 on PATH, Karate 2.1.4 from `vendor/karate.jar` (`node scripts/fetch-vendor.mjs --jar`)
Run by: an agent

The phase P1 exit check of #280 and the [P1 plan](../plans/2026-11-09-gpui-p1-core.md): a headless run of the bench's 2,000-test project from `grace-core` alone writes the same `run.json` as Electron Grace.

## How

1. The project: `writeBigProject` of `e2e/big-project.ts`, as the bench makes it (200 features of 10 tests, 1 test in 50 failing).
2. GPUI Grace: `cargo run -p grace-core --example grace-run -- <project> --data <dir>`, which opens the folder with `grace_core::runner::Runner` and runs everything (`d:`) with the data folder's settings. No window.
3. Electron Grace: its own `Runner` (`src/main/runner.ts`) through a one-off Vitest test built on `test/runner.ts`'s `setup()`, in a separate worktree of `develop`, opening the same folder and running `d:`. Electron Grace runs only on Windows, so two lines were changed for Linux in that worktree and nowhere else: the classpath joined with `path.delimiter` instead of `;`, and `java` from PATH instead of `vendor\jre\bin\java.exe`. Neither touches what `run.json` holds.
4. Both `run.json` files compared as JSON, key order included, leaving out only what the plan allows to differ: `startedAt`, `durationMillis`, each test's `durationMillis`, and `fingerprint` (it hashes the files' times).

## Result

| Check | Result |
|---|---|
| Top-level keys, in order | Same: `version`, `startedAt`, `durationMillis`, `env`, `threads`, `fingerprint`, `filter`, `selection`, `failedOf`, `outcome`, `exitCode`, `tests` |
| Every value but the allowed four | Equal |
| Tests | 2,000 in both, in the same order, each with the same keys in the same order, the same `kind`, `file`, `refId`, `name`, `line`, `key` and `status`; every test has a `durationMillis` in both |
| Outcome | `finished`, exit code 1, 1,960 passed and 40 failed, in both |
| Project folder | `projects/d8da96976160` in both (the first 12 hex digits of the SHA-1 of the root) |
| GPUI Grace's time | 7.3 s and 7.5 s from open to the stored run (a debug build), Karate's own summary 3.7 s |

**Pass.** GPUI Grace's runner writes Electron Grace's `run.json` for the 2,000-test project.

## What still needs a person

The same comparison on Windows with the bundled JRE, where Electron Grace runs unchanged: on the owner's PC, with his go-ahead. Nothing opens a window: both runners are headless.
