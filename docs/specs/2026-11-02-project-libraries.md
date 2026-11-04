# Project libraries

Date 2026-11-02. Status: draft. Part of [#64](https://github.com/hpagniez/grace/issues/64) (second slice). Follows [the first slice](2026-11-02-project-setup.md), whose Non-goals name this work.

## Intent

A Karate project's Java code often calls outside libraries: a SQL driver, a company client. Grace puts the project's folders on the classpath but no library, so a scenario that opens a database fails with `No suitable driver found`. The owner's own case is a work suite of this kind: one Maven or Gradle project, `src/test` under the opened folder, a SQL driver among its dependencies, run today with Maven or Gradle from a terminal and from the IDE.

Opening such a project in Grace should run it with its libraries, on its own, with nothing to copy or list by hand.

This is the shape Karate's own guide to [database testing](https://docs.karatelabs.io/advanced/database-testing/) teaches: a helper class in `src/test/java` that the build compiles, using plain JDBC through `DriverManager`, loaded in a feature with `Java.type`, and the JDBC drivers (H2, MySQL) as `test`-scoped Maven dependencies. The guide says nothing about compiling the helper or about the standalone jar: it assumes the project's own build.

The owner's rule, recorded in [ADR 0026](../decisions/0026-follow-the-karate-cli-first.md): Grace first does what the Karate CLI does, and builds its own way only where the CLI has none.
- The CLI has a way to add libraries: its launcher puts every jar of `~/.karate/ext/` and of the project's `.karate/ext/` on the classpath ([CLI.md, Classpath Construction](https://github.com/karatelabs/karate/blob/main/docs/CLI.md#classpath-construction)). Grace does the same (R1).
- The CLI has no way to get a Maven or Gradle project's libraries: the person copies them, and Karate's docs send Java teams to their build. Grace asks the project's own build for them (R2 to R6), recorded in [ADR 0027](../decisions/0027-list-project-libraries-with-its-own-build-tool.md).

## Who and what success looks like

A person who opens a Karate project whose tests need libraries: a Maven or Gradle project that declares them, or any project with jars in the CLI's extension folders.

Success:
- Every library the project's tests use at runtime is on the classpath: the build's, as the build resolves them (versions, transitive libraries, the company repository), and the extension folders' jars, as `karate run` adds them.
- It happens on its own when the project opens or is refreshed. Most opens cost nothing extra.
- When the build cannot list them, a notice says why.

## Goals, non-goals, anti-goals

**Goals:** the Requirements below.

**Non-goals** (later slices of #64 if someone needs them):
- Multi-module builds: picking a module when the opened folder has no `src/test`, and a module's tests that need a sibling module.
- A build that needs a profile or a `-D` option to resolve its test libraries (`mvn test -Pqa`): Grace runs the plain command.
- Re-listing when something other than the project's build files changes: a parent pom outside the folder, `~/.m2/settings.xml`, a Gradle init script of the person's own.
- Copying jars into `.karate/ext/` for the person, or writing anything into the project other than what its own build writes when the person clicks Build ([build from the notice](2026-11-04-build-from-notice.md)).
- The launcher's `--cp` flag: it is given per call, so there is nothing in the project to read.
- A setting to add or remove jars by hand, or a view listing them.
- Compiling the project's Java code on its own. Grace runs what the build already compiled (`target/test-classes`, first slice) and says when that is missing or older than the source (R8). Since [build from the notice](2026-11-04-build-from-notice.md) the notice has a Build button; Grace builds only when the person clicks it.

**Anti-goals:**
- A project that runs with `karate run` but not in Grace because of the classpath.
- A library's copy of a file replacing the project's own (`karate-config.js`, a feature), or the project's copy of Karate replacing Grace's.
- Any cost while idle, and any process left running after a listing (a Gradle daemon).

## Constraints

- Karate integration rules in `AGENTS.md`. Karate itself still runs without a shell.
- `mvnw.cmd`, `mvn.cmd`, `gradlew.bat` and `gradle.bat` are batch files. Node cannot spawn a batch file without `cmd.exe`, so the build tool runs through `cmd.exe /d /s /c`, every path quoted.
- The bundled Karate jar is a fat jar whose libraries keep their own package names (Netty, json-path, Thymeleaf, ...). A Maven or Gradle Karate project depends on Karate, so the build's list holds a second Karate and those libraries again. The JVM takes a class from the first classpath entry that has it, so Grace's jar must come first.
- Java expands a classpath entry `<folder>\*` to every `.jar` in the folder, not in its sub-folders; a missing folder is ignored.
- Gradle starts a daemon that stays up to three hours and takes 0.5 to 1 GB. `--no-daemon` runs the build in a process that ends with it.
- A Windows command line holds at most 32,767 characters. The spike's 30 jars from a Maven repository were 6,467 characters.
- Neither `mvn` nor `gradle` is on the development machine's PATH, but CI's Windows runners have both, and existing unit tests open a `maven` fixture: no test may reach a real tool, so the tests control what Grace finds. The real tools are checked by hand (Checks).

### What exists today

- `detectLayout(root)` in `src/main/karate/command.ts` gives the layout (`maven`, `gradle`, `standalone`), the classpath (source folders then compiled folders, or the project folder) and the feature roots. Every dry run and run calls it.
- `buildArgs` puts `karate.jar`, the debug listener jar in a debug run, then the classpath after `-cp`.
- `rediscover` runs the dry run on open, on Refresh and on an environment switch; a re-run that changes the environment goes through it too. Cancel kills the running JVM.
- `<dataDir>/projects/<sha1 of the root, 12 hex>/` holds a project's runs and its dry-run folder.
- The renderer says `Finding tests…` in the tree (no tree yet), the progress area and the banner while discovering.
- The open notice joins the skipped-properties text and the `karate-pom.json` text in one notice.

## Approaches considered

Tried on a Maven project with `org.xerial:sqlite-jdbc` 3.54.1.0 and a scenario that writes and reads a SQLite file through `java.sql.DriverManager`, with the bundled Karate 2.1.4 and Maven 3.9.16 on the bundled Java: Grace's classpath today fails with `No suitable driver found for jdbc:sqlite:test.db`. The driver jar in `.karate/ext/` passes. Maven's list (`dependency:build-classpath`: 16 s with the downloads the first time, 3 s after, 30 jars) after `karate.jar` passes.

- **A. Only the CLI's extension folders.** What the CLI does, nothing to run, but the person copies every jar the build already knows. Kept as R1, not enough alone: the owner wants it done on its own.
- **B. A + ask the project's Maven or Gradle for its test classpath (chosen).** Exact and on its own; a build JVM for seconds, only when a build file changed.
- **C. A + copy the build's jars into `.karate/ext/`.** Everything is then the CLI's way, but Grace writes about 30 jars into the project, which the README promises it never does, Karate's own among them. Rejected.
- **D. Resolve the jars in Grace** (read the build files, download from Maven Central). Maven's resolution (parents, imported BOMs, profiles, exclusions, the company repository) would have to be rebuilt, and a Gradle build file is code only Gradle runs. Rejected.

## Requirements

### R1. The CLI's extension folders

1. Every dry run, run and debug run of any project, Maven, Gradle or standalone, adds `<home>\.karate\ext\*` and then `<project folder>\.karate\ext\*`, each only if the folder exists. `<home>` is the person's home folder (`%USERPROFILE%`), the CLI's `~`.
2. They are not feature roots, and environments are not read from them.

### R2. Which tool

1. Maven projects (layout `maven`): `mvnw.cmd` in the project folder if it exists, else `mvn.cmd` or `mvn.bat` found on PATH, then in the places of [find-build-tools R1](2026-11-03-find-build-tools.md).
2. Gradle projects (layout `gradle`): `gradlew.bat` in the project folder if it exists, else `gradle.bat` found on PATH, then in the places of [find-build-tools R1](2026-11-03-find-build-tools.md). A quoted PATH entry is read as the folder inside its quotes, as `cmd.exe` reads it.
3. Standalone projects: no tool runs.
4. The tool runs in the project folder with the person's environment, unchanged, so their proxy, `MAVEN_OPTS` and company repository settings apply. Only for Maven, and only when `JAVA_HOME` is not set and no `java` is on PATH, does Grace set `JAVA_HOME` to its bundled Java. Gradle gets none: it needs a JDK to resolve a Java project's classpath (found by the Checks: on the bundled Java it fails with `does not provide the required capabilities: [JAVA_COMPILER]`), and without one its own launcher says `JAVA_HOME is not set`.

### R3. The commands

1. Maven: `<tool> -q -B dependency:build-classpath -Dmdep.outputFile=<file>`. The plugin version is the project's, else Maven's default. With no `includeScope` the goal lists every scope, which is the test classpath.
2. Gradle: `<tool> -q --no-daemon --console=plain --no-configuration-cache --init-script <script> graceClasspath`. The init script, shipped with Grace, adds a `graceClasspath` task to the root project that writes the files of `sourceSets.test.runtimeClasspath`, joined by `;`, to the file named by the `GRACE_CLASSPATH_FILE` environment variable.
3. `<file>` is `<dataDir>/projects/<hash>/libraries.txt`, deleted before the tool starts and after Grace reads it.
4. The list is the file's entries, in order, that exist, less any already on the classpath (Gradle lists the build's own output folders, which the first slice adds).

### R4. When Grace lists

1. On open and on Refresh, before the dry run, for Maven and Gradle projects.
2. Grace keeps the last list in `<dataDir>/projects/<hash>/libraries.json`: the tool (`maven` or `gradle`), a key, and the list. The key is the SHA-256 of each of these files' name and content, an absent file counted as absent: Maven `pom.xml`, `.mvn/maven.config`, `.mvn/extensions.xml`, `.mvn/wrapper/maven-wrapper.properties`; Gradle `build.gradle`, `build.gradle.kts`, `settings.gradle`, `settings.gradle.kts`, `gradle.properties`, `gradle/libs.versions.toml`, `gradle/wrapper/gradle-wrapper.properties`.
3. Grace runs the tool only when there is no kept list, the tool or the key differ, or an entry of the list no longer exists. Otherwise it uses the kept list and starts no process.
4. An environment switch, a run, a re-run (also one that switches the environment) and a debug run never start the tool: they use the kept list as it is, even if a build file changed since. Refresh picks the change up, as it does for new feature files.
5. While the tool runs, where Grace says `Finding tests…` it says `Listing the project's libraries with Maven…` (or `Gradle`).
6. Cancel stops the tool and its child processes (`taskkill /T /F`), keeps the list as it was, and shows the last tree, as it does for a dry run.

### R5. The classpath

1. Karate's classpath is: `karate.jar`, the debug listener jar in a debug run, the project's folders (source and compiled, or the project folder), the home extension jars, the project extension jars, then the build's libraries in the build's order.
2. The build's libraries are not feature roots and environments are not read from them.

### R6. When the tool fails

1. A failure is: no tool found, the tool exits with a code other than 0, or it exits with 0 and wrote no file. An empty file is an empty list, not a failure.
2. The kept list is left as it was, so the next open or Refresh tries again and, failing again, says so again. Runs use the kept list if there is one, else none.
3. A notice says `Grace could not list the project's libraries with <Maven|Gradle>: <reason>.` then `Grace uses the libraries it listed before.` if it kept a list, else `Tests that use them will fail.` With no tool found, the reason names where Grace looked and the notice ends with the fix ([find-build-tools R2](2026-11-03-find-build-tools.md)). Otherwise the reason is the first line of the tool's output starting with `[ERROR]`, `[FATAL]` or `ERROR:` (every such prefix removed; when it ends with `:`, as Maven's `Some problems were encountered while processing the POMs:` does, the next such line), else the deepest `>` cause under Gradle's `* What went wrong:` (its first line names only the task), else the last non-empty line; without its own full stop, cut to 300 characters.
4. On open this notice joins the open notice, after the skipped-properties and `karate-pom.json` texts. On Refresh it shows alone.
5. The dry run and the tree go ahead either way.

