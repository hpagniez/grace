# App check: thread count (F12)

Date: 2026-10-26
Branch: `feature/thread-count` at 3055664 ([PR #26](https://github.com/Tusquito/karate-kid/pull/26))
Machine: Windows 11 Pro 10.0.26300 (x64), Node 24.15.0, bundled Temurin 21.0.12.1 and Karate 2.1.4.
Spec: [Thread count](../specs/2026-10-26-thread-count.md).

## Automated part (run by an agent)

| Check | Result |
|---|---|
| `npm test` | 593/593 passed at 3055664, including `src/shared/threads.test.ts`, the new thread-count tests in `src/main/settings.test.ts` and `src/main/runner.test.ts` ("Runner threads", 8), and `src/renderer/Threads.test.tsx` (6) |
| `npm run e2e` | 94 passed at c8fc052, including the new `e2e/threads.spec.ts` (8). After the last commit, which changed tests only, `e2e/threads.spec.ts` passed again (8/8). One run of `e2e/cursor.spec.ts` "drag cursor wins over the busy cursor" failed (got `progress`, expected `row-resize`), then passed in 3 reruns and in the full run. F12 does not touch that code |
| `npm run typecheck`, `npm run lint` | passed |

Not covered by the automated part:
- How the box looks, and the toolbar at a narrow width.
- The spin buttons, the mouse wheel and screen readers.
- A real suite at 8 and 32 threads, including the HTTP tab, cancel and the console.
- An upgrade over existing settings and history.
- A real project whose `karate-pom.json` sets `threads`.

## By hand (done by a person)

Build with `npm run package`, then start `out\karate-kid-win32-x64\karate-kid.exe`. Write the result under each step: passed, or what you saw instead.

### A. How it looks

1. The Threads box fits "32" next to its spin buttons, without clipping. It lines up with Env and Properties…, in light and dark mode.
   - Result: passed (checked by Tusquito, 2026-10-26).
2. At the smallest window width, the toolbar still shows the box and every button.
   - Result: passed (checked by Tusquito, 2026-10-26).
3. The cursor is a text cursor over the box, and not-allowed while the box is greyed out during a run.
   - Result: passed (checked by Tusquito, 2026-10-26).

### B. Mouse and keyboard

1. Clicking the spin buttons steps the count, and the count survives a restart. Holding an arrow down does not misbehave.
   - Result: passed (checked by Tusquito, 2026-10-26).
2. The mouse wheel over the focused box does nothing unexpected.
   - Result: passed (checked by Tusquito, 2026-10-26).
3. Tab goes Env, Threads, Properties…. Narrator reads the box as "Threads".
   - Result: passed (checked by Tusquito, 2026-10-26).

### C. Real parallel runs

1. A real suite runs clearly faster at 8 threads than at 1. Progress counts, tree statuses and step details are right.
   - Result: passed (checked by Tusquito, 2026-10-26).
2. The http sample project at 8 threads: the HTTP tab lists all 13 calls in the right status groups.
   - Result: passed (checked by Tusquito, 2026-10-26).
3. Cancel a run at 8 threads: every JVM ends (no `java.exe` left), and the run shows "Cancelled".
   - Result: passed (checked by Tusquito, 2026-10-26).
4. At 8 threads the console interleaves lines but stays readable. At 1 thread the lines come in order.
   - Result: passed (checked by Tusquito, 2026-10-26).
5. A big suite at 32 threads: the app survives. A JVM out of memory would show as a failed run, with the error in the console.
   - Result: passed (checked by Tusquito, 2026-10-26).

### D. Upgrade and pom

1. Start the new build on an existing `settings.json` and history from v1.1.0. Projects open at 1, with filters, environment and properties intact. Old History rows show "—".
   - Result: passed (checked by Tusquito, 2026-10-26).
2. A real project whose `karate-pom.json` sets `threads` above 1 opens (it could not before), the box shows 1, and runs use the box's count.
   - Result: passed (checked by Tusquito, 2026-10-26).
