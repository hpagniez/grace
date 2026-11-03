# Find Maven and Gradle when they are not on PATH

Date 2026-11-03. Status: draft. [#231](https://github.com/hpagniez/grace/issues/231). Extends R2 and R6.3 of [the project libraries spec](2026-11-02-project-libraries.md).

## Intent

Grace lists a Maven or Gradle project's libraries with the project's wrapper, else with `mvn` or `gradle` on PATH. On Windows many Java developers have neither: their IDE runs its own Maven. Grace then says it could not list the libraries, and every test that opens a database fails with `No suitable driver found`. Seen on a Maven work suite with no `mvnw` and no `mvn` on PATH.

Grace should also find the Maven or Gradle that the machine has elsewhere: where the tool's own variables point, and the Maven that IntelliJ IDEA bundles. When it finds none, the notice says where it looked and how to fix it.

The Karate CLI has no way to list a build's libraries (project libraries spec, Intent), so this is Grace's own way, as ADR 0027 already decided for the listing.

## Requirements

### R1. Where Grace looks

When the project has no wrapper (`mvnw.cmd`, `gradlew.bat`), Grace takes the first launcher it finds, in this order, and looks no further:

1. PATH, as today: `mvn.cmd` or `mvn.bat`, `gradle.bat`.
2. Maven: `bin` under `MAVEN_HOME`, then under `M2_HOME`. Gradle: `bin` under `GRADLE_HOME`. The same launcher names as on PATH; a quoted value is read as the folder inside its quotes.
3. Maven only: IntelliJ IDEA's own Maven, `plugins\maven\lib\maven3\bin\mvn.cmd`, in each folder directly under these two folders, the highest build first:
   - `%ProgramFiles%\JetBrains`: the installer, installed for all users.
   - `%LOCALAPPDATA%\Programs`: the Toolbox App's default tools folder, and the installer for one user (its default mode).

   The build is the IDE's `build.txt` (`IU-261.24374.151`) less its product code, compared number by number. A folder with no `build.txt` comes last.
4. Grace runs what it found as it runs a launcher on PATH (project libraries spec R2.4 and R3): the person's environment, and Grace's Java only when there is no other.

### R2. When Grace finds none

The notice of project libraries R6.3 gives the places and the fix:

- Maven: `Grace could not list the project's libraries with Maven: the project has no mvnw.cmd, and mvn is not on PATH, in MAVEN_HOME, in M2_HOME or in IntelliJ IDEA. Tests that use them will fail. Install Maven, or add the Maven wrapper to the project.`
- Gradle: `Grace could not list the project's libraries with Gradle: the project has no gradlew.bat, and gradle is not on PATH or in GRADLE_HOME. Tests that use them will fail. Install Gradle, or add the Gradle wrapper to the project.`
- With a kept list, `Grace uses the libraries it listed before.` takes the place of `Tests that use them will fail.`, as today.

### R3. Docs

The README's project line names the places Grace looks.

## Cost

- **Idle:** zero. Nothing is watched or polled.
- **Open or Refresh, kept list still good, or a wrapper:** nothing more than today. The lookup runs only when the tool must run (project libraries R4.3).
- **Lookup:** PATH's folders as today, then up to two file checks for the home variables, then for Maven two folder listings (`%ProgramFiles%\JetBrains`, `%LOCALAPPDATA%\Programs`, a few dozen entries), one file check per entry and one small `build.txt` read per IntelliJ IDEA found. Well under 10 ms, and only when nothing earlier in R1 was found.
- **Runs:** none; they use the kept list.
- **Size:** no dependency, a few dozen lines.
- **Bench:** the bench project is standalone; its numbers do not move.

## Design decisions

The issue's order and places. Mine, open to correction:

- The home variables take the same launcher names as PATH (`mvn.bat` too, which Maven 3.0 to 3.2 shipped), so they read as one list of folders.
- Every folder under the two IntelliJ parents is checked for the bundled Maven, not only those named `IntelliJ IDEA…`: Toolbox and the installer name them differently (`IntelliJ IDEA`, `IntelliJ IDEA Ultimate`, `IntelliJ IDEA Community Edition 2024.3`), and a folder without the bundled Maven is skipped anyway.
- Newest by `build.txt`, not by folder name: Toolbox's folder names carry no version.
- Not looked at: a Toolbox tools folder moved in its settings (Toolbox 2.1 and later), the folders of Toolbox before 2.0 (2023) under `%LOCALAPPDATA%\JetBrains\Toolbox\apps`, and an install folder chosen by hand. A setting to point at a tool stays out of scope (#231) unless this lookup is not enough.
- IntelliJ IDEA's Maven runs with the person's `~/.m2/settings.xml`, not with settings chosen inside the IDE (a user settings file or local repository set in its Maven page); a project that needs those lists with them only from a terminal or a wrapper.

## Acceptance criteria

```gherkin
Feature: Find Maven and Gradle when they are not on PATH

  Scenario: Maven in MAVEN_HOME
    Given a Maven project with no mvnw.cmd, and no mvn on PATH
    And MAVEN_HOME holds bin\mvn.cmd
    When I open the project
    Then Grace lists its libraries with MAVEN_HOME's mvn.cmd

  Scenario: M2_HOME when MAVEN_HOME has no Maven
    Given a Maven project with no mvnw.cmd, and no mvn on PATH
    And MAVEN_HOME holds no bin\mvn.cmd, and M2_HOME does
    When I open the project
    Then Grace lists its libraries with M2_HOME's mvn.cmd

  Scenario: Gradle in GRADLE_HOME
    Given a Gradle project with no gradlew.bat, and no gradle on PATH
    And GRADLE_HOME holds bin\gradle.bat
    When I open the project
    Then Grace lists its libraries with GRADLE_HOME's gradle.bat

  Scenario: PATH comes first
    Given a Maven project with no mvnw.cmd
    And mvn.cmd on PATH, and MAVEN_HOME holds bin\mvn.cmd
    When I open the project
    Then Grace lists its libraries with the mvn.cmd on PATH

  Scenario: IntelliJ IDEA's Maven, the newest first
    Given a Maven project with no mvnw.cmd, no mvn on PATH, and no MAVEN_HOME or M2_HOME
    And IntelliJ IDEA build 243.21565.193 in %ProgramFiles%\JetBrains
    And IntelliJ IDEA build 243.21565.1000 in %LOCALAPPDATA%\Programs
    When I open the project
    Then Grace lists its libraries with the Maven of build 243.21565.1000

  Scenario: No Maven anywhere
    Given a Maven project with no mvnw.cmd, and no Maven on PATH, in MAVEN_HOME, in M2_HOME or in IntelliJ IDEA
    When I open the project
    Then the notice says "Grace could not list the project's libraries with Maven: the project has no mvnw.cmd, and mvn is not on PATH, in MAVEN_HOME, in M2_HOME or in IntelliJ IDEA. Tests that use them will fail. Install Maven, or add the Maven wrapper to the project."
    And the tree lists the project's tests
```

## Tests

No test reaches the machine's own tools: the Runner's `lookupEnv` option takes the variables the lookup reads (PATH, the three homes, `ProgramFiles`, `LOCALAPPDATA`); `test/runner.ts` passes none. e2e's `launch()` drops the homes and points `ProgramFiles` and `LOCALAPPDATA` at an empty folder, as it already drops PATH's tools.

## Sources

- JetBrains, [Toolbox App FAQ](https://www.jetbrains.com/help/toolbox-app/frequently-asked-questions.html), "Where are the IDEs located on my hard drive?": `%LocalAppData%\Programs` on Windows.
- JetBrains, [Toolbox App 2.0 overhauls installations and updates](https://blog.jetbrains.com/toolbox-app/2023/08/toolbox-app-2-0-overhauls-installations-and-updates/): from 2.0 the default tools folder on Windows is `%LocalAppData%\Programs` (`FOLDERID_UserProgramFiles`), and it no longer changes.
- JetBrains, [Toolbox App 2.1](https://blog.jetbrains.com/toolbox-app/2023/11/toolbox-app-2-1-is-now-available/): the tools install location can be set again in its settings; tools installed before stay where they were.
- JetBrains, [Install IntelliJ IDEA](https://www.jetbrains.com/help/idea/installation-guide.html), silent installation: `mode=user` (for the current user) by default, `mode=admin` for all users. The page names no default folder for either. Assumed from Windows' own folders and from the machine-wide installs seen (`C:\Program Files\JetBrains\<IDE>`): `%ProgramFiles%\JetBrains` for all users, `%LOCALAPPDATA%\Programs` for one.
- Checked on the development machine: IntelliJ IDEA 2026.1.2 from the Toolbox App in `%LOCALAPPDATA%\Programs\IntelliJ IDEA`, with `build.txt` `IU-261.24374.151` and `plugins\maven\lib\maven3\bin\mvn.cmd`.
