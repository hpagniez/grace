# Karate v2 feature inventory (for a desktop runner GUI)

Collected 2026-10-24. Sources: every page in `https://docs.karatelabs.io/sitemap.xml` under getting-started, running-tests, core-syntax, http-requests, http-responses, assertions, reusability, advanced, extensions, api-reference, ide-support, faq (66 pages, fetched as HTML and converted locally). Also the GitHub repo `karatelabs/karate` at `main` (latest release **v2.1.4**, 2026-10-22): `docs/CLI.md`, `docs/DESIGN.md`, `docs/EXT.md`, and the Java sources of the CLI and event system. Also the `karatelabs/karate-cli` README (the Rust launcher).

Conventions:
- `D` = `https://docs.karatelabs.io`. `GH` = `https://github.com/karatelabs/karate/blob/main`.
- `[GUI: run|config|view|edit|none]` = the main way a desktop runner would touch the feature:
  - `run`: start, stop, or select an execution.
  - `config`: a setting the GUI exposes or writes.
  - `view`: a result or artifact the GUI shows.
  - `edit`: something the GUI helps the user write in `.feature`/JS files.
  - `none`: no GUI value; listed only for completeness.
- `(unverified)` = not confirmed in docs and source, or the docs and source disagree. The note says which.
- Commercial (paid Karate Labs) features are marked **[commercial]**. The OSS core cannot use them.

---

## 1. Project & files

- **Feature files (`*.feature`)**: Gherkin with `Feature`, `Background`, `Scenario`, `Scenario Outline` + `Examples`. Supports `Given/When/Then/And/But` or `*`. No step definitions. D/core-syntax/feature-files `[GUI: edit]`
- **JS scripts (`*.karate.js`)**: a second runnable file type with the full `karate.*` API (process control, async). `karate run` discovers both `.feature` and `.karate.js`. GH/docs/CLI.md (Supported File Types) `[GUI: run]`
- **Recommended layout**: Maven-style `src/test/java/<pkg>/*.feature` next to the JSON and JS data, with `karate-config.js` at the classpath root and `logback-test.xml` in `src/test/resources`. D/core-syntax/project-structure `[GUI: view]`
- **`karate-pom.json` project file**: persisted run settings. The CLI loads it automatically from the working directory. Keys:
  - `paths`, `tags`, `env`, `threads`, `scenarioName`, `configDir`, `workingDir`, `dryRun`, `clean`
  - `output{dir,html,junitXml,cucumberJson,jsonLines}`
  - `logging{report,console}`
  - `listeners`, `listenerFactories`

  D/running-tests/command-line; GH/karate-core/src/main/java/io/karatelabs/core/KaratePom.java `[GUI: config]`
- **`karate-config.js`**: a JS function that returns global variables. It runs before **every** scenario. D/core-syntax/configuration `[GUI: edit]`
- **`karate-config-<env>.js`**: environment-specific overlay, loaded after `karate-config.js` when `karate.env` is set. D/core-syntax/configuration `[GUI: edit]`
- **`karate-base.js`**: framework-level defaults, loaded before `karate-config.js`. D/core-syntax/configuration `[GUI: edit]`
- **Config auto-discovery**: if `karate-config.js` and `karate-base.js` are not on the classpath, Karate finds them in the working directory. D/getting-started/whats-new-v2 (More Improvements) `[GUI: config]`
- **`karate-boot.js`** (experimental): suite-lifetime boot file at the working-directory root. It activates extensions via `boot.ext(name)` and has `boot.env/sysenv/sysprop/read/log`. `boot.classpath('src/test/resources')` re-anchors the project root. D/advanced/extensions; GH/docs/CLI.md (Path Resolution) `[GUI: edit]`
- **Path prefixes**:
  - `classpath:` = classpath root
  - `this:` = relative to the current feature
  - `file:` = absolute path
  - no prefix = relative to the feature file
  - A leading `/` in a `read()` reference means the project root

  D/core-syntax/actions; GH/docs/CLI.md `[GUI: edit]`
- **Data files**: `read()` auto-converts JSON, XML, YAML, CSV, JS, `.feature`, text, and binary. Also `readAsString`, `readAsBytes`, `readAsStream`. D/core-syntax/actions; D/core-syntax/data-types `[GUI: view]`
- **Build-dir detection**:
  - `build.gradle*` or `settings.gradle*` → `build/`
  - `pom.xml` → `target/`
  - Override with `-Dkarate.output.dir`

  GH/docs/CLI.md (Temp Directory Structure) `[GUI: config]`
- **`karate-temp/`**: runtime temp dir holding Chrome profiles and the `callSingle` disk cache. `karate clean` removes it. GH/docs/CLI.md `[GUI: view]`
- **Build integrations**: Maven (`io.karatelabs:karate-core` 2.1.4, `karate-junit6`), Gradle (`useJUnitPlatform`, forward `karate.*` sysprops), standalone fat JAR (`karate-<ver>.jar`), and the Karate CLI launcher. D/getting-started/install-dependencies `[GUI: config]`
- **Java requirement**: Java 21+ (virtual threads). D/getting-started/install-dependencies `[GUI: config]`
- **Starter projects**: `karate-template` and `karate-todo` (a live report on GitHub Pages). D/extensions/examples-and-demos `[GUI: none]`

## 2. Configuration & environments

- **`karate.env`**: the environment name. Set it with:
  - `-e/--env`
  - `-Dkarate.env`
  - the `KARATE_ENV` env var
  - pom `env`
  - Builder `.karateEnv()`

  Read it in config with `karate.env`. D/running-tests/command-line `[GUI: config]`
- **Config dir**: `-g/--configdir`, the `karate.config.dir` sysprop, or the `KARATE_CONFIG_DIR` env var. D/running-tests/command-line `[GUI: config]`
- **System properties**: `-Dkey=value` (JAR, Maven) or `Runner.Builder.systemProperty()`. Read them with `karate.properties[...]` or `karate.sysprop(name, default)`. D/core-syntax/configuration; D/getting-started/whats-new-v2 `[GUI: config]`
- **OS env vars**: read with `karate.sysenv(name, default)`. Default applies when unset or empty. D/getting-started/whats-new-v2 `[GUI: config]`
- **`configure` keyword / `karate.configure(key, value)`**: per-scenario settings, auto-restored at scenario end. Keys from D/api-reference/keywords (Configuration Options) and GH/docs/DESIGN.md (KarateConfig):
  - **HTTP**: `url`, `ssl` (bool or `{keyStore,keyStorePassword,keyStoreType,trustStore,trustStorePassword,trustAll,algorithm}`), `connectTimeout` (30000), `readTimeout` (30000), `proxy` (string or `{uri,username,password,nonProxyHosts}`), `followRedirects` (true), `strictResponseParsing` (false), `charset` (utf-8), `headers` (JSON or function), `cookies`, `localAddress`, `httpRetryEnabled`, `auth`, `ntlmAuth`
  - **Logging and reporting**: `logging`, `report`
  - **Flow**: `retry` (`{count, interval}`), `continueOnStepFailure`, `abortSuiteOnFailure`, `abortedStepsShouldPass`
  - **Caching**: `callSingleCache` (`{minutes}`)
  - **Data handling**: `xmlNamespaceAware`, `matchEachEmptyAllowed`
  - **Gatling**: `pauseIfNotPerf`
  - **Driver**: `driver`, `driverTarget`
  - **Hooks**: `beforeScenario`, `afterScenario`, `afterScenarioOutline`, `afterFeature`, `onStepFailure`
  - **Mocks**: `cors`, `responseHeaders`, `requestExpressionsEnabled`, `javaBridgeEnabled`
  - **Channels**: kafka, grpc, and websocket options

  `[GUI: config]`
