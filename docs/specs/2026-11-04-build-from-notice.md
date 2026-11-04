# Build the project's Java code from the not-built notice

Date 2026-11-04. Status: approved. Covers [#233](https://github.com/hpagniez/grace/issues/233). Reopens a non-goal of [Project libraries](2026-11-02-project-libraries.md) and amends its R8.6.

## Intent

A Maven or Gradle project can keep Java helpers in `src/test/java`, which its scenarios call through `Java.type` (a JDBC helper, say). Grace runs what the build compiled and says when that is missing or older than the source (project libraries R8): `The project's Java code in src/test/java is not built: build it (mvn test-compile) before running.` The person then has to leave Grace for a terminal, run the command, come back and press Refresh. A suite like that cannot run in Grace until they do.

The notice should carry a Build button. Grace runs the project's own build with the tool it already finds to list the libraries, shows that it is building, lets Cancel stop it, and refreshes when it is done.

## Goals, non-goals, anti-goals

- **Goals:** one click from the notice to a built project and a refreshed tree; a failure that says why in one line, and says when the cause is a missing JDK.
- **Non-goals:** building on open, on Refresh, before a run or on any file change; a Build button anywhere but the notice; choosing the goal, a profile or `-D` options (Grace runs the plain command, as for the listing); multi-module builds beyond what the plain command does at the project folder; finding a JDK for the person or installing one; showing the build's output (the console belongs to Karate's runs).
- **Anti-goals:** any cost while idle; a process left running after a build (a Gradle daemon); a build that starts without the person's click; a build that blocks the window.

## Constraints

- Follow the Karate CLI first (ADR 0026): the Karate CLI has no build step, so this is Grace's own way.
- Idle costs nothing (ADR 0023): the button is a flag on a notice; nothing runs until the click.
- Run lightly: Grace spawns the build tool once per click and nothing stays after it.
- Spawn from the main process without a shell of the person's choosing, as the listing does (`cmd.exe /d /s /c`, every path in a variable), and cancel with `taskkill /PID <pid> /T /F`.
- Compiling needs a JDK. Grace bundles a JRE only, which cannot compile.

### What exists today

- `testCodeNotice` (`src/main/karate/compiled.ts`) gives the R8 text on open and Refresh; `open` and `refresh` (`src/main/runner.ts`) join it with the other notices into one `notice` event. The window shows the last notice in `Notice` (`src/renderer/Notice.tsx`), and clears it on every `project` event.
- `listLibraries` (`src/main/runner.ts`) finds the tool (`findTool`, `src/main/karate/libraries.ts`), runs it by `cmd.exe` with `TOOL_COMMAND` and `toolEnv`, keeps it in `this.proc` so `cancel` kills it, sends `listing` events that the run bar, the console's state line and an empty tree turn into "Listing the project's libraries with Maven…", and turns a failure into one line with `failureReason`.
- `toolEnv` lends Grace's JRE to Maven when the person has no Java (`JAVA_HOME` unset and no `java.exe` on PATH).
- `guarded` makes Grace busy: Run, Refresh, the environment switch and opening a project are refused until it ends.

## Approaches considered

1. **Share the listing's process code. Chosen by the owner.** One way to run the build tool for both the listing and the build: the same spawn, the same `this.proc` for Cancel, the same busy guard, the same failure line. The listing's event names what the tool does, so the window says "Building…" where it says "Listing…". On success the refresh runs inside the same busy time. One copy of the process code and one Cancel path; the listing code is reworked a little, under its tests.
2. **A separate build module with its own spawn and cancel.** A second copy of the process code and a second Cancel path to keep in step.
3. **Open a terminal window that runs the command.** Grace could not show progress, cancel it or report its error line.
4. **Build on every open or Refresh.** Rejected by the owner: Grace builds only on the person's click.

## Requirements

### R1. The Build button

1. When the Java code notice of project libraries R8.2 or R8.3 shows, on open or Refresh, and Grace finds the project's build tool as the listing does (project libraries R2), the notice shows a **Build** button after its text, whatever other notices the text joins.
2. Without a tool, the notice shows no button.
3. The button is disabled while Grace is busy: a discovery, a run, a listing or a build.
4. The button goes with its notice: any later notice replaces it, and a `project` event clears it, as notices do today. A Refresh shows it again while the code is still not built.

### R2. The build

1. A click runs the build tool, in the project folder, by `cmd.exe` as the listing does (project libraries R3): Maven `"<tool>" -B -q test-compile`, Gradle `"<tool>" -q --no-daemon --console=plain testClasses`.
2. The tool gets the person's environment as it is. Grace never lends it its own Java: a JRE cannot compile.
3. Grace is busy from the click to the end of the refresh after it (R4). Run, Refresh, the environment switch and opening a project are refused as during any discovery.
4. The main process refuses a build with no project open, for a standalone project, or while busy, as it refuses other requests then.

### R3. While it builds

