# Project libraries: sample projects and their check

Date 2026-11-02. For [#64](https://github.com/hpagniez/grace/issues/64), spec [docs/specs/2026-11-02-project-libraries.md](../../specs/2026-11-02-project-libraries.md). Results: [docs/checks/2026-11-02-project-libraries.md](../../checks/2026-11-02-project-libraries.md).

Sample projects that a real Maven or Gradle lists, and a Playwright check that opens each in the built app, runs it, and checks the result. Unit tests and `npm run e2e` never reach a real Maven or Gradle; this check does, so it runs only by hand, before a release or after a change to how Grace lists libraries.

## The samples

Each Karate 1.x sample has a `karate-config.js` that uses Java interop and a feature with two scenarios: one writes and reads a SQLite table through `org.xerial:sqlite-jdbc` 3.54.1.0, the other starts a local Karate mock and calls it over HTTP (Karate's own server and client, beside the second Netty the 1.x tree brings).

| Sample | What it holds |
|---|---|
| `samples/karate1-maven` | Maven, `com.intuit.karate:karate-junit5:1.4.1`: 87 jars, Graal JS 22.3, Armeria, Netty 4.1.96, Jackson 2.15, logback 1.4 |
| `samples/karate15-maven` | Maven, `io.karatelabs:karate-junit5:1.5.2`: 93 jars, Graal JS 24.0, Netty 4.2.6, Jackson 2.19, logback 1.5 |
| `samples/karate1-gradle` | Gradle, `com.intuit.karate:karate-junit5:1.4.1` |
| `samples/work-suite` | A stand-in for the owner's work suite: one Maven project on Karate 1.5.2, a company library (`com.acme:acme-db`, source in `samples/acme-db`) served by its own repository, a helper class in `src/test/java` that the build compiles and the config loads with `Java.type`, a JUnit runner class, and the SQLite driver in a `work` profile that `.mvn/maven.config` switches on |
| `samples/db-guide-sqlite` | Karate's [database testing](https://docs.karatelabs.io/advanced/database-testing/) guide as written, with SQLite: Maven on Karate 2.1.4 (`io.karatelabs:karate-junit6`), `org.xerial:sqlite-jdbc` 3.54.1.0 `test`-scoped, a helper at the guide's path and name (`src/test/java/com/mycompany/DbUtils.java`, written for this sample) with its `query`, `queryWithParams`, `execute` and `executeInTransaction` over plain JDBC and a constant URL to a local file (`jdbc:sqlite:target/test.db`), and features that load it with `Java.type('com.mycompany.DbUtils')`: Background setup and `afterScenario` cleanup, a parameterised query, a transaction that commits and one that rolls back, and the database checked after a POST to a local mock. The guide's `callStoredProc` is left out: SQLite has no stored procedures. The mock opts in to Java with `configure javaBridgeEnabled = true`, which Karate 2 turns off in mocks by default |
| `samples/db-helper` | Karate's [database testing](https://docs.karatelabs.io/advanced/database-testing/) shape on Karate 2.1.4 (`io.karatelabs:karate-junit6`): a helper `com.example.qa.DbHelper` in `src/test/java` (plain JDBC, constants for the connection, parameterised queries, a transaction), loaded with `Java.type`, and H2 2.2.224 in memory as a `test`-scoped dependency |
| `samples/db-helper-karate1` | The same on Karate 1.5.2 (`io.karatelabs:karate-junit5`) |

`db-helper` runs three times: compiled with `mvn test-compile` first; never compiled, as in a fresh clone, where both scenarios must fail with Karate's `Java.type: class not found`; and changed since its last build (a new method, not compiled), where the scenario that uses it must fail with `… is not a function`. Grace never compiles the project; in those two cases it must say so on open (spec R8), and the check reads that notice.

The check also runs `work-suite` without its `.mvn/maven.config`: the profile is then given only on a command line (`mvn test -Pwork`), which Grace never sees, and the scenario must fail. That case shows the limit, not a bug.

## Run the check

Needs a built app, a Maven, a Gradle and a JDK; none has to be installed.

```bash
npm run package
```

```bash
CHECK_MAVEN_HOME=<maven folder> CHECK_GRADLE_HOME=<gradle folder> CHECK_JDK=<jdk folder> npx playwright test --config docs/research/2026-11-02-project-libraries/playwright.config.ts
```

- `CHECK_JDK` must hold `bin\javac.exe`: the check compiles the company library and runs `mvn test-compile` in copies of `work-suite`, `db-guide-sqlite` and `db-helper`, as the person's own build would have. Gradle needs a JDK too.
- Set `MAVEN_OPTS=-Dmaven.repo.local=<folder>` and `GRADLE_USER_HOME=<folder>` to keep the downloads out of your own `~/.m2` and `~/.gradle`.
- Each case copies its sample to a temp folder, so the samples here never get a `target/`, a `test.db` or a `company-repo/`.
- The machine rules of `AGENTS.md` hold: never at the same time as `npm test`, `npm run e2e` or `npm run bench`.