- **Declarative `configure auth`** (v2): `basic`, `bearer`, `oauth2` (client credentials with auto refresh, or `flow: 'authorization_code'` PKCE), and `ntlm`. Called features inherit it. D/getting-started/whats-new-v2 `[GUI: config]`
- **`callSingleCache`**: disk cache of `karate.callSingle()` results for dev loops, e.g. `{ minutes: 15 }`. D/reusability/calling-features `[GUI: config]`
- **Secrets pattern**: read from env vars or sysprops. Never hard-code. D/core-syntax/configuration `[GUI: config]`
- **Modular config**: per-env JSON files read from `karate-config.js`. D/core-syntax/configuration `[GUI: edit]`
- **Precedence of run options**, highest first:
  1. `karate.options` sysprop
  2. `KARATE_OPTIONS` env
  3. individual `karate.*` sysprops
  4. `KARATE_*` env vars
  5. `Runner.Builder` values
  6. `karate-pom.json`
  7. defaults

  A malformed `karate.options` is logged at WARN and ignored. D/running-tests/command-line `[GUI: config]`
- **CLI vs pom precedence**: CLI flags override pom values. `--no-pom` ignores the pom. Positional paths and `-P` *replace* the pom's paths. GH/docs/CLI.md `[GUI: config]`
- **Launcher config**: `~/.karate/karate-cli.json` (global) and `.karate/karate-cli.json` (project), with keys `channel`, `karate_version`, `jvm_opts`. Env vars: `KARATE_HOME`, `JAVA_HOME`, `NO_COLOR`. https://github.com/karatelabs/karate-cli (README) `[GUI: config]`
- **Commercial license file**: `.karate/karate.lic` or `KARATE_LICENSE_TEXT`. Needed only for the kafka, grpc, and websocket extensions. D/extensions/kafka **[commercial]** `[GUI: none]`

## 3. Execution / CLI

- **Launcher binary `karate`** (Rust, from karate.sh). Install:
  - `curl -fsSL https://karate.sh/install.sh | sh`
  - `irm https://karate.sh/install.ps1 | iex` (Windows)

  Management commands: `setup` (`--all`, `--item jar|jre`, `--force`), `update` (`--all`), `doctor` (`--json`), `version`. It delegates everything else to `java -jar ~/.karate/dist/karate-X.jar`. Global flag `--cp <jar>` adds extra jars, such as the IDE debug jar. Sources: karate-cli README; GH/docs/CLI.md. `[GUI: run]`
- **`karate init`** (Rust, interactive project scaffolding) is listed in CLI.md. It is not in the karate-cli README command table (unverified). `[GUI: run]`
- **`karate run [options] <paths...>`** (Java/picocli). The bare `karate <paths>` form delegates to `run`. A bare `karate` loads `karate-pom.json` if present, else prints help. Flags below are verified in `RunCommand.java`; see GH/karate-core/src/main/java/io/karatelabs/cli/RunCommand.java and D/running-tests/command-line. `[GUI: run]`
  - positional `paths` (0..*); `file.feature:LINE` (or `:L1:L2`) selects scenarios by line and bypasses tag filters
  - `-P, --path <p>`: repeatable and comma-separated; combines with positional paths
  - `-t, --tags <expr>`: repeatable (AND); comma inside one value = OR; `~` = NOT
  - `-T, --threads <n>`: default 1
  - `-e, --env <name>`
  - `-n, --name <name>`: exact scenario name, trimmed; bypasses tag filters; `:LINE` narrows to one outline row
  - `-o, --output <dir>`: default `target/karate-reports` or `build/karate-reports`
  - `-g, --configdir <dir>`
  - `-w, --workdir <dir>`: working dir for relative path resolution; the output dir is NOT relative to it
  - `-C, --clean`: clean the output dir before running
  - `-B, --backup-reportdir [true|false]`: default **true**; moves the old report dir to `<dir>_yyyyMMdd_HHmmss[_n]`
  - `-D, --dryrun`
  - `-p, --pom <file>` (note: `-p` is NOT port here)
  - `--no-pom`
  - `--log-report <level>`: `trace|debug|info|warn|error`, default `debug`
  - `--log-console <level>`: also `off|none`. The help text says default `info`. The logging docs say unset leaves Logback alone (docs and help text disagree).
  - `-f, --format <list>`: `html`, `cucumber:json`, `junit:xml`, `karate:jsonl`; prefix `~` to disable, e.g. `-f ~html,karate:jsonl`
  - `--listener <classes>`, `--listener-factory <classes>`: RunListener and RunListenerFactory FQCNs with no-arg constructors; must be on the classpath
  - `--no-color`: top-level; `NO_COLOR` and `FORCE_COLOR=1` env are also honoured; colour is auto-off with no TTY
  - `-h/--help`, `-V/--version`
- **Flags the brief asked about that do NOT exist in `run`**:
  - `-d` (debug)
  - `-m` (mock is `karate mock -m`)
  - `-p` as port (it is `--pom`)
  - `-n` as dry run (`-n` is name, dry run is `-D`)

  `karate perf` is only listed as a "Future Command" in CLI.md (unverified / not implemented in core). `[GUI: none]`
- **`karate mock`**: see §13. `[GUI: run]`
- **`karate clean`**: `-o/--output`, `-w/--workdir`, `-p/--pom`, `--no-pom`. Cleans report and temp dirs. GH/karate-core/src/main/java/io/karatelabs/cli/CleanCommand.java `[GUI: run]`
- **Exit codes**:
  - `0`: all passed (also when no paths were given)
  - `1`: test failures
  - `2`: configuration or usage error (picocli)
  - `3`: runtime error / exception

  GH/docs/CLI.md (Exit Codes); RunCommand.java (`return result.isFailed() ? 1 : 0`, `return 3`) `[GUI: run]`
