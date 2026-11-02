# Check: project libraries with a real Maven and Gradle (#64)

Date: 2026-11-02
Build: `feature/project-libraries` at `3e316fbf` (develop `5bfdc983` merged, bundled JRE 21.0.13+9, Karate 2.1.4), run from source through the Runner, the same code the app runs.
Machine: the development machine (Windows 11 Pro, Ryzen 7 5800X, 32 GB). Neither Maven nor Gradle is installed: Maven 3.9.16 and Gradle 9.8.2 were downloaded to a scratch folder, checked against their published SHA-512 and SHA-256, and given to the Runner as its PATH. Their downloads went to scratch folders too (`MAVEN_OPTS=-Dmaven.repo.local=…`, `GRADLE_USER_HOME=…`), already filled by an earlier try, so no listing below downloads anything.
Spec: [docs/specs/2026-11-02-project-libraries.md](../specs/2026-11-02-project-libraries.md), Checks.

**Result: a Maven and a Gradle project whose scenario opens a SQLite database through `java.sql.DriverManager` pass in Grace with nothing copied by hand.** A first open lists the libraries in about 3.5 s (Maven) and 7.7 s (Gradle), a later open uses the kept list in under 0.5 s, and no build JVM is left running. The check found that Gradle cannot list on the bundled JRE; the spec and the code changed (below).

## What was run

Two projects, each with `org.xerial:sqlite-jdbc` 3.54.1.0 and `io.karatelabs:karate-junit6` 2.1.4 as test dependencies, and one scenario that creates, fills and reads a SQLite table. Each was opened twice with the same data folder (the second open finds the kept list), and run after each open. Times are the Runner's open (listing, then the dry run) and its run of all tests. Peak memory is the largest total working set of every `java.exe`, sampled every 250 ms during the open. "Build JVMs left" counts the Maven or Gradle `java.exe` processes after the open.

| Project | Java for the build | Open | Peak Java during open | Build JVMs left | Run |
|---|---|---|---|---|---|
| Maven, first open | none: Grace lends its JRE | 3,443 to 3,555 ms | 261 to 292 MB | 0 | 1,195 to 1,252 ms, passed |
| Maven, again | (no listing) | 471 to 474 ms | 59 to 61 MB | 0 | 1,190 to 1,246 ms, passed |
| Gradle, first open | IntelliJ's JBR 25 (a JDK) as `JAVA_HOME` | 7,666 ms | 609 MB | 0 | 1,184 ms, passed |
| Gradle, again | (no listing) | 498 ms | 58 MB | 0 | 1,216 ms, passed |

The same Maven project run without its libraries takes 956 to 964 ms and fails at `No suitable driver found`; the 0.3 s more with them is mostly SQLite's own native library being loaded and the database written.

## What it changed

- On the bundled JRE, Gradle 9.8.2 fails to resolve a Java project's `testRuntimeClasspath`: `Toolchain installation '…\vendor\jre' does not provide the required capabilities: [JAVA_COMPILER]`. Gradle needs a JDK, which anyone who builds with Gradle has. Grace now lends its JRE to Maven only (R2.4). With no Java at all, the notice reads: `Grace could not list the project's libraries with Gradle: JAVA_HOME is not set and no 'java' command could be found in your PATH. Tests that use them will fail.`
- Gradle's first `* What went wrong:` line names only the failed task; the reason is now its deepest `>` cause, and a reason's own full stop no longer doubles the notice's (R6.3).

## Needs the owner

Their work suite, opened and run in a packaged build: it is the case this feature is for, with their Maven or Gradle, their JDK and their company repository.
