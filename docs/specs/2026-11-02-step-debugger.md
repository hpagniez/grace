# Step debugger: breakpoints on steps, pause, step, inspect variables

Date 2026-11-02. Status: approved. Covers [#65](https://github.com/hpagniez/grace/issues/65). Research: [docs/research/2026-11-02-step-debugger/](../research/2026-11-02-step-debugger/README.md). Toolchain: [ADR 0025](../decisions/0025-compile-the-debug-listener-with-ecj-on-the-bundled-jre.md).

## Intent

When a Karate test fails, people read the step log and the report and guess what a variable held at that point. Karate Labs sells step debugging in its paid IDE plugins. Grace gives it for free: stop a test before a chosen step, see every variable the scenario holds, then step through or carry on.

Open-source Karate has no debug server. Grace brings its own: a small Java `RunListener` passed with `--listener`, which blocks before a step until the app says go, and sends the scenario's variables over a local socket.

## Who and what success looks like

Someone writing or fixing Karate tests, with Grace open beside the IDE. Success: they set a breakpoint on a step in a couple of clicks, start a debug run from the test's row, and when it stops they read the variables without leaving Grace, then step to the next step or continue to the end. When they are not debugging, nothing about Grace is heavier or slower.

## Goals, non-goals, anti-goals

- Goals: breakpoints on the steps of any test in the tree, before or after a run; a debug run of any tree row; pause, continue, step over, step into; the paused scenario's variables, with their type and value.
- Non-goals (this spec): conditional breakpoints, hit counts, watch expressions, evaluating an expression, editing a variable, step out, breaking on a failed step, breakpoints inside called features that are not in the tree, keeping breakpoints after the project is closed, debugging on more than one thread.
- Anti-goals: any cost to a normal run (it gets no listener, no socket, no new argument); any timer or poll while a debug run is paused; Grace hanging when Java dies or the socket breaks; a debug run that leaves Java running.

## Constraints

- `AGENTS.md`: Karate integration rules, Solid 2 rules, UI conventions (semantic tokens, `Icon`, `Tooltip`, `StatusMark`, shared class strings), idle costs nothing, debugging runs one thread.
- Karate 2.1.4's listener interface as the research found it: `RunListener.onEvent(RunEvent)`, `StepRunEvent` fired before a step runs, `ScenarioRuntime.getAllVariables()`, `FeatureRuntime.getCallDepth()`. Never Karate's paid debug hooks (`karate.debug.port`, `io.karatelabs.debug.Main`).
- The listener ships as `resources/listener.jar`, compiled from `listener/` by `npm run vendor` (ADR 0025).

### What exists today

- A run (`Runner.runTarget` then `runBody`) does a dry run, then spawns Karate with `-T <threads>`, tails the JSONL every 100 ms and emits events to the window. `Cancel` kills the process tree.
- Each tree row has a Run button (Ctrl+Enter on the selected row). The details pane shows a scenario's steps only once its feature finishes (from `FEATURE_EXIT`), each with its line, keyword, text, status, time and log.
- The bottom panel has the Console, History and HTTP tabs. The toolbar holds `Cancel` and `Run all` at its right end.
- Events to the window go through the run's masker, which hides secret property values.

## Approaches considered

Breakpoints: on step rows in the tree (each scenario's steps as children), on the steps in the details pane, or in a feature file view. The tree has no step rows, and adding them would multiply its rows; the feature file editor (#79) is not built. **Chosen** by the owner: the details pane.

Controls: in the toolbar, or in a bar inside the details pane. **Chosen** by the owner: the toolbar, in place of `Run all` while a debug run is going.

Variables: a section of the details pane under the steps, or a bottom panel tab. **Chosen** by the owner: a `Variables` tab in the bottom panel.

Steps before a run (needed to set a breakpoint on a test that has not run): kept from the dry run, or read from the feature file when the test is selected. The dry run lists every step of every scenario, but keeping them costs memory for the whole project, idle (around 4 MB on the bench's 10,000 tests). Reading the file costs one small read on selection and nothing idle. **Chosen** (builder's call): the file.

## Requirements

### R1. The debug listener

1. `listener/grace/DebugListener.java` implements `io.karatelabs.core.RunListener` and uses only `java.base` and Karate's public classes named in the research.
2. Its constructor connects to `127.0.0.1` on the port in the system property `grace.debug.port`, sends the token in `grace.debug.token` as its first line, then reads one line: the breakpoints. A reader thread then takes the app's later lines.
3. On each `STEP_ENTER` it pauses when any of these hold: the step's feature path (`getResource().getRelativePath()`) and line are a breakpoint; a pause was asked; the last command was step into; the last command was step over and the step's call depth is at most the paused step's depth. Otherwise it returns `true` at once.
4. To pause, it sends one line with the feature path, line, call depth, the step as written (prefix, keyword, text), the paused test (the feature path and `refId` of the depth-0 scenario, found through `getCallerScenario()`) and the scenario's variables, then blocks until a command arrives.
5. Each variable is sent as name, kind and value. Kind: `string`, `number`, `boolean`, `null`, `json` (a `Map` or `List`, value as JSON), `xml` (a DOM `Node`, value as XML text), `function`, or `other` (with the Java class's simple name, value as `toString()`). A value longer than 64 KB is cut there and says how much was left out.
6. Any error (the app gone, a value that cannot be read) stops the pausing for the rest of the run: `onEvent` returns `true` and the run goes on. It never throws.

### R2. The debug run in the main process

1. A debug run is a run like any other (its folder, history record, report, notices, cancel) with these differences: one thread whatever the Threads setting, `listener.jar` after `karate.jar` on the classpath, `--listener grace.DebugListener`, and `-Dgrace.debug.port=<port> -Dgrace.debug.token=<token>`. A normal run and the dry run are unchanged.
2. Before Java starts, the main process listens on `127.0.0.1`, port 0, and makes a random token. It accepts one connection whose first line is the token, then stops listening. Any other connection is closed.
3. It sends the project's breakpoints as the first line, and again whenever they change during the run. A pause asked before the listener connects goes with the first line.
4. On a pause it stops tailing the JSONL (no timer runs while paused) after one last read, and emits `debugPaused` with the step, its call depth, the paused test's key and the variables, through the run's masker. On continue, step over or step into it sends the command, emits `debugResumed` and tails again. A window that reloads gets the breakpoints again.
5. Masking: a value Java cut at 64 KB also has its cut end masked when that end is the start of a secret's form. The masker also knows each secret's XML-escaped form, as an `xml` value shows it.
6. `Cancel` while paused or running kills Java as for any run; the run ends cancelled. When the run ends, the socket and the server close. A socket that closes or fails while Java runs is not an error: the run goes on, without pausing.
7. The record in the history says `threads: 1` and `debug: true`. Flaky detection leaves debug runs out: a long pause can fail a test that passes otherwise (an expired token, a server timeout). A re-run of a debug run from the history is a normal run.

### R3. Breakpoints

1. A breakpoint is a feature path (the tree's `file`, which is Karate's feature path) and a step line. The main process keeps the open project's breakpoints in memory; closing the project or the app drops them.
2. The window toggles a breakpoint through the preload API; the main process sends the full set back as `breakpoints`, and to the listener when a debug run is going.
3. A breakpoint on a line that is no longer a step stays and never fires; it shows only on step rows whose line it is.

### R4. Steps in the details pane

1. A selected scenario or row with no run steps shows its steps as written in the feature file: the background's steps, then its own, each with its line. The main process reads the file when the window asks, for that one test; a row shows its outline's steps.
2. A step line is a line whose first word is `*`, `Given`, `When`, `Then`, `And` or `But`, outside a doc string (`"""` or ```` ``` ````), within the background or the test's own section.
3. These rows have no status, time or log. The run's steps replace them when its feature finishes, as today.
4. Each step row, run or not, has a breakpoint toggle at its left: a button, named "Add breakpoint at line N" or "Remove breakpoint at line N", that shows a `failed`-coloured dot when the breakpoint is set and an outline of it on hover or focus.
5. A background step's breakpoint shows in every scenario of the feature, and pauses each of them.

### R5. Starting a debug run

1. Each tree row with a Run button gets a Debug button just before it, with a bug icon and the tooltip "Debug <name> (Ctrl+Shift+Enter)". Ctrl+Shift+Enter on the selected row does the same; Ctrl+Enter without Shift stays a normal run. Both are disabled as the Run button is.
2. A feature row, and a scenario or outline row, shows a small `failed`-coloured dot after its name while one of its lines holds a breakpoint (for a scenario or outline: a line from its own line up to the next test's).

### R6. The toolbar during a debug run

1. While a debug run is going, `Run all` gives way to the debug controls; `Cancel` stays.
2. While Java runs: `Pause` (asks the listener to stop before the next step).
3. While paused: `Continue` (F5), `Step over` (F10) and `Step into` (F11). The keys work anywhere in the window while paused.
4. The controls follow the toolbar's narrow rule: icon only below 1280 px, with the label as a tooltip.

### R7. While paused

1. The paused test is selected in the tree, its parent rows opened and the row scrolled into view. Its details show its steps (R4) with the paused step marked: a `running`-tinted row and the word "Paused" beside its time.
2. Nothing spins while paused. The paused test's mark, in its row and its details badge, is a pause icon in the `running` colour (a mark of the window, not a new test status). Other `running` marks stop where they are. The progress line says "Paused" with the pause icon. Its elapsed time stops its timer and shows the time at the pause; on resume it shows the time since the start again, pause included, as Karate's own duration does.
3. Inside a called feature, the details show the called feature's file and line under the test's steps ("Paused in called.feature:5") in place of a marked row.
4. A `Variables` tab sits after the HTTP tab while a debug run is going. The first pause of a run selects it. It lists the variables by name: name, kind, then the value, one line cut with `Cut`; a `json` or `xml` value opens, like a step log, into a box with the full value, JSON indented.
5. While Java runs between pauses the tab shows the last pause's variables, dimmed, with "Running…". When the run ends the tab goes away; if it was showing, the Console shows.

## Cost

- Size: `listener.jar`, under 10 KB, in `resources/`. `ecj.jar` (3.4 MB) is a build tool in `vendor/`, never shipped. No npm dependency.
- Idle and normal runs: nothing new. No server, socket or listener exists outside a debug run. Breakpoints are a small set in memory. Reading a test's steps from its feature file happens once per selection of a test that has no run steps, a few KB.
- Start: no change. The listener is loaded only by a debug run's Java.
- During a debug run: Java on one thread (the user asked for it). The listener checks each event with one type test and a set lookup per step. Each pause sends the variables once, at most 64 KB per value.
- While paused: zero CPU. Java is blocked on a socket read, the main process waits on the socket, the JSONL tail is stopped, and nothing animates.

## Design decisions already made

- Breakpoints in the details pane, controls in the toolbar, variables in a bottom panel tab: the owner's choice.
- Builder's calls: steps before a run read from the feature file (R4.1, cost above); breakpoints kept in memory only (keeping them across restarts risks lines that moved); a debug run with no breakpoint runs to the end, like an IDE; F5, F10 and F11 as in the common IDEs; a local TCP socket with a token, accepting one connection (Node's named pipes and Java's Unix sockets do not meet on Windows); values cut at 64 KB; a paused test keeps the `running` status (no new test status) and shows a pause mark, with every spinner and the elapsed clock stopped (R7.2); debug runs flagged in `run.json` and left out of flaky detection (R2.7), a format change allowed before 1.0.0.

## Acceptance criteria

Scenarios run in Vitest unless marked **(e2e)**, which run in Playwright against the built app. Main-process scenarios run the bundled Java and Karate.

```gherkin
Feature: Step debugger

  Background:
    Given the project "debug" with "debug.feature":
      """
      Feature: debug

        Background:
          * def base = 'from background'

        Scenario: variables
          * def name = 'grace'
          * def user = { id: 7 }
          * def called = call read('called.feature') { arg: 'in' }
          * match user.id == 7

        Scenario: second
          * def other = 'second'
      """
    And "called.feature" tagged @ignore with the step "* def inner = arg + '!'" on line 5

  # R1, R2
  Scenario: A breakpoint pauses before its step, with the variables
    Given a breakpoint on debug.feature line 8
    When I debug "variables"
    Then the run pauses at debug.feature line 8, depth 0
    And the variables are base "from background" (string) and name "grace" (string)
    When I continue
    Then the run finishes and both scenarios passed

  Scenario: Step over stays at the same depth
    Given a breakpoint on debug.feature line 9
    When I debug "variables"
    And I step over
    Then the run pauses at debug.feature line 10, depth 0
    And the variable called is json holding arg "in" and inner "in!"

  Scenario: Step into enters a called feature
    Given a breakpoint on debug.feature line 9
    When I debug "variables"
    And I step into
    Then the run pauses at called.feature line 5, depth 1

  Scenario: Pause stops before the next step
    Given no breakpoint
    When I debug the feature and ask to pause before it starts
    Then the run pauses at debug.feature line 4

  Scenario: A debug run uses one thread and the listener; a normal run does not
    Given the Threads setting is 4
    Then a debug run's arguments hold "-T 1", "--listener grace.DebugListener" and listener.jar on the classpath
    And a normal run's arguments hold none of them

  Scenario: Only the token's connection is accepted
    When a debug run starts
    And another connection sends a wrong token
    Then that connection is closed and the listener's connection is served

  Scenario: Cancel while paused
    Given the run is paused
    When I cancel
    Then the run ends cancelled and no Java process of the project is left

  Scenario: A broken socket does not stop the run
    Given the run is paused
    When the app's end of the socket closes
    Then the run goes on to the end without pausing

  Scenario: Secrets are masked in variables
    Given a property "token" marked secret with the value "s3cr3t"
    And a step "* def auth = karate.properties['token']" before the breakpoint
    When the run pauses
    Then the variable auth shows "****" in place of "s3cr3t"

  Scenario: A long value is cut
    Given a variable holding 100 KB of text
    When the run pauses
    Then its value is the first 64 KB and says how much was left out

  Scenario: Debug runs do not prove a test flaky
    Given a normal run where "variables" passed
    And a debug run, with no file changed, where "variables" failed
    Then "variables" is not marked flaky
    And the debug run's record says debug: true and threads: 1

  # R3, R4
  Scenario: Steps before a run, from the feature file
    When I select "variables" before any run
    Then the details list lines 4, 7, 8, 9 and 10 as written, with no status
    And a step inside a doc string is not listed

  Scenario: Setting and removing a breakpoint
    When I press "Add breakpoint at line 8"
    Then a dot shows on line 8 and the button is named "Remove breakpoint at line 8"
    And the "variables" row and the debug.feature row show a breakpoint dot
    When I press it again
    Then no dot shows

  Scenario: A background breakpoint shows in every scenario
    Given a breakpoint on line 4
    When I select "second"
    Then its line 4 shows the dot

  # R5, R6, R7
  Scenario: Debugging from the tree, paused, stepping, finished (e2e)
    Given a breakpoint on debug.feature line 8
    When I press the Debug button of "variables"
    Then the toolbar shows Continue, Step over and Step into, and Cancel, and no Run all
    And "variables" is selected, line 8 is marked "Paused"
    And the Variables tab is selected and lists base and name
    When I press F10
    Then line 9 is marked "Paused" and the Variables tab lists user
    When I press Continue
    Then the run finishes, Run all is back and the Variables tab is gone

  Scenario: The keyboard starts a debug run or a normal run
    Given "variables" is selected
    When I press Ctrl+Shift+Enter
    Then a debug run of "variables" starts
    And Ctrl+Enter starts a normal run

  Scenario: Nothing spins while paused
    Given the run is paused in "variables"
    Then its row and its details badge show the pause mark, not spinning
    And no running mark spins
    And the progress line says "Paused" and its elapsed time does not change
    When I continue
    Then the elapsed time counts from the run's start again

  Scenario: Pause while running
    Given a debug run is going and not paused
    Then the toolbar shows Pause
    When I press Pause
    Then the run pauses before the next step

  Scenario: Paused in a called feature
    Given the run is paused at called.feature line 5
    Then the details show "Paused in called.feature:5"

  Scenario: Between pauses the variables wait
    Given the run is paused
    When I step over
    Then until the next pause the Variables tab shows the last variables dimmed, with "Running…"

  Scenario: The debug controls at 760 px (e2e)
    Given the window is 760 px wide and the run is paused
    Then Continue, Step over, Step into and Cancel are icons, whole inside the window
```