- **CLI extension SPI**: ext jars add subcommands via ServiceLoader (`CliCommandProvider`), e.g. commercial `karate serve`. GH/karate-core/src/main/java/io/karatelabs/Main.java `[GUI: none]`
- **Standalone JAR**: `java -jar karate.jar [run] ...`, same flags, `-Dkarate.env=...` accepted. D/getting-started/standalone-execution `[GUI: run]`
- **Maven/Gradle + JUnit 6**: `@Karate.Test Iterable<DynamicNode>` returning `Karate.run(paths).tags().threads().karateEnv().relativeTo(getClass())`. Run with `mvn test -Dtest=X`, `-Dkarate.options="..."`, or `./gradlew test --tests X`. D/running-tests/junit; D/running-tests/command-line `[GUI: run]`
- **`karate.options` grammar**: the same as `karate run`, POSIX-tokenised, and it can hold paths. Paths in it replace Builder paths. D/running-tests/command-line `[GUI: run]`
- **Dry run (`-D`)**: parse and report without executing. Behaviour:
  - Config JS and before/after hooks are skipped.
  - `@setup` scenarios still run.
  - All report formats are produced.
  - `karate.suite.dryRun` is readable.
  - The console says `DRY RUN — nothing was executed`.
  - Step status is **skipped** per the command-line page but **passed** per the java-api page (docs inconsistent; unverified).

  D/running-tests/command-line `[GUI: run]`
- **Fail-fast**: `configure abortSuiteOnFailure = true` skips queued work and interrupts in-flight scenarios at the next step. D/running-tests/parallel-execution `[GUI: config]`
- **Console summary**: elapsed, threads, thread time, features, ignored, efficiency, scenarios passed/failed. It also prints the `file:///.../karate-summary.html` link. D/running-tests/parallel-execution; D/running-tests/test-reports `[GUI: view]`
- **Run a scenario by line or name from an IDE**: this is how IDE plugins target one test. D/running-tests/command-line `[GUI: run]`
- **CI recipes**: GitHub Actions, GitLab, Jenkins (`publishHTML`, `junit`), GitHub Pages publishing, a secret-leak grep over reports, Codespaces. D/running-tests/ci-cd `[GUI: none]`

## 4. Selection & tags

- **Tag filters**: `@tag`, `~@tag`, comma = OR, repeat `-t` = AND. Builder docs also show `"(@smoke or @critical) and not @flaky"`; CLI support for that boolean syntax is unverified. D/running-tests/tags; D/running-tests/parallel-execution `[GUI: run]`
- **Tag inheritance**: feature + scenario tags. For outlines: feature + outline + that Examples block. Tags are case-sensitive. D/running-tests/tags `[GUI: view]`
- **Tagged `Examples:` blocks**: filters apply per block. D/running-tests/tags `[GUI: run]`
- **`@ignore`**: always excluded. Such scenarios can still be called. D/running-tests/tags `[GUI: view]`
- **`@env=a,b` / `@envnot=x`**: run only in (or exclude from) given `karate.env` values. D/running-tests/tags `[GUI: view]`
- **`@lock=<name>` / `@lock=*`**: named or exclusive mutual exclusion in parallel runs. Replaces v1 `@parallel=false`, which is silently ignored. D/running-tests/parallel-execution `[GUI: view]`
- **`@setup`**: reserved; runs only via `karate.setup()`, `setupOnce()`, or a call. D/reusability/dynamic-scenarios `[GUI: view]`
- **`@report=false`**: runs but is hidden from all report artifacts. Failures show a redacted message. D/running-tests/logging `[GUI: view]`
- **Key=value tags**:
  - `@key=v1,v2`, read with `karate.tagValues`
  - In JUnit XML, `<property name="key">`; plain tags go to `tags`
  - Xray keys: `requirements`, `test_key`, `test_id`, `test_summary`, `test_description`, `tr:<field>`

  D/running-tests/tags; D/running-tests/test-reports `[GUI: view]`
- **Coverage tags** `@req=` / `@cov=openapi:<operationId>`: read by the commercial coverage extension, even in a dry run. D/running-tests/command-line **[commercial]** `[GUI: view]`
- **Call by tag**: `call read('file.feature@tag')`, `@name=x`. D/running-tests/tags `[GUI: edit]`
- **Name filter `-n`** and **line filter `:LINE`**: both bypass `@ignore` and `@env`. D/running-tests/command-line `[GUI: run]`
- **`karate.tags`**: the effective tags at runtime. D/running-tests/tags `[GUI: none]`

## 5. Parallelism

- **Virtual-thread parallel runner**: `-T n` or `.parallel(n)`. Features, scenarios, and outline rows all run in parallel. D/running-tests/parallel-execution `[GUI: config]`
- **Scenario isolation required**: no shared state, no order dependence. D/running-tests/parallel-execution `[GUI: none]`
- **`@lock`** for shared resources, see §4. `[GUI: view]`
- **Efficiency metric and timeline Gantt** (`karate-timeline.html`): thread utilisation. D/running-tests/parallel-execution `[GUI: view]`
- **Browser pooling**: `PooledDriverProvider`; the pool size follows the thread count. D/extensions/ui-testing `[GUI: config]`
- **Per-thread listeners**: `RunListenerFactory`, called once per execution thread. GH/.../core/RunListenerFactory.java `[GUI: none]`
- **Thread-count guidance**: 1–2× cores, about 256 MB per thread. D/running-tests/parallel-execution `[GUI: config]`

## 6. Reports & outputs

- **Output dir**: default `target/karate-reports` (Maven) or `build/karate-reports` (Gradle). Set with `-o`, pom `output.dir`, `-Dkarate.output.dir`, or `.outputDir()`. Paths are relative to the process cwd, not to `workingDir`. D/running-tests/test-reports; GH/docs/CLI.md `[GUI: config]`
- **Report dir backup**: the previous dir is moved to `karate-reports_yyyyMMdd_HHmmss[_n]` on each run (default on; `-B false` to disable). This gives free run history. GH/karate-core/src/main/java/io/karatelabs/core/Suite.java (`backupReportDirIfExists`) `[GUI: view]`
- **HTML report (default on)**: Tailwind and Alpine based, self-contained (JSON inlined). Contents:
  - Summary chart, failures panel, tag-chip filter
  - Dark mode
  - Per-step request/response with JSON highlighting
  - Embeds and screenshots
  - Disable with `-f ~html`

  D/running-tests/test-reports `[GUI: view]`
  - Files:
    - `index.html` (redirect)
    - `karate-summary.html`
    - `karate-timeline.html`
    - `feature-html/<feature.path dotted>.html`
    - `res/` (css/js)

    GH/karate-core/src/main/java/io/karatelabs/output/HtmlReportWriter.java
