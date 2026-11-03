# Open a test in VS Code

Date 2026-11-03. Status: approved. Covers [#68](https://github.com/hpagniez/grace/issues/68).

## Intent

Someone sees a failed scenario in Grace, then has to find its file and line in their IDE by hand. Grace hands off to the editor instead: one click opens the feature file in Visual Studio Code at the scenario's line, or at the line where Karate says it failed.

## Who and what success looks like

Someone with Grace open all day beside VS Code. Success: from a failed test's details, one click lands them in VS Code on the line to fix, even when the failure is inside a called feature.

## Goals, non-goals, anti-goals

- Goals: open the selected scenario or row at its line; open the place Karate names in a failed test's error, at that line.
- Non-goals: other editors, an editor setting or a custom command; links in the console; a tree context menu (after [#215](https://github.com/hpagniez/grace/pull/215) merges); opening a feature or a folder; a column.
- Anti-goals: Grace opening any file outside the open project; Grace starting a process or a shell to open it; any cost while idle.

## Constraints

- Security rules in `AGENTS.md`: the window reaches main only through the typed preload API, and main checks everything it receives.
- The Karate CLI has nothing to open an editor, so Grace has its own way. What it opens follows Karate: the tree's feature paths resolve as the run resolves them (project root, then its classpath roots), and a failure's place is the `<path>:<line>` Karate writes on its error's first line (for example `C:\fixture\tests\basic.feature:20`, `test/fixtures/events/1-normal-run-failure.jsonl`).
- VS Code registers `vscode://file/<full path>:<line>` with Windows when it is installed. On Windows, VS Code asks the person to confirm before it opens a file from such a link ([microsoft/vscode#95252](https://github.com/microsoft/vscode/issues/95252)); that prompt is VS Code's and Grace cannot skip it.
- VS Code's `code` command is `code.cmd`, which Node starts only through a shell.
- PR #215 changes `Tree.tsx`, `Banner.tsx`, `Progress.tsx`, `runner.ts` and `src/main/karate/*`: this change adds one method to `runner.ts` and touches none of the others.

### What exists today

The Details pane (`src/renderer/Details.tsx`) shows a scenario's or row's `file:line` under its status, and a failed test's error in a red box (`details-error`). Main resolves a tree path to a file in `Runner.steps()` with `locate([root, ...classpath], file)`.

## Approaches considered

1. **A: the window sends a path and a line; main checks them. Chosen.** The button sends the tree's path, the error link the path Karate wrote. Main resolves the path, refuses it unless it is a file inside the open project, then opens the `vscode://` link. One IPC call, a short main module and a small `runner.ts` method.
2. **B: the window sends the test's key only; main finds both lines.** No path crosses from the window, but main would keep every test's error text, history runs included: more change in `runner.ts` and `src/main/karate/*` (#215's files) for no safety A lacks, since A refuses anything outside the project.
3. **Dropped: start `Code.exe -g file:line`.** Grace would search the disk for VS Code, and the command on the `PATH` is `code.cmd`, which needs a shell.

## Requirements

### R1. Open a test

1. A scenario's or row's details show a small ghost button "Open in VS Code" with an icon, after its `file:line`.
2. A click opens the test's feature file in VS Code at the test's line (a row's line for a row).
3. The button shows whatever the run state; opening a file never touches a run.

### R2. Open where it failed

1. When a failed test's error starts with a line that is an absolute Windows path, a colon and a line number (`C:\…\auth.feature:9`), that first line shows as a link, its text as Karate wrote it. The rest of the error shows as today.
2. A click opens that file at that line, which may be in a called feature.
3. Any other error (no such first line, such as a `karate-config.js` failure) shows as today, with no link.

### R3. Main checks the request

1. The request is a path of at most 1000 characters and a line, an integer of 1 or more; anything else is refused with the notice "Invalid request".
2. A relative path resolves as the run resolves the tree's features: the project root, then its classpath roots. An absolute path stays as it is.
3. The resolved path must be an existing file whose real path (links followed) is inside the open project's real folder. Otherwise, or with no project open, Grace shows the notice "Grace opens only files of the open project" and opens nothing.
4. Grace names the file under the project folder as the project was opened, not by its real path: a project on a mapped drive keeps its drive letter, so VS Code opens the file inside a workspace opened from that drive, and its prompt for a share host does not appear.

### R4. Open in VS Code

1. When Windows has no app for `vscode://` links, Grace shows the notice "Open in VS Code needs VS Code installed" and opens nothing.
2. Otherwise Grace hands Windows the link `vscode://file/<path>:<line>`: the path of R3.4 with `/` separators, percent-encoded where a URL needs it (`%`, space, `#`, `?`). A share path `\\server\share\…` gives `vscode://file//server/share/…`: VS Code reads `file///server` as `\server`, a folder of the current drive. Grace starts no process and no shell.

## Cost

- Idle: nothing; no timer, poll or listener is added.
- A click: one IPC call, one `realpath` and `stat` of the file, one lookup of the `vscode://` handler, one hand-off to Windows.
- Showing a failed test: one regular expression on its error's first line.
- Memory, start time: no change. Size: under 1 KB of code; no dependency.

## Design decisions already made

- VS Code only, through its `vscode://` link, with no setting: the owner's choice. No decision record: nothing is hard to reverse.
- Approach A: the owner's choice.
- The button and the error link, in the Details pane only: proposed and accepted. The error's link covers a failed step of the scenario itself, since Karate's line is that step's line, so step rows get no link.
- My call: one notice for a path outside the project, a missing file and no open project; they all mean Grace will not open it.

## Acceptance criteria

Scenarios run in Vitest (jsdom for the window, Node for main) unless marked **(e2e)**, which run in Playwright against the built app with `shell.openExternal` replaced by a recorder.

```gherkin
Feature: Open a test in VS Code

  # R1
  Scenario: The button opens the test at its line
    Given a scenario "greets" at "tests/basic.feature:12" is selected
    When I click "Open in VS Code"
    Then Grace asks main to open "tests/basic.feature" at line 12

  Scenario: The button shows during a run
    Given a run is going
    When I select a scenario
    Then "Open in VS Code" shows

  # R2
  Scenario: The error's place is a link
    Given a failed scenario whose error starts with "C:\p\tests\auth.feature:9"
    When I click the link "C:\p\tests\auth.feature:9"
    Then Grace asks main to open "C:\p\tests\auth.feature" at line 9

  Scenario: An error without a place has no link
    Given a failed scenario whose error starts with "Config evaluation failed: karate-config.js"
    Then the error shows as text, with no link

  # R3
  Scenario: A feature of the tree resolves under the classpath
    Given a Maven project with "src/test/java/users/users.feature"
    When main is asked to open "users/users.feature" at line 3
    Then it opens "<project>\src\test\java\users\users.feature"

  Scenario Outline: Main opens only files of the open project
    When main is asked to open <path>
    Then it shows "Grace opens only files of the open project"
    And it opens nothing

    Examples:
      | path                                     |
      | "../outside.feature"                     |
      | an absolute path outside the project     |
      | a file of the project that was deleted   |
      | a link inside the project to a file out  |
      | any path, with no project open           |

  Scenario: Main refuses a malformed request (e2e)
    When main is asked to open a path with line 0, 1.5 or "12"
    Then it shows "Invalid request"

  # R4
  Scenario: The link VS Code opens
    Given the checked file "C:\My Project\a#b.feature"
    Then the link is "vscode://file/C:/My%20Project/a%23b.feature:7" for line 7

  Scenario: The link to a file on a share
    Given the checked file "\\server\share\p\x.feature"
    Then the link is "vscode://file//server/share/p/x.feature:3" for line 3

  Scenario: Open a scenario in VS Code (e2e)
    Given a project with a failing scenario is open
    When I select the scenario and click "Open in VS Code"
    Then Windows is handed "vscode://file/<project path>/<feature path>:<scenario line>"
    When I run it and click its error's link
    Then Windows is handed the link to the failing step's line

  Scenario: VS Code is not installed (e2e)
    Given Windows has no app for vscode:// links
    When I click "Open in VS Code"
    Then a notice says "Open in VS Code needs VS Code installed"
    And Windows is handed nothing
```
