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

## Karate 1.x projects and a stand-in for the work suite, in the built app

Added the same day, after the review asked what a Karate 1.x dependency tree behind Grace's `karate.jar` breaks. The [sample projects and their check](../research/2026-11-02-project-libraries/README.md) are committed so the check can be repeated before a release. Build: `feature/project-libraries` at `9bfa89e1`, packaged with `npm run package`; the check opens each sample in the built app with Playwright (as `npm run e2e` does), Maven 3.9.16, Gradle 9.8.2 and IntelliJ's JBR 25 as the JDK.

| Case | Listed jars | Open (lists, then finds the tests) | Run |
|---|---|---|---|
| `karate1-maven`: `com.intuit.karate:karate-junit5:1.4.1` | 87 | about 5.9 s | both scenarios passed |
| `karate15-maven`: `io.karatelabs:karate-junit5:1.5.2` | 93 | about 5.8 s | both passed |
| `karate1-gradle`: 1.4.1 with Gradle | | 8.0 to 13.1 s | both passed |
| `work-suite` | | about 5.9 s | passed |
| `work-suite` without `.mvn/maven.config` | | about 5.8 s | failed, as it must (below) |

The open times are rounded up by the check's polling; the listing alone is in the first table.

**Karate 1.x: nothing broke.**
- **What sits behind Grace's jar:** the 1.x trees put a second Netty (4.1.96 or 4.2.6), Graal JS (22.3 or 24.0), Armeria, Jackson, Apache HttpClient 4, logback (1.4 or 1.5) and slf4j behind Grace's `karate.jar`.
- **What ran:** with them on the classpath, the SQLite scenario passed, and so did a `karate-config.js` that uses `Java.type`. A local `karate.start` mock answered an HTTP call through Karate 2's own server and client.
- **What the console showed:** no `SLF4J`, multiple-binding, `ClassNotFound`, `NoSuchMethod` or `LinkageError` line. The check fails on any of them. The same runs at the command line, outside the app, showed no warning either.
- **Two Karate 1 to 2 differences seen** (Karate's, not the classpath's): Karate 2's JavaScript rejects a regex literal (`replace(/\\/g, '/')`) in `karate-config.js`, and the object `karate.start` returns has no `stop()`. A 1.x suite that uses either fails in Grace as it would with the Karate 2 CLI.

**The work-suite sample proves:**
- **The full chain:** a Maven project on Karate 1.5.2 opens and passes. Its feature calls a helper class the build compiled. The helper calls a company library that comes only from the project's own repository (declared in the pom), and that library opens the database through the SQLite driver.
- **Options in `.mvn/maven.config` reach Grace:** the driver comes from a `work` profile that `.mvn/maven.config` switches on. Maven reads that file for every command, Grace's listing included.
- **Command-line options don't:** without `.mvn/maven.config`, as if the team typed `mvn test -Pwork`, the driver is not listed and the scenario fails. A suite that needs a profile or `-D` option for its test libraries must put it in `.mvn/maven.config` (or make the profile active by default) for Grace to get them.

**What only the owner's real suite can prove:**
- **Their company repository:** a Nexus or Artifactory reached through `~/.m2/settings.xml`, with its mirror, credentials, proxy and VPN. The sample's repository is a folder.
- **Their Maven, Gradle and JDK versions,** and whether their build lists its test classpath without the network or a step the sample lacks.
- **Their own libraries and Karate version** working behind Grace's Karate 2.1.4.
- **How they run it:** the command they use and any option only that command carries.
- **Their run in the packaged `Grace.exe`,** started from Explorer with their own PATH and `JAVA_HOME`.

## Karate's database testing guide, followed

Added the same day: the samples above call `java.sql.DriverManager` from the scenario, but Karate's guide to [database testing](https://docs.karatelabs.io/advanced/database-testing/) uses a different shape:
- a helper class in `src/test/java` that the build compiles, using plain JDBC, with the connection settings as constants in the class;
- the helper loaded in a feature with `Java.type`;
- the drivers as `test`-scoped dependencies (H2 2.2.224, MySQL 8.2.0).

The `db-helper` samples follow that shape, with a helper written for them (`com.example.qa.DbHelper`). It covers parameterised queries through `PreparedStatement`, updates, and a transaction that rolls back. The database is H2 2.2.224 in memory, a `test`-scoped dependency. The samples come on Karate 2.1.4 (`io.karatelabs:karate-junit6`) and on Karate 1.5.2 (`io.karatelabs:karate-junit5`). Same build and tools as above; times in the same form.

| Case | What happened | Result |
|---|---|---|
| 1. Helper compiled (`mvn test-compile` before opening), Karate 2.1.4 | listed in about 5.4 s; both scenarios passed | as expected |
| 1. Same on Karate 1.5.2 | about 6.4 s; both passed | as expected |
| 2. Helper never compiled (a fresh clone) | both scenarios failed; the console says `Error: TypeError: Java.type: class not found: com.example.qa.DbHelper`; no notice | a clear message from Karate, but not why |
| 2. Helper changed since its last build (a new method, not compiled) | the two old scenarios passed; the new one failed with `Error: TypeError: .countOrders is not a function (called on DbHelper)`; no notice | misleading: it names the method, not the stale build |
| 3. `test`-scoped driver in the list | `libraries.json` holds `h2-2.2.224.jar` (30 jars, Karate 2.1.4) | as expected: `dependency:build-classpath` without `includeScope` lists the test scope |

**Case 2 and the spec.** Grace never compiles: it runs what the project's build compiled. So a helper that is missing or stale fails with Karate's own words, which never say "build the project first". A person who ran `mvn test` before opening Grace never sees it; one who opens a fresh clone does. The owner chose a notice, now built (spec R8).

**Case 2 again, with the notice** (`feature/project-libraries` at `e4ace800`, packaged by `npm run e2e`, the same tools; all 9 cases of the check passed):
- **Never compiled:** opening the project shows `The project's Java code in src/test/java is not built: build it (mvn test-compile) before running.` The run still fails with Karate's `Java.type: class not found`.
- **Changed since its last build:** opening shows `The project's Java code in src/test/java changed since its last build: build it again (mvn test-compile) before running.`
- **Compiled:** no notice, on Karate 2.1.4 and 1.5.2 alike.

The same build passed `npm test` (105 files, 1645 tests) and `npm run e2e` (175 passed, 9.7 m).

## Needs the owner

Their work suite, opened and run in a packaged build, for the points just above.