- **JSON Lines event stream**: `karate-json/karate-events.jsonl` (opt-in, `-f karate:jsonl`). Written live. See §19 for the schema. D/running-tests/test-reports `[GUI: view]`
- **Cucumber JSON**: `cucumber-json/<feature>.json`, one per feature (opt-in `-f cucumber:json`). D/running-tests/test-reports `[GUI: view]`
- **JUnit XML**: `junit-xml/<feature>.xml`, one per feature (opt-in `-f junit:xml`). Tags become `<property>` elements. D/running-tests/test-reports `[GUI: view]`
- **No legacy "Karate JSON" per-feature result files are documented in v2.** The full `FeatureResult.toJson()` appears only inside the JSONL `FEATURE_EXIT` event (unverified that no other JSON file exists). `[GUI: view]`
- **Embeds**: `karate.embed(data, mime, name)`, the `doc` keyword (HTML templates into the report), auto-screenshots, image diffs. D/core-syntax/actions; D/advanced/karate-object `[GUI: view]`
- **Friendly failure labels**: the Gherkin comment above a `match` or `assert` is shown as the failure title. D/getting-started/whats-new-v2 `[GUI: view]`
- **Complete mismatch diffs**: every mismatched JSON or XML path, with actual vs expected values and types. D/assertions/match-keyword `[GUI: view]`
- **cURL export** of HTTP requests (where it is surfaced is unverified). D/getting-started/whats-new-v2 `[GUI: view]`
- **Report verbosity**: `logging.report` threshold and `@report=false`. D/running-tests/test-reports `[GUI: config]`
- **Third-party reporting**: maven-cucumber-reporting, Allure (via Cucumber JSON), ReportPortal (via JSONL), and Xray, Zephyr, qTest, TestRail (via JUnit XML). D/running-tests/test-reports `[GUI: none]`
- **Gatling HTML report**: produced by the Gatling plugin, standard Gatling location (path not stated in docs; unverified). D/extensions/performance-testing `[GUI: view]`

## 7. Logging

- **`configure logging`** keys:
  - `report`: capture threshold, default `debug`
  - `console`: SLF4J threshold, plus `off`/`none`
  - `pretty`: JSON bodies, default true
  - `mask`: `headers`, `jsonPaths` (`$.a.b`, `$..x`), `patterns` (`{regex, replacement}`), `replacement`, `enableForUri(fn)`, and per DESIGN.md also `defaults`

  Deep-merges. Mid-scenario changes are auto-restored. D/running-tests/logging `[GUI: config]`
- **Two thresholds**: `report` vs `console`. HTTP bodies always go to the report at INFO. Console tiers: INFO = one-liner, DEBUG = +headers, TRACE = +body. D/running-tests/logging `[GUI: config]`
- **CLI and pom**: `--log-report`, `--log-console`, pom `logging{report,console}`. Masks are config-only, not CLI. D/running-tests/logging `[GUI: config]`
- **SLF4J categories**:
  - `karate.runtime`, `karate.http`, `karate.mock`, `karate.server`, `karate.scenario`, `karate.console`
  - Gatling: `io.karatelabs.gatling.KarateExecutor`, `io.karatelabs.gatling.LogReplayer`
  - Setting `console` overrides `logback.xml` pins.

  D/running-tests/logging `[GUI: config]`
- **`logback-test.xml`**: user Logback config, e.g. a file appender to `target/karate.log`. D/running-tests/logging `[GUI: edit]`
- **User logging**: `print`, `karate.log()`, `karate.logger.debug/info/warn/error()`, `karate.pretty()`, `karate.prettyXml()`. D/running-tests/debugging `[GUI: view]`
- **ANSI colour**: auto-stripped without a TTY. `--no-color`, `NO_COLOR`, `FORCE_COLOR=1`. A GUI that captures stdout gets plain text unless it forces colour. D/running-tests/logging `[GUI: config]`
- **Mask verification**: a DEBUG line `http log mask active: N headers, ...` on `karate.runtime`. D/running-tests/logging `[GUI: view]`

## 8. Debugging

- **Print and inspect helpers**: `karate.pretty`, `keysOf`, `typeOf`, `karate.match()` (returns `{pass, message}`). D/running-tests/debugging `[GUI: view]`
- **Run one scenario**: by line (`f.feature:15`) or `-n name`; use `-T 1` to keep logs ordered. D/running-tests/debugging `[GUI: run]`
- **IDE debugger (DAP) hook**: if `-Dkarate.debug.port=<port>` is set, `Runner` delegates to `io.karatelabs.debug.Main.run(String[])`. That class lives in the **commercial `karate-ide` jar**. Without the jar it logs a WARN and runs normally. The CLI way to add the jar is `karate --cp /path/karate-ide-v2.jar run ...`. The OSS core ships no debug server. GH/karate-core/src/main/java/io/karatelabs/core/Runner.java (`startDebugServerIfRequired`); GH/docs/CLI.md **[commercial]** `[GUI: run]`
- **Debug building blocks in OSS**: `Runner.Builder.debugSupport(RunInterceptor, DebugPointFactory)` and the karate-js `DebugPointFactory`. These are low-level Java hooks a custom debugger could use (no docs; unverified usability). GH/.../core/Runner.java `[GUI: none]`
- **VS Code and IntelliJ plugins**: breakpoints in `.feature` files, embedded-JS debugging, Java and Karate in one session, step-back. These are PRO-tier features. VS Code run modes `auto|karate-cli|vscode-java`. D/ide-support/vs-code; D/ide-support/intellij **[commercial]** `[GUI: none]`
- **`karate.stop(port)`**: pauses the scenario until `curl http://localhost:<port>` is hit. A GUI could expose a "resume" button. D/extensions/ui-testing `[GUI: run]`
- **`configure driver = { stop: false }`**: keep the browser open after the scenario. D/extensions/ui-testing `[GUI: config]`
- **`highlight()` / `highlightAll()`**, driver `highlight: true`. D/extensions/ui-testing `[GUI: config]`
- **`onStepFailure` hook**: attach artefacts on failure, see §17. `[GUI: view]`

## 9. HTTP / API

- **Request keywords**:
  - `url`, `path` (segments, auto-encoded), `request`, `method` (get/post/put/patch/delete/head/options)
  - `status`, `param/params`, `header/headers`, `cookie/cookies`, `form field/form fields`
  - `multipart field/fields/file/files/entity`, `soap action`

  D/http-requests/making-requests; D/api-reference/keywords `[GUI: edit]`
- **URL persistence**: `url` resets per request; `configure url` persists across calls. D/core-syntax/configuration `[GUI: config]`
- **Headers and cookies**: global `configure headers` / `configure cookies`; dynamic header functions. D/http-requests/headers-and-cookies `[GUI: config]`
- **Auth patterns**:
  - Basic, Bearer, API key, OAuth2 client credentials, JWT, token refresh
  - NTLM (`configure ntlmAuth` or `auth.type='ntlm'`)
  - mTLS via `configure ssl = {keyStore,...}`

  D/http-requests/headers-auth `[GUI: config]`
