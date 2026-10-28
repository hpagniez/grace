---
status: accepted
date: 2026-10-28
decision-makers: Tusquito
---

# Run the Windows tests in four parallel jobs

## Context and Problem Statement

[ADR 0007](0007-github-actions-ci-cd-on-the-free-private-plan.md) says "One Windows job runs both suites in turn, so install and downloads happen once and the suites never overlap". On a ready PR that job takes about 20 minutes: setup 46 s, `npm test` about 7.8 minutes (422 s of it in `src/main/runner.test.ts`), then the e2e step about 11 minutes. A draft PR takes about 8.5 minutes. Agents wait for it on every PR.

The repo is private on GitHub Free, so `windows-latest` has 2 CPUs. The Java tests are CPU-bound: two of them at once on 2 CPUs are only 15 % faster than one after the other. Every global `java.exe` leak check (`ourJava()` in the runner tests, `process.test.ts`, `ourJavaPids()` in `e2e/helpers.ts`) sees every Java process on its machine, which is why the suites must not overlap on one machine.

Measurements, causes and estimates: [research note](../research/2026-10-27-ci-test-speed.md). Spec: [docs/specs/2026-10-28-faster-ci.md](../specs/2026-10-28-faster-ci.md).

How does CI get faster without dropping a test or a leak check, within about 1,000 Windows minutes a month?

## Considered Options

1. Keep one job.
2. One job, more workers (Vitest workers, Playwright `workers: 2`).
3. Two jobs in `test.yml`: `unit` and `e2e`.
4. Four jobs in `test.yml`: `unit` as a matrix of two halves with explicit file lists, `e2e` as a matrix of two Playwright shards.
5. Public-repo runners (4 CPUs, no minute cap): make the repo public.

## Decision Outcome

Chosen: option 4. Each job runs on its own VM, so a global leak check sees only its own job's Java, and no test or assertion changes.

- `test.yml` has two jobs, `unit` and `e2e`, each a matrix of two. Each job does the full setup (checkout, Node, the Electron and `vendor/` caches, `npm ci`, `npm run vendor`).
- `unit`: half 2 runs a fixed list of runner test files, half 1 runs every other test file. The list lives in `test.yml`. Not `vitest --shard`: it splits by file count after a hash of the path, not by time, so the slow runner files could land in one half.
- `src/main/runner.test.ts` is split into one file per `describe` block, with the shared helpers in `test/runner.ts`, so the list can balance the halves.
- `e2e`: `playwright test --shard=1/2` and `--shard=2/2`. Each half packages the app. The draft check stays in the `e2e` job, before the step named `e2e`.
- `merge-when-green.yml` requires every step named `e2e` in the run to have passed, not only the first.
- Within each job nothing changes: `fileParallelism: false` and Playwright `workers: 1` stay, and so does the AGENTS.md rule against running `npm test` and `npm run e2e` at the same time on one machine.

This supersedes ADR 0007's "One Windows job runs both suites in turn". The rest of ADR 0007 stands.

Why not the others:

- Option 1 keeps 20 minutes per ready PR.
- Option 2 gains about 15 % on 2 CPUs, needs leak checks scoped per test first, and puts two Electron windows and their JVMs on 2 CPUs.
- Option 3 alone gets a ready PR to about 12 minutes; the halves get it to about 7 for one or two more billed minutes.
- Option 5 is not decided: the repo stays private for now (ADR 0007).

### Consequences

- Good: a ready PR takes about 7 minutes instead of 20, a draft PR about 5 instead of 8.5 (estimates; the check record of the F44 PR has the first real times).
- Good: a failure in one suite still shows the other suite's result.
- Bad: about 3 more billed Windows minutes per ready run (about 21 to 24) and about 3 more per draft run (about 9 to 12): setup runs four times, and billing rounds each job up to a whole minute.
- Bad: a unit failure no longer stops the e2e jobs, so a red PR spends their minutes too.
- Bad: the halves stay even only by hand. A new slow test file, or Playwright's count-based shards, can make one half the long pole. The spec says how to rebalance.
- Neutral: separate jobs make leak checks scoped per test (research note 3.4) no longer needed for CI; they stay a possible local speed-up.
