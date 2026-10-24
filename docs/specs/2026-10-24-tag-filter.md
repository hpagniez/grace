# Tag filter: run by tag

Date 2026-10-24. Status: approved. Covers [features](../features.md) F04.

## Intent

Teams mark scenarios with tags (`@smoke`, `@wip`, `@slow`) to run a subset: the smoke set before a merge, everything but work in progress at night. Today the app runs a whole folder, a feature or one test, with no way to say "only `@smoke`, never `@wip`". This feature adds a tag filter: the person clicks the project's tags to include or exclude them, sees at once which tests match, and Run all, a folder or a feature runs only those.

It builds on the [walking skeleton](2026-10-24-walking-skeleton.md): same tree, same run flow, same one-process-at-a-time runner.

## Who and what success looks like

The walking skeleton's user, with a project whose features carry tags.

Success:
- Clicking a tag chip updates the tree at once (no Java started) and shows how many tests match.
- Run all with a filter runs exactly the matching tests, and the progress total counts only them.
- The filter is still set after closing and reopening the app.

## Goals, non-goals, anti-goals

**Goals:** the behaviour in the Requirements section below.

**Non-goals for this version:**
- AND between included tags, and typed Karate tag expressions (`anyOf`, `allOf`, `not`). New `could` row F23 in [features.md](../features.md).
- Filtering by scenario name (`-n`), environment (`-e`, F08), or `@env=` tags. Scenarios tagged `@env=x` are already left out of the tree by Karate when no environment is set.
- Saved, named filters ("smoke", "nightly"). One filter per project.

**Anti-goals** (failure even if everything else works):
- Running a test the filter excludes. This includes the case Karate itself allows: `file.feature:LINE` bypasses `-t`.
- The app showing a test as matching that Karate then does not run, without the person being told.
- A forgotten filter silently shrinking Run all: the filter must always be visible while it is on.

## Constraints