- **Request signing**: `configure headers = function(req){...}`, called per request with the built request (HMAC, AWS SigV4). D/http-requests/request-signing `[GUI: edit]`
- **Multipart and file upload**. D/http-requests/multipart-requests `[GUI: edit]`
- **GraphQL**: query and variables as JSON, file-based queries, placeholder substitution. D/http-requests/graphql `[GUI: edit]`
- **SOAP**: `soap action`, XML namespaces, XPath assertions, faults. D/http-requests/soap `[GUI: edit]`
- **Polling**: `retry until <expr>` with `configure retry = {count, interval}`. D/http-requests/polling-and-async `[GUI: config]`
- **Response variables**: `response`, `responseStatus`, `responseHeaders` (case-insensitive), `responseCookies`, `responseTime`, `requestTimeStamp`, `responseBytes`, `responseType`, `karate.prevRequest`, `karate.response`. D/http-responses/response-handling; D/http-responses/response-time `[GUI: view]`
- **HTTP client**: Apache HttpClient 5.6 with Brotli. Settings: proxy with `nonProxyHosts` wildcards, `localAddress`, `httpRetryEnabled`. D/getting-started/whats-new-v2 `[GUI: config]`
- **`karate.http(url)`** JS client; **`karate.waitForHttp(url)` / `karate.waitForPort(host, port)`**. D/advanced/karate-object `[GUI: edit]`

## 10. Assertions

- **`match`** operators: `==`, `!=`, `contains`, `!contains`, `contains only`, `contains any`, `contains deep`, `contains only deep`, `match each`, `match each contains deep`, `match header`, and v2 `within` / `!within` (numeric range). D/assertions/match-keyword `[GUI: edit]`
- **Fuzzy markers**:
  - `#ignore`, `#null`, `#notnull`, `#present`, `#notpresent`
  - `#boolean`, `#number`, `#string`, `#uuid`, `#regex`, `#array`, `#object`
  - `#? expr`, `#[n]`, `#[] schema`
  - `##` optional; `$` parent reference; `_` self

  D/assertions/fuzzy-matching `[GUI: edit]`
- **Schema validation**: reusable schema JSON files, embedded `#(schema)` refs, cross-field checks. D/assertions/schema-validation `[GUI: edit]`
- **`assert <js-expr>`**. D/core-syntax/variables `[GUI: edit]`
- **Data manipulation**: `set`, `set` multiple (table), `remove`, `delete`, `replace`, `copy`; JsonPath `get` and `$var[...]`; XPath. D/assertions/match-keyword `[GUI: edit]`
- **Soft assertions**: `configure continueOnStepFailure = true` (boolean only in v2). D/core-syntax/configuration `[GUI: config]`
- **`karate.expect()`**: Chai-style (`to.equal`, `be.above`, `have.property`, `include`, `.not`, ...). D/assertions/match-keyword `[GUI: edit]`
- **Complete mismatch diffs** and **assertion labels from comments**. D/assertions/match-keyword `[GUI: view]`
- **Response validation patterns**: array size, contains variants, error responses. D/http-responses/response-validation `[GUI: edit]`

## 11. Reusability & data-driven

- **`call read('x.feature')`**: arguments, return values, isolated vs shared scope, `call` per array element (data-driven), tag selectors. D/reusability/calling-features `[GUI: edit]`
- **`callonce`**: once per feature. **`karate.callSingle()`**: once per suite, with optional disk cache. D/reusability/calling-features `[GUI: edit]`
- **Calling JS functions and Java** (`Java.type`). D/reusability/calling-features `[GUI: edit]`
- **`Scenario Outline` + `Examples`**:
  - `<placeholders>`
  - type hints (column `!` suffix evaluates JS)
  - magic variables `__row`, `__num`
  - dynamic scenario names
  - CSV, JSON, and YAML data sources

  D/reusability/data-driven-tests `[GUI: view]`
- **`table` keyword**: inline JSON arrays. D/core-syntax/variables `[GUI: edit]`
- **Dynamic scenarios**: `Examples: | karate.setup().data |`, `@setup` + `karate.setup([name])` / `karate.setupOnce()`, generator functions. D/reusability/dynamic-scenarios `[GUI: view]`
- **Built-in call args**: `__arg`, `__loop`. D/api-reference/keywords `[GUI: none]`
- **`karate.faker.*`** test data and **`karate.uuid()`**. D/core-syntax/variables `[GUI: edit]`
- **Conditional logic**: `if`, ternaries, `karate.abort()`, `karate.fail()`, conditional `karate.call()`, `karate.map`/`filter`/`forEach`/`repeat`/`range`/`sort`/`distinct`. D/advanced/conditional-logic; D/advanced/karate-object `[GUI: edit]`

## 12. Async / WebSocket / MQ / DB

- **Listen and signal**: the `listen <timeout>` keyword with `karate.signal(result)` and the `listenResult` variable. D/advanced/polling-and-async; D/advanced/message-queues `[GUI: edit]`
- **WebSocket (OSS)**: `karate.webSocket(url, handler, options)` and `karate.webSocketBinary()`, with options for subprotocol and headers. D/advanced/websocket `[GUI: edit]`
- **WebSocket extension**: `karate.channel('websocket')` with `send`, `pop`, `collect`, and codecs such as STOMP. Needs `karate-agent` or `karate-async` jar plus a license. D/extensions/websocket **[commercial]** `[GUI: none]`
- **Message queues**: through Java wrapper classes (JMS, ActiveMQ, RabbitMQ) and listen/signal. D/advanced/message-queues `[GUI: edit]`
- **Kafka extension**: `boot.ext('kafka')`, Avro and Protobuf, TLS and SASL. D/extensions/kafka **[commercial]** `[GUI: none]`
- **gRPC extension**: `boot.ext('grpc')`, reads `.proto` files directly. D/extensions/grpc **[commercial]** `[GUI: none]`
- **Database**: through Java interop with JDBC helper classes (no built-in DB keyword). D/advanced/database-testing `[GUI: edit]`
- **Process control**: `karate.exec(cmd)` and `karate.fork(cmd)` (process object with `waitSync`, `waitForOutput`, `waitForPort`, `waitForHttp`, `onStdOut`, `close`, `signal`). D/advanced/karate-object; GH/docs/DESIGN.md `[GUI: none]`
- **Custom polling and backoff** in JS; `karate.pause(ms)`. D/advanced/polling-and-async `[GUI: edit]`

## 13. Mocks / test doubles

- **Mock feature files**: each `Scenario` is a request matcher. Matcher functions: `pathMatches`, `methodIs`, `typeContains`, `acceptContains`, `headerContains`, `headerValue`, `paramValue`, `paramExists`, `bodyPath`. Request vars: `request`, `requestHeaders`, `requestParams`, `pathParams`, ... Response vars: `response`, `responseStatus`, `responseHeaders`, `responseDelay`. D/extensions/test-doubles `[GUI: edit]`
- **Stateful mocks**: `Background` variables persist across requests; catch-all scenario. D/extensions/test-doubles `[GUI: edit]`
- **JS mock handlers** (v2): a `.js` file with `request`, `response`, and `session` objects. D/extensions/test-doubles `[GUI: edit]`
- **Proxy mode**: `karate.proceed([url])`, selective stubbing, response modification. D/extensions/test-doubles `[GUI: edit]`
- **CORS and global response headers**: `configure cors`, `configure responseHeaders`. D/extensions/test-doubles `[GUI: config]`
- **Starting mocks**:
  - In tests: `karate.start(path|config)`, auto-stopped at the end of the owning scenario or suite
  - Java: `MockServer.feature(...).http(0).build()` with `getPort()`, `getVariable()`, `stop()`
  - CLI

  D/extensions/test-doubles; GH/docs/DESIGN.md `[GUI: run]`
