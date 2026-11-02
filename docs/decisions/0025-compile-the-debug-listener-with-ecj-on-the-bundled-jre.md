---
status: accepted
date: 2026-11-02
decision-makers: Tusquito
---

# Compile the debug listener with ECJ on the bundled JRE

## Context and Problem Statement

The step debugger ([#65](https://github.com/hpagniez/grace/issues/65)) needs our own Java class: a Karate `RunListener` passed with `--listener`, which pauses steps and reads variables ([spike](../research/2026-11-02-step-debugger/README.md)). The repo has no Java build. The bundled Temurin JRE ([ADR 0006](0006-bundle-jre-and-karate-jar.md)) has no `jdk.compiler` module, so it has no `javac` and cannot run `java Foo.java`. How do we turn the listener's source into something the app ships, on a developer's machine and on CI, without slowing the app or the build?

## Considered Options

1. ECJ, the Eclipse Java compiler: one jar from Maven Central (`org.eclipse.jdt:ecj`, EPL-2.0), run on the bundled JRE.
2. A Temurin 21 JDK, downloaded beside the JRE for its `javac`.
3. Commit the compiled class.
4. Maven or Gradle with a wrapper.

## Decision Outcome

Chosen: option 1. It is the smallest download that compiles from reviewed source on every machine that already runs `npm run vendor`.

- `scripts/vendor.json` pins `ecj` (version, URL, SHA-256) like the JRE and the Karate jar. `npm run vendor` downloads it to `vendor/ecj.jar`, compiles `listener/` against `vendor/karate.jar` with `vendor/jre`, and zips the classes into `vendor/listener.jar` with Windows' `tar.exe`. It recompiles on every run (under a second), so a changed source or a new Karate jar is always picked up.
- A Karate bump that breaks the listener's API fails `npm run vendor`, so the `vendor-updates` PR fails before anyone merges it.
- `listener.jar` ships in `resources/` beside `karate.jar`. `ecj.jar` never ships.
- Only a debug run puts `listener.jar` on the classpath and passes `--listener`. A normal run is unchanged.
- `bump-vendor.mjs` and Dependabot do not move the ECJ pin: it is a build tool with no runtime exposure, bumped by hand when a newer Java needs it.

### Consequences

- Good: ECJ is 3.4 MB, against about 190 MB for a JDK, and compiles the listener in 0.9 s. The CI `vendor/` caches keep it.
- Good: the shipped cost is a few KB, loaded only when debugging; start time, idle cost and normal runs do not change ([ADR 0023](0023-run-lightly-as-a-release-requirement.md)).
- Good: the listener's source is reviewed like any other code; no binary is committed.
- Bad: ECJ is a second Java compiler to trust. It is the one Eclipse IDE uses, and the listener is one small file using only `java.base` and Karate's public listener interface.
- Bad: one more pin to bump by hand.

## More Information

Option 2 compiles the same way for 55 times the download on every cache miss. Option 3 ships a binary nobody can review and must be rebuilt by hand for every Karate bump. Option 4 adds a build tool and its own downloads for a single file.