- Everything in the walking skeleton's Constraints section.
- Karate tag behaviour per the [CLI spike](../research/2026-10-24-karate-cli-spike/README.md#tags) and the probe below.

### What the dry run says about tags (probe, 2026-10-24, Karate 2.1.4)

A dry run of a feature tagged `@feat`, holding a scenario `@s1`, a scenario `@env=dev`, and an outline `@ol` with two `Examples:` blocks tagged `@ex1` and `@ex2`:

| `-t` | Scenarios selected, with their `SCENARIO_ENTER.tags` |
|---|---|
| none | `plain ["feat","s1"]`, `row 1 ["feat","ol","ex1"]`, `row 2 ["feat","ol","ex2"]` |
| `@ex2` | `row 2` |
| `~@ex1` | `plain`, `row 2` |
| `@s1,@ex2` | `plain`, `row 2` |

So `SCENARIO_ENTER.tags` is the scenario's full set: feature + scenario, or feature + outline + its `Examples:` block. The `@env=dev` scenario is not in the dry run at all. The tree's scenario and row nodes already store these tags (walking skeleton R2).

## Approaches considered

How the app knows which tests match:

1. **The app matches from the tree's tags; Karate gets `-t` at run time.** Matching is set logic on tags the tree already holds, so chips, dimming and counts update instantly with no Java. The run passes the same filter as `-t`, so Karate still does the filtering. Risk: app and Karate disagree; the probe shows they agree for OR and NOT on plain tags, and the existing end-of-run message ("Karate ran n of total selected tests") shows any gap. **Chosen.**
2. **A dry run with `-t` on every filter change.** Exact by construction, but each chip click costs about a second of Java, blocks Run meanwhile, and fights the runner's one-process rule when clicks come fast.
3. **Option 1 plus a `-t` dry run before each filtered run to check the count.** Catches a disagreement, at about a second per run, for a case the probe suggests does not happen with OR and NOT. Worth it only with typed expressions (F23).

## Requirements

### R1. Tag chips

- A filter bar sits between the toolbar and the tree. It shows one chip per tag found on the tree's scenarios and rows (their full tag sets, so feature, outline and `Examples:` tags appear), sorted by name, shown with `@`.
- The bar is hidden when the project has no tags.
- Clicking a chip cycles it: **off → include → exclude → off**. Include is shown green, exclude red and struck through, off neutral.
- A **Clear** control sets every chip to off. It shows only when a chip is not off.
- Chips and Clear are disabled while the runner is busy (a run or a discovery dry run), as Run is today.

### R2. Matching

- A scenario or row **matches** when: no chip is included, or at least one of its tags is included; **and** none of its tags is excluded. So included tags combine with OR, and any excluded tag removes the test.
- A folder, feature or outline matches when at least one scenario or row under it matches.
- The **filter is on** when at least one chip is not off. With the filter off, everything matches and the app behaves as in the walking skeleton.
- Matching is computed in the window from the tree's tags, with no Java process. It is redone whenever the tree or the filter changes.

### R3. The tree under a filter

- While the filter is on, a counter in the filter bar shows "`<m>` of `<n>` tests match" (scenarios and rows).
- Nodes that do not match are dimmed and have no ▶ button. They can still be selected, and the details pane still shows their name, tags and last result.
- A **Hide non-matching** toggle in the filter bar removes non-matching nodes from the tree instead of dimming them. It has no effect while the filter is off.
- Run all is disabled when the filter is on and nothing matches; the counter then reads "0 of `<n>` tests match".

### R4. Running under a filter

- What each ▶ runs while the filter is on:
  - suite, folder or feature: its targets as today, plus the filter as `-t` (see "Building `-t`"). Only matching tests run.
  - a matching scenario or row: `file.feature:LINE` as today. It matches, so the bypass of `-t` changes nothing.
  - an outline whose rows all match: `file.feature:LINE` with the outline's line, as today.
  - an outline whose rows match only in part: `file.feature:L1:L2…` with the lines of its matching rows. If any matching row has no line (dynamic Examples), the outline cannot be run under this filter: show "Clear the filter to run this outline".
- The pre-run discovery of walking skeleton R3 is unchanged, then:
  - the saved filter is applied to the new tree as it is. A tag that vanished since the last Open or Refresh is not dropped here: an included tag that no longer exists matches nothing, so the run shrinks ("Nothing to run" or "This test no longer matches the filter") and never widens past what the person saw.
  - the clicked node is found in the new tree as today. If it no longer matches, do not run: show "This test no longer matches the filter".
  - `total` and the nodes reset to "pending" are the matching scenarios and rows under the node. Zero → "Nothing to run", as today.
- Matching is decided by the main process for runs (it builds the command line), using the same matching function as the window.
- At the end of the run, the walking skeleton's "Karate ran `<n>` of `<total>` selected tests" covers any test the app expected but Karate did not run. The other way round, if `SUITE_EXIT`'s scenario count is greater than `total`, show "Karate ran `<n>` tests, more than the `<total>` selected": the app and Karate disagreed on the filter.

### R5. Dropping tags that left the project

- After an Open or a Refresh whose dry run succeeds, saved tags that no longer exist in the project are dropped from the filter and from the saved settings. The chips and counter show the result.
- A failed dry run, and the pre-run discovery of R4, drop nothing.

### R6. Saved filter

- The filter (included and excluded tags) and the Hide non-matching toggle are saved per project folder in the app's `settings.json`, next to the last opened project, and restored when that folder is opened again.
- A change is saved when it is made.

## Design decisions already made

### Building `-t`

From the chips: all included tags in one `-t`, joined with commas (OR); each excluded tag in its own `-t ~@tag` (repeated `-t` = AND).

```
include smoke, api; exclude wip, slow  →  -t @smoke,@api -t ~@wip -t ~@slow
include none;       exclude wip        →  -t ~@wip
```

This is exactly R2's rule in Karate's syntax, using only forms the spike verified. No `-t` when the filter is off. `-t` goes in `buildArgs` before the targets.

### Where the filter lives

- The main process owns the saved filter: it loads it when a project opens, sends it to the window with the project, saves changes, and uses it to build runs.
- The window sends filter changes to main over a new IPC call, and computes matching locally for display.
- One pure matching function in `src/shared/` (no Node imports), used by both sides.

### Settings file

`settings.json` gains a per-project map, keyed by the project's absolute path:

```json
{
  "lastProject": "D:\\work\\api-tests",
  "projects": {
    "D:\\work\\api-tests": { "include": ["smoke"], "exclude": ["wip"], "hideNonMatching": false }
  }
}
```

Tags are stored without `@`, like the tree. Writing one key must keep the others (today `saveLastProject` overwrites the whole file). An unreadable or malformed file, or a malformed entry, reads as no filter.

## Acceptance criteria

Test projects:
- The **clean sample project** of the walking skeleton. Its tags: `passing hello` @smoke, `failing match` @wip, `calls helper feature` none, `smoke but wip` @smoke @wip (all in `basic.feature`); `slow loop` @slow, `smoke and slow` @smoke @slow (`slow.feature`); the 3 outline rows none. 9 tests.
- A new **tags sample project**: one feature tagged `@feat` with an outline `@ol` whose two `Examples:` blocks are tagged `@fast` (2 rows) and `@slow` (1 row), and one plain scenario tagged `@other`. All pass.

Every scenario is automated with Playwright against the built app, each on a fresh copy of its fixture, like the walking skeleton.

```gherkin
Feature: Tag filter

  Background:
    Given karate-kid is built with the bundled Java and Karate on Windows 11

  Scenario: Chips list the project's tags
    When I open the clean sample project
    Then the filter bar shows the chips "@slow", "@smoke" and "@wip"
    And every chip is off and no counter is shown

  Scenario: Include and exclude dim the tree
    Given the clean sample project is open
    When I click "@smoke" once and "@wip" twice
    Then "@smoke" shows as included and "@wip" as excluded
    And the counter shows "2 of 9 tests match"
    And "passing hello" and "smoke and slow" are not dimmed
    And "failing match", "smoke but wip" and "outline.feature" are dimmed and have no run button

  Scenario: Third click turns a chip off
    Given the clean sample project is open with "@smoke" included
    When I click "@smoke" twice
    Then "@smoke" is off and no test is dimmed

  Scenario: Hide non-matching
    Given the clean sample project is open with "@smoke" included and "@wip" excluded
    When I switch on "Hide non-matching"
    Then the tree shows only "passing hello" and "smoke and slow" and their folders and features

  Scenario: Run all runs only matching tests
    Given the clean sample project is open with "@smoke" included and "@wip" excluded
    When I click Run all
    Then the progress section shows "0 / 2" before any scenario finishes
    And the run ends "Finished" with 2 passed and 0 failed
    And "failing match" keeps its previous status

  Scenario: A feature run is filtered
    Given the clean sample project is open with "@wip" excluded
    When I click run on "basic.feature"
    Then only "passing hello" and "calls helper feature" run and the progress shows "2 / 2"

  Scenario: A tag removed before a run does not widen it
    Given the clean sample project is open with only "@wip" included
    When I remove every "@wip" tag in another editor and click Run all
    Then nothing runs and the app says there is nothing to run

  Scenario: Part of an outline matches
    Given the tags sample project is open with "@fast" included
    When I click run on the outline
    Then only its 2 "@fast" rows run and the progress shows "2 / 2"

  Scenario: Nothing matches
    Given the tags sample project is open
    When I exclude "@feat"
    Then the counter shows "0 of 4 tests match"
    And Run all is disabled

  Scenario: The filter survives a restart
    Given the clean sample project is open with "@smoke" included and "Hide non-matching" on
    When I close and restart the app
    Then "@smoke" is still included and "Hide non-matching" is still on

  Scenario: A tag removed from the project leaves the filter
    Given the clean sample project is open with "@wip" excluded
    When I remove every "@wip" tag in another editor and click Refresh
    Then there is no "@wip" chip and the filter is off
```

Also automated (Vitest):
- The matching function: OR across included, any excluded removes, empty filter matches all, containers match through any leaf.
- `-t` building, including include-only, exclude-only and both.
- Run targets under a filter: suite, folder, feature, full outline, partial outline (`file:L1:L2`), partial outline with a dynamic row.
- Run end: a `SUITE_EXIT` scenario count above `total` produces the "more than selected" notice.
- Settings: load and save of the per-project filter, keeping `lastProject` and other projects, and a malformed file or entry read as no filter.
- A Node test with the bundled Java: on the tags sample project, dry runs with `-t @fast`, `-t @fast,@other` and `-t ~@slow` select exactly the tests the matching function picks from the unfiltered tree.

## Left to the builder

- Component structure, styling, exact IPC names, and how the store holds the filter.
- Exact wording of the counter, empty states and notices.
- Whether tags with values (`@owner=bob`) work with `-t @owner=bob`. If they do, they are chips like any other; if not, leave them out of the chips and say so in this spec.
- Debouncing saves if chip clicks write the file too often.
- Checking that `file.feature:L1:L2` with two outline-row lines runs exactly those rows (the spike verified multi-line selection for scenarios only). The "Part of an outline matches" scenario covers it.