- **`karate mock` CLI flags** (verified in source):
  - `-m, --mock <file>`: repeatable, required
  - `-p, --port <n>`: default 0 = dynamic; the URL is printed
  - `-s, --ssl` (self-signed cert if none given), `-c, --cert <pem>`, `-k, --key <pem>`
  - `--path-prefix <p>`
  - `-W, --watch`: hot reload
  - `--max-initial-line-length` (4096 → 414), `--max-header-size` (8192 → 431), `--max-content-length` (1048576 → 413)
  - The docs also list `-e` for `karate.env`, but `MockCommand.java` declares no such option (**unverified**).

  GH/karate-core/src/main/java/io/karatelabs/cli/MockCommand.java; D/extensions/test-doubles `[GUI: run]`
- **Mock console output**: `URL: <protocol>://localhost:<port>` (a GUI can parse the port), then "Press Ctrl+C to stop". Stop with Ctrl+C or `GET /__admin/stop`. D/extensions/test-doubles; MockCommand.java `[GUI: run]`
- **Mock trust boundary**: `requestExpressionsEnabled` and `javaBridgeEnabled` default false; since 2.1.4 `exec`/`fork` are off in mocks. D/extensions/test-doubles `[GUI: config]`
- **Browser request interception**: `driver.intercept({patterns, mock})`, Chrome only. D/extensions/ui-testing `[GUI: edit]`

## 14. Performance (Gatling)

- **`karate-gatling` module**: Java-only DSL, no Scala. `karateProtocol(uri(...).nil()/pauseFor(method(...)))`, `karateFeature("classpath:...")` (tag selectors, silent mode), `nameResolver`, `protocol.runner.karateEnv()`. D/extensions/performance-testing `[GUI: edit]`
- **Run via Maven only**: `mvn clean test-compile gatling:test [-Dgatling.simulationClass=...]`, or a Maven profile (`mvn clean test -P gatling`). There is no core CLI `perf` command. D/extensions/performance-testing `[GUI: run]`
- **Data flow**: Gatling session access, `karateSet()`, feeders, chaining, `karate.callSingle`, detecting Gatling at runtime, think time (`pauseFor`, `karate.pause`), `configure localAddress`. D/extensions/performance-testing `[GUI: edit]`
- **Reporting**:
  - URI pattern grouping, Gatling groups
  - Custom perf events for non-HTTP work (`PerfHook`)
  - Failure messages, fail-fast
  - Optional `logReplay()` of failure context
  - No Karate HTML report under Gatling

  D/extensions/performance-testing `[GUI: view]`
- **Limitation**: Gatling `throttle` is not supported. Distributed testing is covered in a wiki. D/extensions/performance-testing `[GUI: none]`

## 15. UI / browser automation

- **`configure driver`** keys:
  - `type`, `executable`, `start`, `stop`, `port`, `host`, `headless`, `timeout`
  - `retryCount`, `retryInterval`
  - `showDriverLog`, `showProcessLog`, `showBrowserLog`
  - `addOptions`, `beforeStart`, `afterStop`, `videoFile`, `httpConfig`
  - `attach`, `userDataDir`, `highlight`, `highlightDuration`, `screenshotOnFailure` (default true)
  - Also `webDriverUrl`, `webDriverSession`, `webDriverPath`

  D/extensions/ui-testing `[GUI: config]`
- **Driver types** (default port):
  - `chrome` (CDP, 9222; recommended), `msedge` (9222), `playwright` (4444)
  - `chromedriver` (9515), `geckodriver` (4444), `safaridriver` (5555), `msedgedriver` (9515)
  - `mswebdriver` (17556), `iedriver` (5555)
  - `android` and `ios` (Appium, 4723), `winappdriver` (4727)

  D/extensions/ui-testing `[GUI: config]`
- **v2 CDP driver**: auto-wait, Shadow DOM locators, browser pooling, auto-screenshot on failure. D/getting-started/whats-new-v2 `[GUI: view]`
- **Remote and grid**: Selenium Grid, cloud providers, custom capabilities, Testcontainers, custom `DriverProvider` / `PooledDriverProvider`. D/extensions/ui-testing `[GUI: config]`
- **Locators**: CSS, XPath, wildcard (`{a}text`), friendly locators (`rightOf`, ...), tree walking, Shadow DOM. D/extensions/ui-testing `[GUI: edit]`
- **Actions**: click, input, special keys, select, file upload, mouse, scroll, dialogs, frames, windows and tabs, cookies. D/extensions/ui-testing `[GUI: edit]`
- **Waits**: `waitFor`, `waitForUrl`, `waitForText`, `waitForEnabled`, `waitForResultCount`, `waitForAny`, `waitUntil`, `retry()`. D/extensions/ui-testing `[GUI: edit]`
- **Browser JS**: `script`, `scriptAll`, `locateAll` with filters. D/extensions/ui-testing `[GUI: edit]`
- **Screenshots, PDF, video**: `screenshot()`, `pdf()`, the `videoFile` embed. D/extensions/ui-testing `[GUI: view]`
- **Mobile**: device emulation, viewport, Appium, screen recording. D/extensions/ui-testing `[GUI: config]`
- **Hybrid API + UI tests**, pre-authenticated sessions. D/extensions/ui-testing `[GUI: edit]`
- **Driver reuse** across called features; `karate.driver` from JS. D/extensions/ui-testing `[GUI: edit]`

## 16. Image comparison

- **`karate-image` extension** (v2; replaces v1 `compareImage`). Activate it in `karate-boot.js` with `boot.ext('image')`. D/extensions/image-comparison `[GUI: config]`
- **Config keys**:
  - `baselineDir`, `optionsDir`
  - `threshold` (%, 0 = exact)
  - `engine`: `resemble`, `ssim`, `resemble,ssim`, `resemble|ssim`
  - `failOnMismatch`
  - `report`: `all`, `mismatched`, `null`
  - `allowScaling`, `rebaseCommand`, `optionsCommand`

  Set via boot, `image.x = ...`, `image.config = {...}`, or per-image `<name>.json`. D/extensions/image-comparison `[GUI: config]`
