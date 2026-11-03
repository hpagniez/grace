# Export a run's report

Date 2026-11-03. Status: approved. Covers [#70](https://github.com/hpagniez/grace/issues/70). Builds on [Run history](2026-10-25-run-history.md) and [Project setup](2026-11-02-project-setup.md).

## Intent

A run's report lives in Grace's data folder (`%APPDATA%\Grace\…\runs\<id>\`), deep out of sight, and is deleted once 20 newer runs exist. To show a result to a colleague, attach it to a ticket or feed it to a tool that reads JUnit XML (Jenkins, Xray, TestRail), the person needs a copy outside Grace. Export saves one run's report as a zip, where the person picks. JUnit XML is in it when the project asks Karate for it, the way the Karate CLI decides.

## Goals, non-goals, anti-goals

- Goals: any run in the history that has a report can be saved as one zip; the zip opens as Karate's HTML report once unzipped; a project whose `karate-pom.json` sets `output.junitXml` gets Karate's JUnit XML in every run, and so in the zip.
- Non-goals: Cucumber JSON; a JUnit switch in Grace; exporting several runs at once; an Export button in the toolbar; opening the run's folder; remembering the last export folder; sending the zip anywhere.
- Anti-goals: any cost while idle; any cost to a run whose project does not ask for JUnit XML; main zipping a path the window sent; a half-written zip left behind; a new dependency.

## Constraints

- Follow the Karate CLI first ([ADR 0026](../decisions/0026-follow-the-karate-cli-first.md)). The CLI writes JUnit XML when `-f` names `junit:xml`, or, with no `-f`, when `karate-pom.json` sets `output.junitXml: true` (`RunCommand.java`, `KaratePom.java`, Karate 2.1.4). Grace always passes `-f` and `--no-pom`, so it adds `junit:xml` to its `-f` itself when the pom asks.
- The window sends main a run's id only, never a path, as the Report button does ([run history R4](2026-10-25-run-history.md)).
- Karate's reports are not masked by Grace: a report holds a secret unless the project sets `configure logging mask`. The Properties dialog already says so ([environment switcher R5](2026-10-25-environment-switcher.md#r5-secrets)).

### What exists today

- `buildArgs` in `src/main/karate/command.ts` passes `-f html,karate:jsonl` to every run and to the dry run that lists the tests. `spawn` in `src/main/runner.ts` calls `detectLayout(root)` at each start, so it reads `karate-pom.json` again for every run.
- `readPom` in `src/main/karate/pom.ts` takes `paths` and `configDir` and lists every other top-level key as unused: the open notice says "Grace does not use these karate-pom.json settings: output, …".
- A run's folder holds Karate's report (`karate-summary.html`, `feature-html/`, `embeds/` and the like), Karate's event file `karate-json/karate-events.jsonl`, and Grace's `run.json`.
- Each history row (`src/renderer/History.tsx`) has Report, Re-run and Re-run failed. Report is enabled when the run has a report (`reportAvailable`: the run finished and `karate-summary.html` exists) and Grace is not busy. Main finds the report with `Runner.runReportPath(id)`.
- The updater already runs Windows' own `tar.exe` (`TAR` in `src/main/updates/fetchZip.ts`) to unzip.

### What Karate's JUnit XML holds

`JunitXmlReportListener` writes `junit-xml/<feature>.xml`, one per feature, on that feature's thread as it ends. Each `<testcase>` carries the scenario's step logs, HTTP bodies included, in `<system-out>`, and its tags as `<property>`. Karate applies the project's log mask to it, as to the HTML report.

## Approaches considered

What Export does:

1. **Save a zip. Chosen by the owner.** One file to attach to a mail or a ticket. Windows' own `tar.exe`, already used by the updater, writes it: no dependency.
2. **Copy the run's folder to a place the person picks.** Slightly less code, but sharing it means zipping it by hand.
3. **Open the run's folder in Explorer.** The least code; the person still has to find, copy and zip the files before the run is pruned.

How the zip is made, my call: Windows' `tar.exe` (chosen) over a zip library (about 30 to 100 KB more to download, for what Windows ships) and over PowerShell's `Compress-Archive` (PowerShell's slower start, for the same result).

When JUnit XML is written:

1. **When `karate-pom.json` sets `output.junitXml: true`. Chosen by the owner.** What the CLI does; costs nothing to a project that does not ask.
2. **Every run.** No setting to find, but every run folder gets a third copy of the step logs (after the HTML report and the event file), about half as much disk again, kept for 20 runs.

Which runs: **any run in the history (chosen by the owner)** over the last run only. Where: **an Export button in each history row (chosen by the owner)**, not in the toolbar too: the newest run is the top row.

## Requirements

### R1. JUnit XML when the project asks

1. When the project's `karate-pom.json` has `"output": { "junitXml": true }`, a run, a re-run and a debug run pass `-f html,karate:jsonl,junit:xml`, and Karate writes `junit-xml/` in the run's folder. Otherwise they pass `-f html,karate:jsonl` as today. The dry run that lists the tests never passes `junit:xml`.
2. Grace reads it at each run start, as it reads `paths` and `configDir`: a change to the file applies from the next run.
3. `output.junitXml` set to anything but `true` or `false` makes the file unreadable, as a wrong `paths` does: the open notice says "karate-pom.json could not be read: output.junitXml is not true or false. Grace uses the default folders."
4. The open notice lists `output`'s other keys as `output.<key>` (for example `output.dir`, `output.cucumberJson`) among the settings Grace does not use, and no longer lists `output.junitXml`. An `output` that is not an object is listed as `output`, as today, and turns nothing on.

### R2. The Export button

1. Each history row has an Export button between Report and Re-run. It is enabled exactly when the row's Report button is.
2. A click sends main the run's id only. Main refuses an id that is not a run id with the notice "Invalid request". A run that is no longer in the history, or has no report, does nothing, as Report does.
3. Main shows Windows' Save dialog, titled "Export report", with the file type "Zip archive (*.zip)" and the proposed name `<project folder name>-report-<run id>.zip` in the person's Downloads folder.
4. Closing the dialog without saving does nothing.

### R3. The zip

1. It holds everything in the run's folder except Grace's `run.json` and Karate's event folder `karate-json`, with `karate-summary.html` at its root, so the report opens once it is unzipped.
2. File names keep their characters: names are written in UTF-8.
3. Main writes it with `%SystemRoot%\System32\tar.exe` in zip format, without a shell and with no window. It names the run folder's entries itself; nothing the window sent reaches the command. tar reads its arguments in the ANSI code page, so no path goes on its command line: it runs in the run's folder and writes the zip to its output, which main saves next to the chosen path and then moves onto it. Any chosen path and any run folder work, whatever their characters.
4. When it is written, a success notice says "Report saved to <full path>".
5. When it cannot be written, a notice says "The report could not be saved: <reason>", no partial file is left, and the chosen path keeps what it had: nothing, or the file the person meant to replace.
6. A file the person chose to replace in the Save dialog is replaced.

## Cost

- Idle: nothing. No timer, poll or listener; the Export button is a plain button in at most 20 history rows, drawn only while the History tab is open. The bench never opens it, so `page.nodes.*` do not move.
- An export: one `tar.exe` for as long as the zip takes (a fraction of a second for a typical report), then it exits. Disk: the zip, where the person put it.
- A run whose project does not set `output.junitXml`: no change. One that does: Karate serialises one XML file per feature on that feature's thread, and the run folder grows by about another copy of the step logs, kept for 20 runs. The bench's projects have no `karate-pom.json`, so no bench measure moves.
- Start time: no change. Size: about 1 to 2 KB of code; no dependency.

## Design decisions already made

- JUnit XML only when `karate-pom.json` asks, as the Karate CLI does: the owner's choice.
- Export saves a zip: the owner's choice.
- Any run in the history, from its row only: the owner's choice.
- My call: the zip leaves out `run.json` (Grace's, not part of the report) and `karate-json/` (Karate's event stream, which Grace reads live; no report viewer opens it).
- My call: Cucumber JSON (`output.cucumberJson`) is not honoured and stays listed as unused; #70 asks for JUnit XML and HTML only.
- My call: the proposed place is Downloads every time; the proposed name carries the project and the run id, so two exports never propose the same name.
- My call: a success notice confirms the save, since nothing else shows that it happened.
- My call: no secrets warning at export. The Properties dialog already says Karate's report can hold secrets, and Windows' Save dialog has no room for a message.
- My call: `tar --format zip` rather than `-a`, so the format does not depend on the name the person typed.

## Acceptance criteria

Scenarios run in Vitest (jsdom for the window, Node for main) unless marked **(e2e)**, which run in Playwright against the built app with `dialog.showSaveDialog` replaced by one that returns a path in a temporary folder.

```gherkin
Feature: Export a run's report

  # R1
  Scenario: The project asks for JUnit XML
    Given the project's karate-pom.json sets output.junitXml to true
    When I run a test
    Then Karate is started with "-f html,karate:jsonl,junit:xml"
    And the dry run that lists the tests is started with "-f html,karate:jsonl"

  Scenario: The project does not ask for JUnit XML
    Given the project's karate-pom.json sets output.junitXml to false
    When I run a test
    Then Karate is started with "-f html,karate:jsonl"

  Scenario: A wrong value makes the file unreadable
    Given the project's karate-pom.json sets output.junitXml to "yes"
    When I open the project
    Then the notice says "karate-pom.json could not be read: output.junitXml is not true or false. Grace uses the default folders."

  Scenario: The other output keys are listed as unused
    Given the project's karate-pom.json has output with junitXml true and dir "target/x"
    When I open the project
    Then the notice says "Grace does not use these karate-pom.json settings: output.dir."

  # R2
  Scenario: Export follows Report
    Given the history has a finished run with a report and a cancelled run
    Then the finished run's Export button is enabled
    And the cancelled run's Export button is disabled
    And while a run is going, every Export button is disabled

  Scenario: Main refuses a malformed id
    When main is asked to export the run "..\x"
    Then it shows "Invalid request"
    And it shows no Save dialog

  Scenario: Closing the dialog does nothing
    When I export a run and close the Save dialog
    Then no file is written and no notice shows

  # R3
  Scenario: The zip holds the report
    Given a run folder with karate-summary.html, feature-html/café.html, junit-xml/a.xml, karate-json/karate-events.jsonl and run.json
    When main zips it to "out.zip"
    Then out.zip holds karate-summary.html, feature-html/café.html and junit-xml/a.xml
    And it holds neither run.json nor anything under karate-json

  Scenario: Any characters in the chosen path
    When main zips a run folder named "отчёт łódź" to "報告/r łódź.zip"
    Then "報告" holds only "r łódź.zip", which holds the report

  Scenario: A zip that cannot be written leaves nothing
    Given the run's folder is deleted while it is zipped
    When main zips it to "out.zip"
    Then the notice starts with "The report could not be saved:"
    And there is no out.zip

  Scenario: Export a run's report (e2e)
    Given a project whose karate-pom.json sets output.junitXml to true is open
    When I run it, open History and click the top row's Export
    Then the success notice says "Report saved to <the chosen path>"
    And the zip holds karate-summary.html and one junit-xml file per feature
```
