---
status: proposed
date: 2026-10-28
decision-makers: Tusquito
---

# Run unit test files in parallel

## Context and Problem Statement

[ADR 0016](0016-windows-tests-in-parallel-jobs.md) kept `fileParallelism: false` within each `unit` job, for two reasons. On a private repo `windows-latest` had 2 CPUs, where two Java tests at once were only 15 % faster. And five unit leak checks (four `ourJava()` calls in `src/main/runner.runs.test.ts`, one in `src/main/karate/process.test.ts`) failed on any `java.exe` from `vendor/jre`, so a Java test file could not run beside another.

Since [ADR 0018](0018-make-the-repository-public.md) the repo is public, and `windows-latest` has 4 CPUs. The 11 test files that start the bundled Java (`src/main/runner.*.test.ts`, `src/main/karate/process.test.ts`, `src/main/karate/argfile.test.ts`) take about 95 % of `npm test`. Run one after the other, `npm test` took 270 s on a 16-CPU machine.

How does `npm test` get faster without dropping a test or a leak check?

## Considered Options

1. Keep every file serial (ADR 0016).
2. Put the 11 Java files in their own serial Vitest project, and run the other files in parallel.
3. Scope the five leak checks to the test's own project, and run every file in parallel.

## Decision Outcome

Chosen: option 3.

- Each of the five leak checks looks only for `java.exe` processes whose command line holds the test's project root. Karate's command line always carries `-w <root>`, and each test copies its fixture to a fresh temp folder. `javaCommandLines(root)` in `test/runner.ts` already did this for other tests; `ourJava()` is removed.
- `vitest.config.mts` drops `fileParallelism: false`. Vitest's default worker count applies.
- The e2e suite does not change: Playwright `workers: 1`, and `ourJavaPids()` in `e2e/helpers.ts` still sees every `java.exe` from `vendor/jre`. So the AGENTS.md rule against running `npm test` and `npm run e2e` at the same time on one machine stays.
- The two `unit` halves in `test.yml` stay, with new file lists. On CI, `src/main/runner.history.test.ts` takes about 135 s, more than any half of the other files: half 2 runs it alone, and half 1 runs every other file (about 315 s in all) on its 3 workers.

This supersedes ADR 0016's "Within each job nothing changes: `fileParallelism: false` ... stay" for the unit tests. The rest of ADR 0016 stands.

Why not the others:

- Option 1 keeps 270 s locally.
- Option 2 measured 254 s: the files it parallelizes take about 15 s in total.

### Consequences

- Good: `npm test` takes 105 s instead of 270 s on a 16-CPU machine, and 110 s with 3 workers, the default on 4 CPUs. On CI, with the old file lists, the two halves took 146 s and 99 s instead of about 185 s and 200 s.
- Bad: no half can end before `src/main/runner.history.test.ts`. Splitting it is the next speed-up.
- Bad: a scoped check misses a leaked JVM whose command line lacks the project root. The e2e checks stay global and still catch it.
- Bad: Java test files now share CPUs. Timing-sensitive tests (the 60 s and 120 s timeouts, `until` waits) may flake on CI's 4 CPUs.