- **API**: `image.diff`, `image.resolve`, `image.write`, plus the result object. A user-side `screenGrab` recipe adopts the latest image as baseline on first run. D/extensions/image-comparison `[GUI: edit]`
- **Engine options**: ignored boxes and regions, `ignoreColors`, tolerances, `errorColor`. D/extensions/image-comparison `[GUI: config]`
- **Report lightbox**:
  - Views: diff, slider, blink, onion, side-by-side, 1:1 zoom
  - Live re-diff
  - Ignore-box authoring
  - "Show options" and "Rebase" produce copy-paste commands; a static report cannot write files.

  A desktop GUI could add a real **accept-baseline** action. D/extensions/image-comparison `[GUI: view]`

## 17. Java API & hooks

- **`Runner.path(...)` / `Runner.builder()`** methods:
  - Selection: `path`, `features`, `overlay(path, bytes)` (run unsaved buffer content), `tags`, `scenarioName`, `skipTagFiltering`
  - Environment: `karateEnv`, `configDir`, `systemProperty`, `systemEnv`, `workingDir`
  - Output: `outputDir`, `outputHtmlReport`, `outputJsonLines`, `outputJunitXml`, `outputCucumberJson`, `backupOutputDir`, `outputMask`, `outputConsoleSummary`
  - Logging: `logLevel`, `consoleLevel`, `captureStepLogs`, `retainStepLogs`, `retainCallResults`
  - Execution: `dryRun`
  - Extension points: `global`, `listener`, `listeners`, `listenerFactory` (instance or class name), `resultListener`, `onSuite`, `driverProvider`, `pool([n])`, `httpClientFactory`, `debugSupport`
  - Caches: `callSingleCache`, `callOnceCacheStore`, `setupOnceCacheStore`
  - `parallel(n)` runs the suite and returns `SuiteResult`

  GH/karate-core/src/main/java/io/karatelabs/core/Runner.java; D/advanced/java-api `[GUI: none]`
- **`Runner.runFeature(path, vars, ...)`**: invoke one feature from Java and get a `FeatureResult`. D/advanced/java-api `[GUI: none]`
- **`RunListener`** (`boolean onEvent(RunEvent)`; returning false from `*_ENTER` skips execution). It replaces v1 `RuntimeHook` and `ResultListener`. CLI: `--listener`. D/advanced/java-api; GH/.../core/RunListener.java `[GUI: run]`
- **`RunEventType`** values: `SUITE_ENTER/EXIT`, `FEATURE_ENTER/EXIT`, `OUTLINE_ENTER`, `SCENARIO_ENTER/EXIT`, `STEP_ENTER/EXIT`, `HTTP_ENTER/EXIT`, `MATCH_EXIT`, `ERROR`, `PROGRESS` (`{completed,total,percent}`). GH/.../core/RunEventType.java; ProgressRunEvent.java `[GUI: view]`
- **Gherkin-level hooks**: `configure beforeScenario` (set in `karate-config.js`), `afterScenario`, `afterScenarioOutline`, `afterFeature` (Runner API), `onStepFailure(info)` with `info.embed()`, `info.proceed()`, `info.stop()`. Hook exceptions fail the scenario. D/advanced/hooks `[GUI: edit]`
- **`karate` object**: variables (`get/set/copy/remove/eval`), transforms, calls, HTTP, assertions, file I/O (`read*`, `write`, `toAbsolutePath`), config and env (`karate.info`, `os`, `scenario`, `feature`, `suite`, `tags`), flow (`abort`, `fail`, `pause`, `stop`, `setup`), process (`exec`, `fork`), async (`channel`, `signal`, `start`), utilities (`embed`, `doc`, `render`, `toJson`, `toCsv`, `toBean`, `toJava`, ...). D/advanced/karate-object `[GUI: edit]`
- **Extension SPI (`Ext`)**: name convention `boot.ext('x')` → `io.karatelabs.ext.x.XExt`. It can add a JS-scope object, observe all events, and add report assets. GH/docs/EXT.md; D/advanced/extensions `[GUI: none]`
- **v1 compatibility shims**: the `com.intuit.karate.*` packages (Runner, MockServer, Main) delegate to v2. D/getting-started/migration-from-v1 `[GUI: none]`

## 18. v2 changes

All items from D/getting-started/whats-new-v2 and D/getting-started/migration-from-v1.

- **Ground-up rewrite**:
  - Own JS engine (`karate-js`, ES6+, no GraalVM).
  - Java 21+ with virtual threads.
  - Maven groupId `io.karatelabs`, package `io.karatelabs.*`.

  `[GUI: config]`
- **First-class CLI** `karate run/mock/clean` plus `karate-pom.json`. `[GUI: run]`
- **`@lock` replaces `@parallel=false`**, which is silently ignored. `[GUI: view]`
- **Assertions**: `match within`, `karate.faker`, `karate.expect`, `karate.uuid`, complete diffs, soft assertions (boolean `continueOnStepFailure`), comment labels. `[GUI: view]`
- **Declarative `configure auth`**, Apache HttpClient 5.6. `[GUI: config]`
- **Mock server rewrite** on Netty: JS handlers, non-blocking delays, built-in CORS. `[GUI: run]`
- **Gatling rewrite**: Java DSL and `PerfHook`. `[GUI: none]`
- **CDP driver rewrite**: auto-wait, pooling, Shadow DOM, W3C WebDriver support, auto-screenshot on failure. `[GUI: view]`
- **Image comparison** moved to the `karate-image` extension and `karate-boot.js`. `[GUI: config]`
- **New HTML report**: Tailwind, dark mode, tag filter, timeline. `[GUI: view]`
- **`karate-events.jsonl`** live event stream. `[GUI: view]`
- **Unified `configure logging`**: replaces `logPrettyRequest`, `logPrettyResponse`, `printEnabled`, `lowerCaseResponseHeaders`, and `HttpLogModifier`. `[GUI: config]`
- **JUnit 6** (`karate-junit6`, `@Karate.Test`). `[GUI: none]`
- **Unified `RunListener`** and `onStepFailure` hook. `[GUI: none]`
- **`karate.sysenv()` / `karate.sysprop()`**. `[GUI: none]`
- **Extension SPI + `karate-boot.js`** (experimental). `[GUI: config]`
- **Other**:
  - JS debugging (via commercial IDE plugins)
  - Disk-backed large JSON operations
  - cURL export
  - Config auto-discovery from the working dir
  - `--no-color`

  `[GUI: none]`

---

## 19. Integration surface for a GUI

