---
status: accepted
date: 2026-10-24
decision-makers: Tusquito
---

# Bundle a Java runtime and the Karate jar, and run projects with their folders on the classpath

## Context and Problem Statement

Karate needs Java 21+ and the Karate jar. Users may have no Java, or the wrong version. Maven and Gradle projects also load files with `classpath:`, which the bare `java -jar karate.jar run` does not resolve for `src/test/resources`. How does karate-kid get Java and Karate, and how does it run a project?

## Considered Options

1. Bundle a Temurin 21 JRE and a pinned `karate-<ver>.jar` inside the app.
2. Use the user's installed Java 21 and download the jar on first launch.
3. Run projects through their own build tool (`mvn test`, `gradle test`).

## Decision Outcome

Chosen: option 1, Windows x64 only for the MVP.

- Ship Temurin JRE `21.0.12.1+1` and `karate-2.1.4.jar` in the app's `resources/` (Forge `extraResource`), with their licence files (GPLv2 + Classpath Exception, MIT).
- Start Karate's main class with the project folders on the classpath, not with `-jar`:
  `java -cp "karate.jar;src/test/java;src/test/resources" io.karatelabs.Main run ...`
  Checked on 2026-10-24 against a Maven-layout project: `karate-config.js` in `src/test/java`, `read('classpath:data/user.json')` from `src/test/resources` and `call read('classpath:common/helper.feature')` all resolved, exit 0. With `-jar` and `-w .` the config was not found; with `-w src/test/java` the resources were not found.

Option 2 needs a setup screen and support for many Java versions. Option 3 needs Maven or Gradle installed, is slow to start, and moves the output under the build tool's control. It stays the likely path for projects with custom Java classes; extending `-cp` with `target/test-classes` and the dependency jars is the cheaper first try.

### Consequences

- Good: zero setup for users; every user runs the same Java and Karate versions.
- Good: the same `-cp` mechanism extends to custom Java classes later.
- Bad: about 57 MB more per platform; each new OS needs its own JRE.
- Bad: upgrading Karate means a new app release.
