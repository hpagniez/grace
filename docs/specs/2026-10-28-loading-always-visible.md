# Loading always visible

Date 2026-10-28. Status: approved. Covers [#80](https://github.com/hpagniez/grace/issues/80).

## Intent

While Grace waits on Karate, the window looks idle. Finding tests (opening a folder, Refresh, an environment switch, the discovery before each run) takes a few seconds of JVM start, and the only sign is the grey word "Preparing…" in the progress row and the `cursor-progress` cursor. After Refresh the old tree stays as it was. Once a run starts the bar stays empty until the first test finishes, then jumps at each result. The person wants something on screen that moves while Grace works, and a bar that fills smoothly.

## Goals, non-goals, anti-goals

**Goals:** the requirements below.

**Non-goals:**
- A loader for saving. Settings, the environment, the filter, the thread count and the run properties are written synchronously in main; a loader would only flash. The Properties dialog keeps its "Saving…" label.
- The startup screen (`index.html`), which already has a spinner.
- The toolbar, the tree's layout and the console banner.
- Motion during a long test after the first result: the elapsed timer covers it.

**Anti-goals:**
- A stripe that keeps moving after Grace stopped waiting (cancelled, tests found, first result in).
- Motion for someone who asked the system for reduced motion.
- A bar a screen reader cannot read.

## Constraints

- Renderer only: no change to `src/main/**`, the preload or the IPC files. Everything derives from `state.busy`, `state.run.active` and `state.run.done`.
- Solid 2 rules and Tailwind v4 (AGENTS.md). No new dependency.

### What the app does today

- `src/renderer/Progress.tsx` draws the bar: a `bg-zinc-200` track holding a green and a red segment whose `width` is set by `pct(run().passed)` and `pct(run().failed)`, with no transition. The status (`data-testid="run-outcome"`) reads "Running…" while a run is active, "Preparing…" while busy with no run, else the outcome.
- `state.busy` is true while the Runner discovers or runs. A run starts with a discovery, so `busy` turns on before `run.active`. `runFinished` clears `busy` too (`src/renderer/reduce.ts`).
- `run.done` counts tests that passed or failed in this run; `run.total` is the dry-run total.
- `src/renderer/Tree.tsx` shows "Discovering tests…" while there is no project or no tree. The console banner says "Finding tests…" for the same moment.
- `src/index.css` defines `--animate-blink` in `@theme` for the banner cursor.

## Approaches considered

For the look while finding tests (mockups, live):
- A. A stripe sliding across the progress bar, the status "Finding tests…". **Chosen by the person.**
- B. A, plus the tree faded under a spinner.
- C. A spinner and "Finding tests…" next to the project path in the toolbar.

For the run (mockups, live):
- A. The stripe until the first result, then a smooth fill. **Chosen by the person.**
- B. A smooth fill with the stripe sliding over the unfilled part until the run ends.

For how to build it:
1. **CSS in `Progress.tsx`:** a stripe element in the track, a `slide` keyframe in `src/index.css`, a width transition on the segments. **Chosen by the person.**
2. The native `<progress>` element. Dropped: one colour, so no green and red segments.
3. Counts animated in JavaScript with `requestAnimationFrame`. Dropped: more code for what a CSS transition does.

## Requirements

### R1. The stripe

1. The progress bar shows a sky-blue stripe sliding left to right across the track, looping, while Grace waits:
   - busy with no run active (finding tests), or
   - a run active with no test finished yet (`run.done` is 0).
2. Otherwise there is no stripe: at the first result, on Cancel, when discovery ends, and when idle.
3. The stripe is drawn over the green and red segments (changed after the final review, 2026-10-28: drawn behind them, it was hidden during every discovery after a run, as the bar keeps the last run's results).
4. With reduced motion asked by the system (`prefers-reduced-motion: reduce`), the stripe shows but does not move.

### R2. The fill

1. The green and red segments change width with a short ease-out transition (about 300 ms), so the bar grows smoothly at each result instead of jumping.
2. When a new run empties the bar, the segments may shrink with the same transition.

### R3. The words

1. The status reads "Finding tests…" while busy with no run active (was "Preparing…").
2. The tree's message with no project or no tree reads "Finding tests…" (was "Discovering tests…").

### R4. Screen readers

1. The bar is a `role="progressbar"` with an accessible name ("Run progress").
2. While the stripe shows, the bar has `aria-busy="true"` and no `aria-valuenow`.
3. Otherwise it has `aria-valuemin="0"`, `aria-valuemax` the run total and `aria-valuenow` the tests done.

## Design decisions already made

- The stripe in the progress bar while finding tests, the tree unchanged (the person, option A).
- The stripe until the first result, then a smooth fill (the person, option A).
- CSS in `Progress.tsx` (the person, approach 1).

The spec author's calls, accepted by the person:
- No loader for saving.
- A still stripe under reduced motion, as the startup spinner stops.
- The progress bar role and its values (R4).
- "Finding tests…" everywhere for one moment (R3).

## Left to the builder

- The stripe's width, colour shade and timing (the mockup used 30 % of the track, `sky-500`, 1.4 s ease-in-out).
- A `data-testid` for the stripe.

## Acceptance criteria

Scenarios run in Vitest (jsdom) unless marked **(e2e)**, which run in Playwright against the built app.

```gherkin
Feature: Loading always visible

  # R1, R3: finding tests

  Scenario: Finding tests shows the stripe
    Given a project is open
    And the Runner is busy with no run active
    Then the progress bar shows the stripe
    And the status reads "Finding tests…"
    And the progress bar is marked busy

  Scenario: Finding tests for the first time
    Given a project is being opened and no tree has been found yet
    Then the tree reads "Finding tests…"

  Scenario: Tests found
    Given the stripe shows while finding tests
    When the Runner is no longer busy
    Then the progress bar shows no stripe

  # R1, R2: runs

  Scenario: The stripe stays until the first result
    Given a run of 4 tests is active
    And no test has finished
    Then the progress bar shows the stripe
    When the first test passes
    Then the progress bar shows no stripe
    And the bar reports 1 of 4 done

  Scenario: The bar fills smoothly
    Given a run is active
    Then the green and red segments have a width transition

  Scenario: Cancel stops the stripe
    Given a run is active with no test finished
    When the run finishes as cancelled
    Then the progress bar shows no stripe

  Scenario: Reduced motion
    Given the system asks for reduced motion
    Then the stripe does not move

  # end to end

  Scenario: Run and finish (e2e)
    Given the sample project is open
    When I press Run all
    Then the progress bar shows the stripe
    When the run finishes
    Then the progress bar shows no stripe
    And the bar reports all tests done
```