> **Corrected by a real run:** the [Karate CLI spike](../research/2026-10-24-karate-cli-spike/README.md#corrections-to-the-inventory) found several points below wrong (no `PROGRESS` event, colour not auto-off, boolean tag syntax, paths resolve from cwd, no mock admin stop). Trust the spike where they differ.

How an Electron app can drive Karate OSS without commercial parts.

1. **Spawn the process.** Use either `karate run ...` (the launcher must be installed; it manages the JRE and jar) or, for full control, `java [-Dprop=..] -jar karate-<ver>.jar run ...` with a bundled or detected Java 21+.
   - Set the working dir explicitly with `-w`. Pass an **absolute** `-o`, because the output dir resolves from the process cwd.
   - Evidence: GH/docs/CLI.md; D/getting-started/standalone-execution.
2. **Build arguments from the UI.** Use only real flags: paths and `-P`, `-t` (repeat for AND), `-T`, `-e`, `-n`, `file:LINE`, `-o`, `-g`, `-w`, `-C`, `-B false`, `-D`, `-p` (pom), `--no-pom`, `--log-report`, `--log-console`, `-f html,karate:jsonl,junit:xml,cucumber:json`, `--listener`, `--listener-factory`, `--no-color`.
   - Evidence: GH/karate-core/src/main/java/io/karatelabs/cli/RunCommand.java.
   - Watch out: an inherited `KARATE_OPTIONS` or `-Dkarate.options` overrides Builder values. Scrub the child environment.
   - Evidence: D/running-tests/command-line (Precedence).
3. **Persist settings in `karate-pom.json`.** The GUI can read and write it as its project file; CLI flags override it per run.
   - Evidence: GH/karate-core/src/main/java/io/karatelabs/core/KaratePom.java.
4. **Live progress.** Always add `-f html,karate:jsonl`, then tail `<output>/karate-json/karate-events.jsonl`. It is flushed after each line.
   - Envelope: `{type, timeStamp, threadId, data}`.
   - Events written: `SUITE_ENTER` (`schemaVersion`, `version`, `env`, `threads`), `FEATURE_ENTER/EXIT`, `OUTLINE_ENTER`, `SCENARIO_ENTER/EXIT`, `ERROR`, `PROGRESS` (`completed`, `total`, `percent`), `SUITE_EXIT` (`summary{featureCount, featuresPassed, featuresFailed, scenarioCount, scenariosPassed, scenariosFailed, scenariosSkipped, passedRate, durationMillis, passed}`).
   - **Not written**: `STEP_*`, `HTTP_*`, `MATCH_EXIT`.
   - Scenario fields: `feature`, `slug`, `executionIndex`, `name`, `line`, `refId`, `callDepth`, `tags`, `isOutlineExample`, `exampleIndex`, `outlineSlug`, `passed`, `skipped`, `durationMillis`, `error` (leads with `path.feature:line`), `errorReason`.
   - Feature fields: `path`, `slug`, `name`, `tags`, `line`, `sourceHash`, `callDepth`. Called features also emit events, so **filter `callDepth == 0`** for the test tree.
   - `FEATURE_EXIT.data` holds the full `FeatureResult.toJson()`, with steps and HTTP logs.
   - Evidence: GH/karate-core/src/main/java/io/karatelabs/core/JsonLinesEventWriter.java; ScenarioRunEvent.java; FeatureRunEvent.java; D/running-tests/test-reports.
5. **Step-level live events** need a Java `RunListener` jar on the classpath, passed with `--listener com.x.Y` (or `karate --cp my.jar run ...` through the launcher). Such a listener can stream `STEP_*`, `HTTP_*`, and `MATCH_EXIT` over a socket or stdout, and can skip work by returning false on `*_ENTER`.
   - Evidence: GH/.../core/RunListener.java; RunEventType.java; D/advanced/java-api.
6. **Exit code**: `0` pass, `1` test failures, `2` config or usage error, `3` runtime error.
   - Evidence: GH/docs/CLI.md (Exit Codes); RunCommand.java.
7. **Results after the run.**
   - Show `karate-summary.html`, `feature-html/*.html`, or `karate-timeline.html` in a webview. They are self-contained static files.
   - Parse `junit-xml/*.xml` or `cucumber-json/*.json` if enabled.
   - Previous runs survive as sibling folders `karate-reports_yyyyMMdd_HHmmss` while `-B` is true. That gives run history for free.
   - Evidence: GH/.../output/HtmlReportWriter.java; GH/.../core/Suite.java.
8. **Console stream.** Use stdout and stderr for the live log pane. Colour is auto-off without a TTY; set `FORCE_COLOR=1` to keep ANSI. Tune output with `--log-console` (incl. `off`).
   - Evidence: D/running-tests/logging.
9. **Cancel.** Kill the process; no graceful-stop API is documented (unverified).
   - Mocks: Ctrl+C, or `GET /__admin/stop`.
   - Fail-fast via `configure abortSuiteOnFailure`.
   - Evidence: D/extensions/test-doubles; D/running-tests/parallel-execution.
10. **Mock servers.** Run `karate mock -m f.feature [-m ..] -p 0 [-W] [--ssl]` and parse `URL: http://localhost:<port>` from stdout.
    - Evidence: MockCommand.java.
11. **Clean.** `karate clean -o <dir>` clears reports and `karate-temp`.
    - Evidence: CleanCommand.java.
12. **Environment discovery.**
    - List `karate-config-*.js` files to offer `-e` choices.
    - Read `@env=` tags.
    - `karate doctor --json` gives machine-readable launcher diagnostics.
    - Evidence: D/core-syntax/configuration; karate-cli README.
13. **Tests and tags without running.** `-D` (dry run) plus JSONL lists every selected scenario with tags, lines, and expanded outlines, and skips config JS. Use it to fill the test tree.
    - Evidence: D/running-tests/command-line (Dry Run).
14. **Debugging (DAP).** Not in OSS.
    - `-Dkarate.debug.port=<port>` makes `Runner` delegate to `io.karatelabs.debug.Main` from the **commercial** `karate-ide` jar. Without the jar it logs a WARN and runs normally.
    - OSS fallbacks: `karate.stop(port)` pause and resume with `curl localhost:<port>`, and driver `stop:false`.
    - Evidence: GH/karate-core/src/main/java/io/karatelabs/core/Runner.java; D/extensions/ui-testing.
15. **Unsaved buffers.** `Runner.Builder.overlay(path, bytes)` runs edited, unsaved content. It is Java-API only, not exposed by the CLI.
    - Evidence: GH/docs/DESIGN.md.

### Gaps / unverified
- The docs disagree on dry-run step status: "skipped" (command-line page) vs "passed" (java-api page).
- Default of `--log-console`: the help text says `info`; the logging docs say unset leaves Logback alone.
- `karate mock -e`: in the docs but not in `MockCommand.java`.
- `karate init`, `karate perf`, and `karate config --show/--export`: listed in CLI.md as Rust, future, or TODO items. Not verified as shipped.
- The boolean tag syntax `(@a or @b) and not @c` is shown only for the Builder; CLI support is unconfirmed.
- No standalone Karate JSON result file in v2 beyond the JSONL; where cURL export is surfaced; the Gatling report path; a graceful cancel mechanism.
- I did not read every code sample on the 66 pages. Items were taken from headings, tables, and targeted sections. The HTTP, assertion, and UI keyword lists come from section headings and the api-reference page.