1. The run bar's status and the console's state line say `Building the project's Java code with Maven…` (`with Gradle…` for Gradle), where they say `Listing the project's libraries with Maven…` during a listing. The tree keeps its tests.
2. A window that reloads during the build says it too.
3. Cancel stops the tool and every child process (`taskkill /T /F`), as for the listing. Grace then shows the notice `Build cancelled.` with the Build button, and does not refresh.

### R4. When it succeeds

1. When the tool ends with exit code 0, Grace refreshes as Refresh does: it lists the libraries if their build files changed (project libraries R4), discovers the tests, and checks the Java code (project libraries R8). The not-built notice goes with the refresh's `project` event; any notice the refresh gives shows instead.
2. If the check still finds the code not built (a build that compiles somewhere else), its notice shows again with the button.
3. Grace keeps the time the successful build started in `<project data folder>/built.json`. From then on the Java code check (project libraries R8.3) counts a source as built when it is not newer than that time or than the newest `.class` file. So a Gradle build that rewrites no class, because no source's content changed though its time did (`git stash` and `pop`, a branch switched and back), still clears the notice. A source changed during or after the build is newer than that time and shows the notice. R8.2 is unchanged: no `.class` file means not built, whatever the kept time.

### R5. When it fails

1. When the tool ends with another exit code, Grace shows `Grace could not build the project's Java code with Maven: <reason>.` with the Build button, where `<reason>` is javac's first error line (`<file>:<line>: error: <message>`, which Gradle prints as javac wrote it), else follows project libraries R6.3 (Maven's first `[ERROR]`, a launcher's `ERROR:`, Gradle's deepest cause, else the last line). Gradle's own cause for a compile error, `Compilation failed; see the compiler error output for details`, points at output Grace does not show. A reason that ends with `?` gets no full stop. Grace does not refresh; the tree stays as it was.
2. When neither `%JAVA_HOME%\bin` nor any PATH folder holds `javac.exe`, that notice adds `Building needs a JDK: set JAVA_HOME to one.`
3. When Grace no longer finds the tool at the click, it shows `Grace could not build the project's Java code with Maven: <why>.` with the reason of project libraries R2 and no button.

### R6. Docs

1. The README's project line says that when the project's Java code is not built, Grace's notice has a Build button that runs `test-compile` or `testClasses`. Its "Nothing else to install" line no longer says Grace never uses the Java on the person's machine: Grace runs the tests on its own Java, and the person's build tool, which lists the libraries and builds the Java code, uses theirs.
2. The project libraries spec: its non-goals no longer exclude building, and point here; its R8.6 says Grace starts no process and changes no file on open and Refresh.

## Cost

- **Idle:** nothing. No timer, poll or watcher. The button is one flag on the last notice.
- **Open or Refresh:** when the project has Java code, one read of the small `built.json`; when the Java code notice shows, one more lookup of the build tool (a file check per PATH folder, and the IntelliJ folders for Maven): about a millisecond. Nothing new otherwise.
- **On a click:** one `cmd.exe` and the build tool's JVM, for as long as the build takes: typically a few hundred MB and some seconds of CPU, more with Maven downloading what is not in `~/.m2` yet. Gradle runs without a daemon, so nothing stays after the build. Then the refresh's usual cost.
- **Disk:** one `built.json` of a few bytes per project built from the notice, in Grace's data folder.
- **Memory, start time:** unchanged. **Size:** a few KB of code; no dependency.

## Design decisions already made

By the owner:

- Reopen the "never builds" non-goal: Grace builds, but only when the person clicks Build.
- A successful Build counts as built (R4.3): Gradle compiles by content, not by time, so a Build that rewrote no class would otherwise leave the notice for ever.
- The JDK check: always try the build; when it fails and no `javac.exe` is found, the notice says a JDK is needed (R5.2), so a project whose build finds a JDK by its own toolchains is never blocked.
- Approach 1: share the listing's process code.

The builder's calls, open to the owner's correction:

- The button sits at the end of the notice, and is disabled while busy (R1.1, R1.3).
- "The tree says so" (the issue) is read as the place that says the libraries are being listed: the run bar and the console's state line (R3.1). The tree only says it with no tests yet, and a project with a not-built notice has its tree.
- The commands: `-B -q` for Maven (no colour codes, only the errors), `-q --no-daemon --console=plain` for Gradle (no daemon left, no progress bars) (R2.1). No time limit: Cancel stops it.
- A cancelled build says `Build cancelled.` and keeps the button (R3.3); the not-built check does not run again until the next Refresh.
- A failed build keeps the tree and does not refresh (R5.1): nothing changed that discovery would see.
- javac's first error line comes before project libraries R6.3 (R5.1), so a Gradle compile error names the file and the error, as Maven's does.
- Runs and environment switches still clear the notice and never check the Java code (project libraries R8.5): the button comes back with Refresh.
- The README's "Nothing else to install" line changes (R6.1): the listing already runs the person's build tool on their Java, and Build needs their JDK.

