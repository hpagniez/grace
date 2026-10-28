# Loading always visible Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A sliding stripe in the progress bar while Grace finds tests and until a run's first result, a smooth fill after that, and a bar screen readers can read.

**Architecture:** Renderer only. `Progress.tsx` derives `waiting = (busy && !run.active) || (run.active && run.done === 0)` from the store and draws a stripe behind the green and red segments; a `slide` keyframe joins `blink` in `src/index.css`. The tree's empty text changes in `Tree.tsx`.

**Tech Stack:** SolidJS 2 RC, Tailwind CSS v4, Vitest + `@solidjs/testing-library`, Playwright.

**Spec:** [docs/specs/2026-10-28-loading-always-visible.md](../specs/2026-10-28-loading-always-visible.md)

## Global Constraints

- No change to `src/main/**`, `src/preload.ts`, `src/shared/ipc.ts`, `src/shared/events.ts`.
- No new dependency. Solid 2 rules (AGENTS.md): no `classList`.
- Texts: "Finding tests…" (status and tree), accessible name "Run progress".
- Never run `npm test` and `npm run e2e` at the same time.

## Review Focus

- A run whose first result is a failure: the stripe stops as it does for a pass (`run.done` counts both). Test in Task 1.
- `scenarios` updates with status `running` before any test finishes: the stripe stays. Test in Task 1.
- The bar's `aria-valuenow` follows each result, not only the end. Test in Task 1.
- `runFinished` with no result (a cancel before the first test): no stripe, because the reducer clears `busy` and `run.active` together. Test in Task 1.
- The stripe on the dark track (`zinc-700`) stays visible: `sky-500` on it. Checked by eye in the e2e run; no test.

---

### Task 1: Stripe, smooth fill, progressbar role and "Finding tests…"

**Files:**
- Modify: `src/index.css` (the `@theme` block)
- Modify: `src/renderer/Progress.tsx`
- Modify: `src/renderer/Tree.tsx` (the `message` text)
- Test: `src/renderer/Tree.test.tsx`, `src/renderer/Home.test.tsx` (lines 172 and 179), `e2e/home.spec.ts` (line 76), `e2e/run.spec.ts` ("Run all with live progress")

**Interfaces:**
- Produces: `data-testid="progress-bar"` on the `role="progressbar"` track, `data-testid="progress-stripe"` on the stripe (present only while waiting), utility `animate-slide`.

- [ ] **Step 1: Write the failing unit tests in `src/renderer/Tree.test.tsx`**

Rename "shows discovery as busy: actions off, Cancel on, Preparing…" to "…Finding tests…" and change its expectations:

```tsx
send({ type: "busy", busy: true });
expect(screen.getByTestId("run-outcome").textContent).toBe("Finding tests…");
expect(screen.getByTestId("progress-stripe")).toBeTruthy();
expect(screen.getByTestId("progress-bar").getAttribute("aria-busy")).toBe("true");
expect(screen.getByTestId("progress-bar").hasAttribute("aria-valuenow")).toBe(false);
send({ type: "busy", busy: false });
expect(screen.queryByTestId("progress-stripe")).toBeNull();
expect(screen.getByTestId("progress-bar").getAttribute("aria-busy")).toBe("false");
```

Change "Discovering tests…" to "Finding tests…" in "disables Run all without a tree or without leaves, and says why".

Add, using `setup()`, `load()` and the `runStarted` event of the "Running…" test (`total: 4`):

- `it("keeps the stripe until the first result, then reports progress")`: after `runStarted` the stripe shows; after `send({ type: "scenarios", updates: [{ key: "s:tests/basic.feature#1", status: "running" }] })` it still shows; after the same key with `status: "failed"` the stripe is gone, the bar's `aria-valuenow` is `"1"`, `aria-valuemax` `"4"`, `aria-valuemin` `"0"`, `aria-label` `"Run progress"`; after `"s:tests/basic.feature#2"` `passed`, `aria-valuenow` is `"2"`.
- `it("drops the stripe when a run is cancelled before any result")`: `runStarted`, then `send({ type: "runFinished", outcome: "cancelled", summary: null, reportAvailable: false })` (match the event type in `src/shared/events.ts`); no stripe.
- `it("fills smoothly and keeps the stripe still under reduced motion")`: during `busy`, the stripe's class contains `animate-slide` and `motion-reduce:animate-none`; each segment under the bar has a class containing `transition-[width]`.

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/renderer/Tree.test.tsx`
Expected: FAIL on `Finding tests…` and the missing `progress-stripe` / `progress-bar`.

- [ ] **Step 3: Add the keyframe in `src/index.css`**

In the `@theme` block, next to `--animate-blink`: `--animate-slide: slide 1.4s ease-in-out infinite;` and `@keyframes slide` from `translateX(-100%)` to `translateX(340%)` (a 30 %-wide stripe crosses the track). Comment it as the progress bar's stripe (#80).

- [ ] **Step 4: Change `src/renderer/Progress.tsx`**

- `const waiting = () => (props.state.busy && !run().active) || (run().active && run().done === 0);`
- The status reads `"Finding tests…"` where it read `"Preparing…"`.
- The track becomes `relative` with `role="progressbar"`, `aria-label="Run progress"`, `aria-busy={waiting()}`, and, only when not waiting, `aria-valuemin={0}`, `aria-valuemax={run().total}`, `aria-valuenow={run().done}` (pass `undefined` while waiting so Solid omits them).
- First child: `<Show when={waiting()}>` a `data-testid="progress-stripe"` `aria-hidden` div, `absolute inset-y-0 left-0 w-[30%] rounded bg-sky-500 animate-slide motion-reduce:animate-none`.
- The green and red segments get `relative transition-[width] duration-300 ease-out` (`relative` paints them over the stripe).

- [ ] **Step 5: Change the tree text in `src/renderer/Tree.tsx`**

`"Discovering tests…"` becomes `"Finding tests…"`. Update `src/renderer/Home.test.tsx` (two expectations) and `e2e/home.spec.ts:76` to the new text.

- [ ] **Step 6: Run the unit tests**

Run: `npx vitest run src/renderer`
Expected: PASS. Then `grep -rn "Preparing…\|Discovering tests" src e2e` finds only the comment in `src/renderer/reduce.ts:213`; change that comment's word to "Finding tests…".

- [ ] **Step 7: Extend the e2e "Run all with live progress" in `e2e/run.spec.ts`**

After the Run all click, before `progress-text` "0 / 9": `await expect(win.getByTestId("progress-stripe")).toBeVisible({ timeout: 15_000 });`. After "Finished": `await expect(win.getByTestId("progress-stripe")).toHaveCount(0);` and `await expect(win.getByTestId("progress-bar")).toHaveAttribute("aria-valuenow", "9");`.

- [ ] **Step 8: Full checks**

Run, one after the other: `npm run typecheck`, `npm run lint`, `npm test`, then `npm run e2e`.
Expected: all green; keep the e2e summary for the PR.

- [ ] **Step 9: Commit**

```bash
git add src/index.css src/renderer/Progress.tsx src/renderer/Tree.tsx src/renderer/reduce.ts src/renderer/Tree.test.tsx src/renderer/Home.test.tsx e2e/home.spec.ts e2e/run.spec.ts
git commit -m "feat(ui): show a moving stripe while grace waits and fill the bar smoothly"
```
