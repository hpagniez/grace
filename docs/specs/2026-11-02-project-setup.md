# Project setup

Date 2026-11-02. Status: approved. Part of [#64](https://github.com/hpagniez/grace/issues/64) (first slice).

## Intent

Grace is for people who already have a Karate project: a Maven or Gradle build, a folder run with the standalone jar, or a project run with the `karate` CLI and its `karate-pom.json`. Opening it should run the tests the way the project's own setup says, and never let a file change a run without saying so. Today two things break that, found by running the bundled Karate 2.1.4 on scratch projects:

- **`karate-pom.json` changes Grace's runs in silence.** Karate loads it from its working directory, which is the project folder. Grace's flags override only its `paths`, `output` and `workingDir`. Its `env` runs every test in that environment while the environment picker shows none; its `tags` and `scenarioName` cut the tree; its `dryRun: true` makes every run "pass" without running a step; its `configDir` moves `karate-config.js`.
- **Compiled Java code is not on the classpath.** For Maven and Gradle Grace puts only the source folders `src/test/java` and `src/test/resources` on the classpath, so a helper class (`Java.type('com.acme.Util')`) or a resource the build generates is not found.

`karate doctor --json`, named in #64, is not in the jar: the bundled jar has only `run`, `mock` and `clean` (`doctor` is read as a path). It belongs to the separate `karate` launcher and reports on the launcher's own Java and jar. Grace brings its own, so it has nothing to ask doctor.

## Who and what success looks like

A person opening an existing Karate project in Grace, often for the first time.

Success:
- What Grace shows (tree, environment, filter, thread count) is what Karate runs.
- A `karate-CLI` project's tests and `karate-config.js` are found where its `karate-pom.json` says.
- Helper classes and resources the build compiled are found.

## Goals, non-goals, anti-goals

**Goals:** the Requirements below.

**Non-goals** (later slices of #64 if someone needs them):
- Dependency jars of a Maven or Gradle project, and jars beside a standalone project: a classpath setting per project, or running the project's own `mvnw` or `gradlew` to list them.
- Multi-module projects: picking a module when the opened folder has no `src/test`.
- `env`, `tags`, `threads` or `scenarioName` of the pom as starting values for Grace's controls.
- Writing `karate-pom.json`.
- The launcher's `.karate/karate-cli.json` (`jvm_opts`, `karate_version`): Grace bundles its Java and Karate.
- A "Karate CLI" layout chip: the toolbar is #81's.

**Anti-goals:**
- A run whose settings come from a file Grace does not show.
- A stale compiled copy of `karate-config.js` or of a feature winning over the source.
- Any cost while idle.

## Constraints

- Karate integration rules in `AGENTS.md`. `--no-pom` and `-g, --configdir` are flags of `run` in 2.1.4 (its `--help`).
- Checked with the bundled jar: `-g` takes a folder relative to the working directory, an absolute folder, or `classpath:<folder>`; Karate then looks for `karate-config.js` and its overlays there and in the working directory, no longer at the classpath root. Pom `paths` take files or folders relative to the working directory; a `classpath:` entry fails with exit 3.

### What exists today

- `detectLayout(root)` in `src/main/karate/command.ts`: `maven` if `pom.xml`, else `gradle` if `build.gradle[.kts]`, else `standalone`. Classpath and feature roots: whichever of `src/test/java` and `src/test/resources` exist, or the project folder for standalone. It runs before every dry run and run.
- `buildArgs` passes no `--no-pom` and no `-g`.
- Environments come from `karate-config-<env>.js` in the project folder and the classpath roots (`overlayEnvs`), and from `@env=` tags under the feature roots (`featureTagEnvs`).
- A notice on open says when saved properties were skipped.

## Approaches considered

For the pom:
- **A. Let Karate read it, and show its values in Grace's controls.** Eleven keys to mirror; `dryRun`, `scenarioName`, `output` and `logging` have no control, and each control would have two sources. Rejected.
- **B. `--no-pom`, and Grace reads only what describes the project (chosen).** `paths` (where the tests are) and `configDir` (where the config is) are the project's shape, the same in every run. The rest are run choices, which Grace's own controls make; a notice names the ones the file sets.
- **C. `--no-pom` and nothing else.** A project whose tests or config sit where only its pom says would show no tests or run without config. Rejected.

For compiled code:
- **A. Add the build's compiled-output folders when they exist (chosen).** No tool, no setting; covers helper classes and resources that need no outside library.
- **B. A classpath setting per project.** Needed for dependency jars; later.
- **C. Run `mvn`/`gradle` to list the classpath.** Needs the tool or its wrapper, a JVM at open and network the first time; later if B is not enough.

## Requirements

### R1. `karate-pom.json`

1. Every Karate command Grace starts, dry run and run, passes `--no-pom` right after `--no-color`.
2. Grace reads `<project folder>/karate-pom.json` before each dry run and run. It uses only `paths` and `configDir`.
3. `paths`: each entry is resolved against the project folder (an absolute entry stays as is). The entries that exist and lie inside the project folder are the feature roots: the dry run lists them, Run all runs them, and `@env=` tags are read under them. A missing entry, one outside the project folder, or a `classpath:` one is left out. An absent or empty `paths` keeps the default feature roots. A `paths` whose entries are all left out gives an empty tree, as Karate would.
4. `configDir`: passed as `-g <value>`, as written, to every dry run and run. Environments then come from `karate-config-<env>.js` in that folder and in the project folder, not in the classpath roots. The folder is the value resolved against the project folder, or, for `classpath:<folder>`, `<folder>` under each classpath root.
5. Opening the project shows one warning notice when the file sets keys Grace does not use, or has entries in `paths` that were left out: `Grace does not use these karate-pom.json settings: env, dryRun.` and `These karate-pom.json paths are not folders or files in the project: ../shared.` (each sentence only when it applies, keys in the file's order).
6. A file that is not valid JSON, not an object, whose `paths` is not a list of strings, or whose `configDir` is not a string: Grace uses nothing from it, as if it were absent, and the notice on open says `karate-pom.json could not be read: <reason>. Grace uses the default folders.`
7. The notice shows when the project opens (Open folder, a recent folder, the last project at start), not on Refresh, an environment switch or a run. When the skipped-properties notice also applies, both show in one notice, that one first.

### R2. Compiled code on the classpath

1. After the source folders, each of these folders that exists is added to the classpath, in this order: Maven `target/test-classes`, `target/classes`; Gradle `build/classes/java/test`, `build/resources/test`, `build/classes/java/main`, `build/resources/main`.
2. They are not feature roots and environments are not read from them: a stale copy of a feature or of `karate-config-<env>.js` must not show.
3. Standalone projects are unchanged: the project folder is the classpath.

### R3. #64's Karate surface

1. The issue's `Karate surface:` line drops `karate doctor --json` and says why (Intent).

## Cost

- Memory: none to speak of: one small JSON file parsed and dropped per dry run and run.
- CPU: one file read and up to four more folder checks before each dry run and run, well under a millisecond. Java loads nothing from the added folders until a test asks for a class or resource.
- Start time: the same; the read happens where `detectLayout` already checks folders.
- Size: none. No dependency, no file.
- Idle stays at zero: nothing is watched or polled; the file is read only when Karate is about to start.

## Design decisions already made

The person's: this first slice (pom read by Grace, compiled folders, doctor dropped); dependency jars and multi-module later.

Mine, open to correction:
- Only `paths` and `configDir` are read; every other key is named in the notice, not used.
- The notice shows on open only, so a project whose pom serves the CLI in CI does not repeat it on every Refresh.
- Paths outside the project folder are left out: the tree is built from paths relative to it.
- The Gradle list covers Java and resources, not Kotlin or Groovy output folders.

## Acceptance criteria

```gherkin
Feature: Project setup

  Scenario: The pom's run settings do not change a run
    Given a project whose karate-config-qa.js sets a variable its karate-config.js sets otherwise
    And whose karate-pom.json holds "env": "qa" and "dryRun": true
    When I open it
    Then a notice says "Grace does not use these karate-pom.json settings: env, dryRun."
    And the environment picker shows no environment
    When I run all tests
    Then a scenario that matches the variable against karate-config.js's value passes

  Scenario: The pom's paths choose the tests
    Given a project with tests/a/a.feature and tests/b/b.feature, whose karate-pom.json holds "paths": ["tests/a", "missing"]
    When I open it
    Then the tree shows a.feature and not b.feature
    And the notice says "These karate-pom.json paths are not folders or files in the project: missing."

  Scenario: The pom's configDir gives the config and the environments
    Given a project whose karate-config.js and karate-config-qa.js are in conf/, and whose karate-pom.json holds "configDir": "conf"
    When I open it
    Then the environment picker offers "qa"
    And a run reads the variables of conf/karate-config.js

  Scenario: A pom Grace cannot read
    Given a project whose karate-pom.json is not valid JSON
    When I open it
    Then the notice says "karate-pom.json could not be read:" and "Grace uses the default folders."
    And the tree lists the project's tests

  Scenario: The notice shows only on open
    Given a project whose karate-pom.json holds "env": "qa" is open
    When I refresh
    Then no notice shows

  Scenario: Maven's compiled output is on the classpath
    Given a Maven project with target/test-classes/data/built.json
    And a feature in src/test/java that reads "classpath:data/built.json"
    When I run it
    Then it passes

  Scenario: Compiled copies stay out of the tree and the environments
    Given a Maven project with target/test-classes/old.feature and target/test-classes/karate-config-stale.js
    When I open it
    Then the tree has no old.feature
    And the environment picker does not offer "stale"

  Scenario: Gradle's compiled output is on the classpath
    Given a Gradle project with build/classes/java/test and build/resources/test
    Then the classpath is src/test/java, src/test/resources, build/classes/java/test, build/resources/test, in that order
```