Known limit: a build cancelled half-way may leave some classes newer than the sources, so the check can say nothing while a class is missing, as with project libraries R8.6's `git checkout`.

## Left to the builder

- One private way in `Runner` to run a build tool, used by `listLibraries` and the build, which owns `this.proc`, the `listing` state and its replay.
- The `listing` event names the task (list or build); `findingText` picks the text.
- The `notice` event gets an optional `build` flag; the window's state keeps it with the notice text, and `Notice` shows the button when set.
- A new IPC call `build`, checked in main (R2.4); the refresh's body shared by `refresh` and the build.
- The JDK lookup next to `findTool`, with the same PATH folder reading.

### Tests

- Unit tests (`src/main/runner.libraries.test.ts` or a sibling): the open and Refresh notices carry `build` with a tool and not without; a click runs the stub tool with `test-compile` (Maven) and `--no-daemon testClasses` (Gradle) in the project folder, without `JAVA_HOME` set to Grace's JRE; success refreshes and the notice goes; a Gradle stub that writes no class still clears the "changed" notice, and a source touched after the build brings it back; failure gives the line and the button; the JDK sentence with and without `javac.exe`; Cancel stops the stub and says `Build cancelled.`; `listing` events name the build, also in a replay; refused while busy and for a standalone project.
- Renderer unit tests: the notice with and without the button, disabled while busy; the run bar and the console say `Building the project's Java code with Maven…`.
- One e2e test on the packaged app: a Maven project with a `mvnw.cmd` stub that writes `target/test-classes/…/DbHelper.class`; open, the notice shows Build; click; the notice goes.

## Acceptance criteria

```gherkin
Feature: Build the project's Java code from the not-built notice

  Background:
    Given a Maven project with src/test/java/com/example/qa/DbHelper.java and no target/test-classes
    And its mvnw.cmd

  Scenario: The notice offers to build
    When I open it
    Then the notice says "The project's Java code in src/test/java is not built: build it (mvn test-compile) before running."
    And the notice has a Build button

  Scenario: No tool, no button
    Given the project has no mvnw.cmd and no Maven is found
    When I open it
    Then the notice says the project's Java code is not built
    And the notice has no Build button

  Scenario: Building, then refreshing
    Given I opened it
    When I click Build
    Then mvnw.cmd runs "test-compile" in the project folder
    And the status says "Building the project's Java code with Maven…"
    When mvnw.cmd writes target/test-classes/com/example/qa/DbHelper.class and ends with exit code 0
    Then Grace refreshes the tests
    And no notice shows

  Scenario: A build that rewrites no class still counts
    Given a Gradle project with src/test/java/com/example/qa/DbHelper.java older than build/classes/java/test/com/example/qa/DbHelper.class, and its gradlew.bat
    And DbHelper.java's time moves without its content changing
    When I open it
    Then the notice says "The project's Java code in src/test/java changed since its last build: build it again (gradle testClasses) before running."
    When I click Build
    And gradlew.bat writes no class and ends with exit code 0
    Then no notice shows
    When DbHelper.java changes
    And I refresh
    Then the notice says the project's Java code changed since its last build

  Scenario: A Gradle project
    Given a Gradle project with src/test/java/com/example/qa/DbHelper.java, no build/classes/java/test, and its gradlew.bat
    When I open it and click Build
    Then gradlew.bat runs "testClasses" without a daemon
    And the status says "Building the project's Java code with Gradle…"

  Scenario: The build fails
    Given I opened it
    And mvnw.cmd prints "[ERROR] COMPILATION ERROR" and ends with exit code 1
    When I click Build
    Then the notice says "Grace could not build the project's Java code with Maven: COMPILATION ERROR."
    And the notice has a Build button
    And the tree is as it was

  Scenario: No JDK
    Given JAVA_HOME holds no bin\javac.exe and no PATH folder holds javac.exe
    And I opened it
    And mvnw.cmd prints "[ERROR] No compiler is provided in this environment. Perhaps you are running on a JRE rather than a JDK?" and ends with exit code 1
    When I click Build
    Then the notice says "Grace could not build the project's Java code with Maven: No compiler is provided in this environment. Perhaps you are running on a JRE rather than a JDK? Building needs a JDK: set JAVA_HOME to one."

  Scenario: Cancel while building
    Given I opened it
    And mvnw.cmd takes a while
    When I click Build
    And I cancel while it builds
    Then mvnw.cmd and its child processes have stopped
    And the notice says "Build cancelled."
    And the notice has a Build button

  Scenario: Busy
    Given I opened it
    And the project's tests take a while to find
    When I refresh
    Then the Build button is disabled until the tests are found

  Scenario: Grace builds only on a click
    When I open it
    And I refresh
    And I run all tests
    Then mvnw.cmd never ran "test-compile"
```
