# App check: flaky tests (F15)

Date: 2026-10-27
Branch: `feature/flaky-tests` at 1e62f7a ([PR #27](https://github.com/Tusquito/karate-kid/pull/27))
Machine: Windows 11 Pro 10.0.26300 (x64), Node 24.15.0, bundled Temurin 21.0.12.1 and Karate 2.1.4.
Spec: [Flaky tests](../specs/2026-10-26-flaky-tests.md).

## Automated part (run by an agent)

| Check | Result |
|---|---|
| `npm test` | 659/659 passed, including `src/main/fingerprint.test.ts`, `src/main/flaky.test.ts`, the new fingerprint tests in `src/main/runner.test.ts` and `src/renderer/FlakyProof.test.tsx` |
| `npm run e2e` | 101 passed, including the new `e2e/flaky.spec.ts` (7). Its first test runs Karate for real at 2, then 1 thread |
| `npm run typecheck`, `npm run lint` | passed |
| CI on PR #27 | `checks` and `test` passed, with e2e after the PR was marked ready |

Not covered by the automated part:
- How the mark and the proof box look, in light and dark mode.
- Narrator on the mark.
- The walk time at run end on a large real project.

## By hand (done by a person)

Build with `npm run package`, then start `out\karate-kid-win32-x64\karate-kid.exe`. Write the result under each step: passed, or what you saw instead.

### A. How it looks

1. A test that passed and failed with the same project files shows the **flaky** mark in the tree. The folders, features and outlines above it show "n flaky". Both read well in light and dark mode.
   - Result: passed (checked by Tusquito, 2026-10-27).
2. Select the flaky test. The Details pane shows one proof box per environment, with each run's time, result, thread count and Report button. It reads well in light and dark mode.
   - Result: passed (checked by Tusquito, 2026-10-27).

### B. Screen reader

1. Narrator reads the mark as "Flaky: passed and failed with the same project files".
   - Result: passed (checked by Tusquito, 2026-10-27).

### C. A large project

1. Run a large real project. When Java exits, the run ends with no noticeable pause from the second walk of the project files.
   - Result: passed (checked by Tusquito, 2026-10-27).
