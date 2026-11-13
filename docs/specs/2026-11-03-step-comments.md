---
status: built
issue: 243
summary: "Show the # comments written above each step in the details pane, muted, before and after a run and on a paused debug step"
---

# Step comments in Details

Date 2026-11-03. Covers [#243](https://github.com/hpagniez/grace/issues/243). Builds on [the step debugger](2026-11-02-step-debugger.md) (steps read from the feature file, R4.1).

## Intent

Teams write `#` comments above their steps to say what a step checks and why: where an assertion comes from, why a value is fixed, what a migrated case did before. A suite ported from another tool puts one above almost every check. Details shows only the step lines, so the person opens the file in VS Code to read why a step failed. Details shows a step's comments above it, as the file has them.

## Goals, non-goals, anti-goals

- Goals: a step's comments above it in Details, muted, as written, for the test's own steps and its Background's; before a run, after it, and on the step a debug run is paused at.
- Non-goals: comments Karate attaches to no step; a step inside a called feature (Details shows no such step today, only "Paused in called.feature:5"); folding or cutting a long comment; editing comments; comments in the tree or the Examples table.
- Anti-goals: any cost while idle; an extra element on a step that has no comment; Grace attaching comments differently from Karate.

## Constraints

- Follow the Karate CLI first ([ADR 0026](../decisions/0026-follow-the-karate-cli-first.md)). Karate 2.1.4 puts each step's comments in `step.comments` of its step result in `FEATURE_EXIT`, trimmed, with the `#` (`GherkinParser.transformStep`). Its lexer gives a token every comment line since the previous token that is not a comment or white space, so blank lines between the comments and the step are allowed. Its own HTML report (`karate-report.js`) shows them above the step as written, muted and in italics, one line each, and never folds them.
- Karate's error text for a failed step (`ScenarioResult.getFailureMessageWithLocation`, the `SCENARIO_EXIT` `error` Details shows) already has the step's last comment, the one closest to it, between the place and the step: `basic.feature:20`, `# expected greeting is wrong on purpose`, `And match …`, then the reason.
- Run text goes through the run's masker before the window sees it ([environment switcher R5](2026-10-25-environment-switcher.md)). `maskEvent` masks every string of an event at any depth, so the `steps` event's comments are masked with no new code.

### What exists today

`extractSteps` (`src/main/karate/steps.ts`) turns a depth-0 `FEATURE_EXIT` into `Step`s and drops `step.comments`. Before a test's feature finishes, Details lists the test's steps as `fileSteps` reads them from the feature file (step debugger R4.1); a `#` line there is skipped. The step a debug run is paused at (call depth 0) is one of those rows, marked. `StepRow` in `src/renderer/Details.tsx` draws one row per step.

## Approaches considered

1. **Karate's comments for a run's steps, the same rule in the file reader. Chosen.** After the feature finishes, the comments are Karate's own. Before it, `fileSteps` collects `#` lines with Karate's rule, so the list looks the same before and after. A few lines in each.
2. **Always read the comments from the file.** One code path, but it ignores what Karate sends, and a file edited during the run would show comments the run never had.
3. **Only the run's steps.** The smallest change, but the steps listed before a run, and the paused step of a debug run, would show none.

## Requirements

### R1. Which comments a step has

1. A step's comments are the `#` lines Karate attaches to it: every line starting with `#` after the previous line that is neither blank nor a comment, when the next such line is the step. Each is trimmed, `#` kept.
2. Once its feature finishes, a test's steps take their comments from Karate's `step.comments`. Before that, `fileSteps` collects them from the feature file by R1.1.
3. Background steps keep theirs.
4. A comment Karate attaches to anything else is not shown: above a tag line, `Feature:`, `Background:`, `Scenario:` or `Examples:`; between a step and its doc string or table; between table rows; at the end of the file. A `#` line inside a doc string is the doc string's text, not a comment.
5. A step with no comment has no `comments` field.

### R2. How they show

1. Details shows a step's comments above its row, one line each, as written, in the muted colour and the mono font of the step text, aligned with the step's status mark. A long line wraps; nothing folds or is cut.
2. A step with no comments shows as today, with no extra element.
3. The step a debug run is paused at shows its comments like any other row, inside the paused tint.
4. The failed test's error box stays as Karate writes it, with the failed step's closest comment above the step; the step list below shows all its comments.

### R3. Masking

1. A run's step comments go through the run's masker, like the step's text and log.
2. Comments read from the feature file are not masked, like the step text read from it today.

## Cost

- Idle: nothing. No timer, listener or poll.
- A run: the `steps` event carries a `comments` array only on steps that have comments; a few short strings each. `fileSteps` collects them in its existing pass over the file.
- Drawing: one element per commented step, plus one line per comment. Every step row also gets one empty text node, Solid's placeholder for the comments when there are none, as each `Show` in the row has: no element, no layout. The PR's bench measured `page.nodes.open` +11 and `page.nodes.run` +7, within their limits.
- Memory, start time: no measurable change. Size: under 1 KB of code; no dependency.

## Design decisions already made

- The failed test's error stays as Karate writes it (approach to R2.4): the owner's choice, my recommendation. Karate already puts the closest comment next to the failure.
- The `#` is kept: the owner's choice, my recommendation. The issue asks for the comments as written, and Karate's report keeps it.
- Comments attached to no step are not shown: the owner's choice, my recommendation. Karate drops them from the step results and its report.
- A long comment wraps, with no fold: the owner's choice, my recommendation. Karate's report does the same, and the comment is the reason the person came to read.
- My call: a step inside a called feature gets no comments. Details does not show that step at all today; showing it is a change of its own.
- My call: the mono font and muted colour of the step text, without italics, so a comment reads as part of the file.

## Acceptance criteria

Scenarios run in Vitest (Node for main, jsdom for the window) unless marked **(e2e)**, which run in Playwright against the built app.

```gherkin
Feature: Step comments in Details

  # R1
  Scenario: A run's step keeps Karate's comments
    Given the recorded FEATURE_EXIT of tests/basic.feature
    When its steps are extracted
    Then the step at line 20 has the comments "# expected greeting is wrong on purpose"
    And the step at line 18 has no comments field

  Scenario: The file reader attaches comments as Karate does
    Given a feature file whose step has two comment lines and a blank line above it
    When its steps are read from the file
    Then the step has both comment lines, trimmed, with the "#"

  Scenario: Background steps keep their comments
    Given a Background step with a comment above it
    When the steps are read from the file, or extracted from a run
    Then the Background step has its comment

  Scenario Outline: A comment attached to no step is not shown
    Given a comment <where>
    When the steps are read from the file
    Then no step has that comment

    Examples:
      | where                                   |
      | above a tag line                        |
      | above Scenario:                         |
      | after a scenario's last step            |
      | between a step and its doc string       |
      | between table rows                      |
      | inside a doc string                     |

  # R2
  Scenario: Comments show above their step
    Given a test whose step has the comments "# why" and "# where from"
    When I select the test
    Then the step's row shows "# why" and "# where from" above the step, muted
    And a step with no comments has no comment element

  Scenario: The paused step shows its comments
    Given a debug run paused at a step with a comment
    When I look at the test's details
    Then the paused row shows the comment

  # R3
  Scenario: Comments are masked
    Given a run whose secret value appears in a step comment
    When the steps reach the window
    Then the comment shows "****" in place of the value

  Scenario: Comments before and after a run (e2e)
    Given the clean project is open
    When I select "failing match"
    Then the step at line 20 shows "# expected greeting is wrong on purpose" above it
    When I run the feature
    Then the failed step shows the same comment above it
    And the error box shows it between "basic.feature:20" and the step
```