### R7. Docs

1. The README's project line says that a Maven or Gradle project's libraries come from its build, and that jars in `.karate/ext/` or `~/.karate/ext/` are on the classpath as with the Karate CLI. Its roadmap line keeps only multi-module projects.
2. `AGENTS.md` states the owner's rule under the Karate integration rules, pointing at ADR 0026.

### R8. Java code not built

Added after the Checks: a helper never compiled fails with Karate's `Java.type: class not found: …`, and one edited since its last build with `… is not a function`, neither of which says the project needs building. The owner chose a notice.

1. On open and on Refresh, for a Maven or Gradle project whose `src/test/java` holds `.java` files (in any sub-folder) other than JUnit runners, Grace compares file times with the build's compiled test folder: Maven `target/test-classes`, Gradle `build/classes/java/test`. A JUnit runner is a `.java` file that imports JUnit (`org.junit`) or Karate's JUnit module (`com.intuit.karate.junit…`, `io.karatelabs.junit…`): Grace runs the features itself and never loads it, so it needs no build ([#236](https://github.com/hpagniez/grace/issues/236)).
2. When that folder holds no `.class` file (it is missing, or empty): `The project's Java code in src/test/java is not built: build it (mvn test-compile) before running.`
3. When a `.java` file other than a runner is newer than the newest `.class` file, and than the start of Grace's last successful Build ([build from the notice](2026-11-04-build-from-notice.md) R4.3): `The project's Java code in src/test/java changed since its last build: build it again (mvn test-compile) before running.`
4. Gradle projects read `gradle testClasses` where Maven projects read `mvn test-compile`. Standalone projects get neither.
5. On open the text joins the open notice, last. On Refresh it joins the libraries' notice, after it, or shows alone. Environment switches and runs never check.
6. On open and Refresh, Grace starts no process and changes no file. The notice's Build button runs the build ([build from the notice](2026-11-04-build-from-notice.md)). A `git checkout` that touches file times can show the second text until the next build.

## Cost

- **Idle:** zero. Nothing is watched or polled; the list is checked only on open and Refresh.
- **Before each dry run and run:** two folder checks for the extension folders and one small JSON read, well under a millisecond.
- **Open or Refresh, build files unchanged:** hash up to seven small files and check that each listed jar exists (about 30 to 100 file checks): a few milliseconds, no process.
- **Build files changed (first open, a dependency added):** one build JVM for the time it takes, then nothing left running. Measured: Maven 16 s the first time with downloads, 3 s after, for 30 jars. Expected for Gradle with `--no-daemon`: about 10 to 30 s and 300 to 700 MB, more the first time its wrapper downloads Gradle (Checks). The dry run waits for it.
- **Runs:** a longer `-cp`. The JVM opens a jar only when it looks up a class or resource there; Karate's lookups at the classpath root may open each jar once (Checks).
- **Open or Refresh, Java code check (R8):** one asynchronous walk of `src/test/java` and of the compiled test folder, reading each `.java` file and the time of each `.class` file: a few milliseconds for a few hundred files, no process.
- **Disk:** `libraries.json`, a few KB per project.
- **Size:** the Gradle init script, under 1 KB. No dependency.
- **Bench:** the bench project is standalone with no extension folder; its numbers do not move.

## Design decisions already made

The owner's: follow the Karate CLI first; the CLI's extension folders for every project; on top, a Maven or Gradle project's libraries listed by its own build, on its own on open and Refresh; multi-module builds later.

Mine, open to correction:
- The extension jars come after the project's folders, not right after `karate.jar` as in the launcher: the CLI puts no project folder on the classpath, and this keeps the project's own files first. They come before the build's libraries, so a jar the person puts there by hand wins over the build's.
- Only open and Refresh list. An environment switch re-discovers but never lists: with a failing build (off VPN, say) each switch would otherwise cost a build JVM and repeat the notice.
- Runs never start the build tool: a run would otherwise wait seconds, and a failing build would delay every run.
- A failure is not remembered: open and Refresh are the person's own acts, so each retries and, failing, says so.
- A failed listing keeps the last good list: after a pom edit, the old list is closer than none.
- No timeout: a first Gradle download can take minutes; Cancel stops it.
- Gradle runs without a daemon, slower each time it lists, so nothing stays in memory.
- The person's environment is passed unchanged to their build tool (unlike Karate's, which drops `JAVA_TOOL_OPTIONS` and the like): it is their build, run as their terminal runs it.
- The project's own Karate jars stay in the build's list, behind Grace's: removing them by name could remove a library of the person's. The spike ran this way.
- The classpath stays on the command line. A Karate project's own Karate libraries are about 30 jars, about 6,500 characters; more than about 150 libraries could pass the Windows limit, and Grace then shows the spawn error as a notice. Shortcut: moving `-cp` into the Java argument file Grace already writes lifts the limit if someone reaches it.

## Acceptance criteria

```gherkin
Feature: Project libraries

  Scenario: A jar in the project's .karate/ext is on the classpath
    Given a standalone project whose .karate/ext holds a jar with data.json
    And a feature that reads "classpath:data.json"
    When I open it
    And I run all tests
    Then the feature passes

  Scenario: A Maven project's libraries are on the classpath
    Given a Maven project whose mvnw.cmd lists a jar with data.json
    And a feature that reads "classpath:data.json"
    When I open it
    And I run all tests
    Then the feature passes

  Scenario: A Gradle project's libraries are on the classpath
    Given a Gradle project whose gradlew.bat lists a jar with data.json
    When I open it
    Then that jar is on Karate's classpath
    And gradlew.bat was called with --no-daemon

  Scenario: The order of the classpath
    Given a Maven project with src/test/java, target/test-classes, a home .karate/ext, a project .karate/ext and a listed library
    Then Karate's classpath is karate.jar, src/test/java, target/test-classes, the home .karate/ext jars, the project .karate/ext jars, then the library

  Scenario: The list is kept until a build file changes
    Given the Maven project was opened once
    When I refresh
    Then mvnw.cmd does not run
    When pom.xml changes
    And I refresh
    Then mvnw.cmd runs once

  Scenario: Switching the environment and running do not start the tool
    Given the Maven project is open
    And pom.xml changed since
    When I switch the environment
    And I run all tests
    Then mvnw.cmd does not run

  Scenario: No tool
    Given a Maven project with no mvnw.cmd, and no Maven on PATH, in MAVEN_HOME, in M2_HOME or in IntelliJ IDEA
    When I open it
    Then the notice says "Grace could not list the project's libraries with Maven: the project has no mvnw.cmd, and mvn is not on PATH, in MAVEN_HOME, in M2_HOME or in IntelliJ IDEA. Tests that use them will fail. Install Maven, or add the Maven wrapper to the project."
    And the tree lists the project's tests

  Scenario: A failed listing keeps the last list
    Given the Maven project's libraries were listed once
    And pom.xml changed, and mvnw.cmd now fails with "[ERROR] Could not resolve dependencies"
    When I refresh
    Then the notice says "Grace could not list the project's libraries with Maven: Could not resolve dependencies. Grace uses the libraries it listed before."
    And the listed jar is still on Karate's classpath

  Scenario: Grace says what it is doing
    Given a Maven project whose mvnw.cmd takes a while
    When I open it
    Then the tree says "Listing the project's libraries with Maven…"

  Scenario: Cancel while listing
    Given a Maven project whose mvnw.cmd takes a while
    When I open it
    And I cancel while Grace lists the libraries
    Then mvnw.cmd and its child processes have stopped

  Scenario: The project's Java code is not built
    Given a Maven project with src/test/java/com/example/qa/DbHelper.java and no target/test-classes
    When I open it
    Then the notice says "The project's Java code in src/test/java is not built: build it (mvn test-compile) before running."
    When I build it with mvn test-compile
    And I refresh
    Then no notice shows
    When DbHelper.java changes
    And I refresh
    Then the notice says "The project's Java code in src/test/java changed since its last build: build it again (mvn test-compile) before running."

  Scenario: A JUnit runner needs no build
    Given a Maven project whose only Java code is src/test/java/examples/ExamplesTest.java, a Karate JUnit runner, and no target/test-classes
    When I open it
    Then no notice shows

  Scenario: Standalone projects run no tool
    Given a standalone project
    When I open it
    Then no build tool runs
```

## Checks

Before the PR is marked ready, by hand on the development machine, with Maven 3.9 and Gradle 9 downloaded to a scratch folder (not installed), each with its downloads in a scratch repository:
- A Maven project and a Gradle project with `sqlite-jdbc` and the SQLite scenario: open in a packaged build, run, it passes. Time and peak memory of the listing, and the run's start with and without the libraries.
- No Java process left after the Gradle listing.

Needs the owner: their work suite, opened and run in a packaged build.
