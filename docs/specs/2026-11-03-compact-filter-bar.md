# A one-line filter bar

Date 2026-11-03. Status: approved. Covers [#127](https://github.com/hpagniez/grace/issues/127). Builds on the [tag filter](2026-10-24-tag-filter.md) and the filter bar of the [premium UI](2026-10-29-premium-ui.md) (R7.3).

## Intent

The filter bar wraps its tag chips onto as many lines as they need. In a big project that is most of the tree's height: with the bench's 53 tags it takes 4 to 6 lines, 127 to 187 px ([big-project note](../research/2026-10-29-big-project-perf/README.md), section 6). The bar keeps one line instead, and the tags that do not fit fold into a panel, so the tree keeps its height whatever the number of tags.

## Goals, non-goals, anti-goals

- Goals: the bar is always one line; every tag stays one click away; a tag that is on but folded away still shows that it is on.
- Non-goals: searching or typing in the tag list; a different order of tags; any change to the tag expression sent to Karate, the filter state or how a click cycles a tag.
- Anti-goals: measuring chips in code on every render or while idle; a chip moving under the pointer when it is clicked; the tree moving when the panel opens.

## Constraints

- Idle costs nothing ([ADR 0023](../decisions/0023-run-lightly-as-a-release-requirement.md)): no timer, poll or measuring loop.
- UI conventions (`AGENTS.md`): colours through tokens, `Icon`, `Tooltip`, the class strings of `buttons.ts` and `ui.ts`.
- `Tree.tsx` does not change.

### What exists today

`src/renderer/FilterBar.tsx` draws, in one `flex-wrap` row across the sheet above the tree and the details: one chip per tag (`chipTags`, sorted A to Z), then Clear and the match counter while a filter is on, then Hide non-matching pushed to the right. A click on a chip cycles it off, included, excluded, off; the window sends the new filter to main, which echoes it back. Chips are disabled during a run.

## Approaches considered

1. **Measure in code.** Measure each chip's width, work out how many fit before the fold chip, and draw only those. It must measure again on a resize, a new tag, and a chip that widens when it turns on (its icon). More code, and a count that can go stale.
2. **The browser wraps, Grace counts. Chosen by the owner.** The chips wrap as today inside a strip that shows only its first line. One `IntersectionObserver` on the strip says which chips are on the hidden lines, and only when that changes. Resizes, new tags and wider chips need no code of their own.

Ways to show the folded tags, also settled with the owner: a panel over the tree (chosen), or a bar that grows in place until folded again (pushes the tree down). Tags that are on: keep their A-to-Z place and are counted on the fold chip (chosen), or move to the front of the line (always visible, but a chip jumps when clicked).

## Requirements

### R1. One line

1. The bar is one line tall whatever the number of tags and the window width: the chips, then the fold chip (R2), then Clear and the counter while a filter is on, then Hide non-matching at the right. Clear, the counter and Hide non-matching never wrap or fold.
2. Chips stay in A-to-Z order. The ones that fit on the line show; the rest fold. A chip never shows cut in part, except a single chip wider than the whole line.
3. A folded chip is out of the Tab order and hidden from screen readers.
4. The focus ring of a chip on the line is not cut.
5. With every chip on the line, the bar looks as it does today, without the fold chip.

### R2. The fold chip

1. While at least one chip is folded, a chip follows the last one that fits: `+N`, N the number of folded tags. When K of the folded tags are included or excluded, it reads `+N · K on`.
2. Its accessible name is "N more tags" ("1 more tag"), plus ", K on" when K is not 0. It says whether the panel is open (`aria-expanded`). Its tooltip is "Show all tags".
3. It works during a run, so the panel can be read; the chips in the panel are disabled then, like those on the line.

### R3. The panel

1. A click on the fold chip opens a panel under the bar, over the tree, holding every tag's chip in A-to-Z order, wrapped. The tree does not move. A long list scrolls inside the panel.
2. A chip in the panel looks, reads, and cycles like the same chip on the line. The panel stays open after the click, so several tags can be set in turn.
3. The panel closes on a second click on the fold chip, on Escape, and on a click outside it. Escape puts the focus back on the fold chip.
4. The panel's chips follow the fold chip in the Tab order.
5. The panel closes when no chip is folded any more (the window grew, or the project has fewer tags).
6. The panel's chips exist only while it is open; closed, the panel is one empty element.

## Cost

- Idle: nothing runs. One `IntersectionObserver` per open project, on the strip; the browser calls it only when a chip crosses the fold: a resize, a tag list change, a chip that widens or narrows when it turns on or off, Clear and the counter appearing. No timer, no poll, no layout read in Grace's code.
- During a run: the tags and the chips' states do not change, so the observer is not called.
- Memory and nodes: the fold chip and the empty panel add a few nodes; the panel adds one chip per tag only while open (53 with the bench project). The chips on the line are the ones drawn today, so `page.nodes.open` does not grow beyond the fold chip.
- Start: one observer call after the first layout, off the paint. Size: about 2 KB of code; no dependency.

## Design decisions already made

- The panel over the tree, not a bar that grows: the owner's choice.
- Tags that are on keep their A-to-Z place, counted on the fold chip: the owner's choice.
- The browser wraps the chips and an `IntersectionObserver` counts the folded ones: the owner's choice (approach 2).
- My call: the panel holds every tag, not only the folded ones, so one A-to-Z list has them all.
- My call: the fold chip works during a run (R2.3); reading which tags are on is harmless.
- My call: no search box in the panel. A project with hundreds of tags may want one; that is a feature of its own.

## Acceptance criteria

Scenarios run in Vitest (jsdom, with a stand-in `IntersectionObserver` that the test drives) unless marked **(e2e)**, which run in Playwright against the built app.

```gherkin
Feature: A one-line filter bar

  # R1, R2
  Scenario: Chips that do not fit fold behind a count
    Given a project with the tags "a", "b", "c" and "d"
    When the chips "c" and "d" are past the line
    Then the fold chip reads "+2"
    And its accessible name is "2 more tags"
    And the chips "c" and "d" are hidden

  Scenario: A folded tag that is on is counted
    Given the chips "c" and "d" are past the line
    And the tag "d" is excluded
    Then the fold chip reads "+2 · 1 on"
    And its accessible name is "2 more tags, 1 on"

  Scenario: No fold chip when every chip fits
    Given a project with the tags "a" and "b"
    When every chip is on the line
    Then there is no fold chip

  # R3
  Scenario: The panel holds every tag and cycles it
    Given the chips "c" and "d" are past the line
    When I click the fold chip
    Then the fold chip says the panel is open
    And the panel holds the chips "@a", "@b", "@c" and "@d"
    When I click "@c" in the panel
    Then Grace sends the filter including "c"
    And the panel is still open

  Scenario: The panel closes when nothing is folded
    Given the panel is open
    When every chip comes back on the line
    Then the panel is closed

  Scenario: During a run the panel opens with its chips disabled
    Given a run is going and the chips "c" and "d" are past the line
    When I click the fold chip
    Then the panel's chips are disabled

  Scenario: The bench project keeps a one-line bar (e2e)
    Given the big project with 53 tags is open
    Then the filter bar is one chip line tall
    And the fold chip reads "+N" with N the number of hidden chips
    When I open the panel and include "module-5-5"
    Then the tree shows the tests that match
    And the fold chip reads "+N · 1 on"
    When I press Escape
    Then the panel is closed and the focus is on the fold chip
```
