---
status: accepted
date: 2026-11-02
decision-makers: Tusquito
---

# List a project's libraries with its own build tool

## Context and Problem Statement

A Maven or Gradle Karate project's Java code often calls outside libraries: a SQL driver, a company client. Grace puts the project's folders on the classpath, but not the libraries, so those tests fail with a class-not-found error ([#64](https://github.com/hpagniez/grace/issues/64)). Following the Karate CLI first ([ADR 0026](0026-follow-the-karate-cli-first.md)), Grace adds the jars of the CLI's extension folders, `~/.karate/ext/` and `.karate/ext/`; but the CLI has no way to get a build's libraries, so the person would copy them there by hand. How does Grace get them on its own, without cost while idle ([ADR 0023](0023-run-lightly-as-a-release-requirement.md))?

## Considered Options

1. Run the project's own Maven or Gradle (its wrapper, else the one on PATH) to list the test runtime classpath, and keep the list until a build file changes.
2. Run the build to copy its jars into `.karate/ext/`.
3. Resolve the jars in Grace: read `pom.xml` and the Gradle build files, download from the repositories.

## Decision Outcome

Chosen: option 1. It is the only one that gives the jars the build itself uses, on its own, for both Maven and Gradle, without writing into the project.

- Maven: `dependency:build-classpath`. Gradle: an init script shipped with Grace adds a task that writes `sourceSets.test.runtimeClasspath`; it runs with `--no-daemon`.
- The tool runs through `cmd.exe` (the wrappers are batch files), in the project folder, with the person's environment unchanged.
- The list is kept per project in Grace's data folder, keyed by the content of the build files, and made again only on open or Refresh when that key changes. Environment switches and runs never start the tool.
- Grace's Karate jar stays first on the classpath, so the project's own copy of Karate never replaces it. The CLI's extension folders come before the build's libraries.

### Consequences

- Good: exact versions, transitive libraries and the company repository, as `mvn test` or `gradlew test` resolves them. Tried with Maven 3.9.16 on a SQLite project: 30 jars, 16 s the first time, 3 s after.
- Good: no cost while idle and none on most opens; nothing stays running after a listing.
- Bad: Grace now starts a program the project provides (its wrapper). It is the program the person already runs to build the project, in the folder they chose to open.
- Bad: a listing costs a build JVM for seconds (Maven) to tens of seconds (Gradle without a daemon) each time a build file changes, and may use the network.
- Bad: it needs the wrapper or the tool on PATH, and a build that resolves with its plain command. Otherwise a notice says why, and the person can still put the jars in `.karate/ext/`.

## More Information

Option 2 writes about 30 jars, Karate's own among them, into a project Grace promises to leave clean. Option 3 means rebuilding Maven's resolution (parents, imported BOMs, profiles, exclusions, the company repository), and a Gradle build file is code only Gradle runs. Spec: [docs/specs/2026-11-02-project-libraries.md](../specs/2026-11-02-project-libraries.md).
