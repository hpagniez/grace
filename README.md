<div align="center">

<img src="assets/mascot.svg" alt="Grace, a martial artist in a white gi and green belt, punching a moth with a strip of tape on its wing" width="240">

# Grace

### The free desktop test runner for Karate

Open a folder of [Karate](https://docs.karatelabs.io/) v2 tests, pick what to run, and watch it run.<br>
Every scenario, every failed step and every HTTP call, in one window.

[![Licence: MIT](https://img.shields.io/badge/licence-MIT-16a34a)](LICENSE)
[![Platform: Windows 11](https://img.shields.io/badge/platform-Windows%2011-0078d4)](#get-started)
[![Karate v2](https://img.shields.io/badge/Karate-v2-f97316)](https://docs.karatelabs.io/)
[![Java 21 bundled](https://img.shields.io/badge/Java%2021-bundled-64748b)](#how-it-works)
[![Status: release candidate](https://img.shields.io/badge/status-release%20candidate-8b5cf6)](https://github.com/hpagniez/grace/releases)

[Get started](#get-started) · [Features](#features) · [How it works](#how-it-works) · [Roadmap](#roadmap) · [Build from source](#build-from-source)

</div>

<br>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/images/run-dark.png">
  <img src="docs/images/run-light.png" alt="Grace after a run of nine tests: the test tree with tag chips on the left, the failed scenario's error and steps on the right, the console below">
</picture>

## Why Grace

Karate Labs sells its GUI runners. The Karate engine is open source, and its command line already does what a runner needs: it finds the tests, runs them in parallel, writes a live event stream and an HTML report. Grace is a desktop app on top of that command line.

- **Free, MIT licence.** No account, no licence key, no trial, no telemetry.
- **Nothing else to install.** A Java 21 runtime and the Karate jar ship inside the zip. Grace never uses the Java on your machine.
- **Your project stays clean.** Run output, history and reports go to Grace's own data folder, not into your project.

## Features

<table>
<tr>
<td width="50%" valign="top">

**Test tree**<br>
Folders, features, scenarios and each row of an outline's examples, found by a Karate dry run. Maven and Gradle projects with their compiled classes, standalone folders, and Karate CLI projects with a `karate-pom.json`. A Maven or Gradle project's libraries come from its own build (`mvnw` or `gradlew`, else the Maven or Gradle found on PATH, in `MAVEN_HOME`, `M2_HOME` or `GRADLE_HOME`, or the Maven that IntelliJ IDEA bundles), and jars in `.karate/ext/` or `~/.karate/ext/` are on the classpath, as with the Karate CLI. When the opened folder has no test, Grace offers the folders up to 3 levels below it that look like a Karate project. A new project starts from the environment, tags and threads in its `karate-pom.json`, or the `karate.env` of its `pom.xml`. The tree opens with only its folders: a feature opens when you click it or when its tests run. Arrow keys move through it, and it stays fast with 10,000 tests.

</td>
<td width="50%" valign="top">

**Run anything**<br>
The whole suite, a folder, a feature, one scenario or one example row. Cancel a run at any time.

</td>
</tr>
<tr>
<td valign="top">

**Tags**<br>
Click a tag to include it, click again to exclude it. Dim or hide the tests that do not match. Saved per project. With many tags, the filter bar keeps to one line and folds the rest.

</td>
<td valign="top">

**Environments and secrets**<br>
Switch `karate.env` and set run properties per environment, as Java system properties or as environment variables of the run. Import a `.properties` secrets file once, so it can leave your repository. Secret values are encrypted with your Windows account.

</td>
</tr>
<tr>
<td valign="top">

**Parallel runs**<br>
1 to 32 threads, saved per project and shown in the history.

</td>
<td valign="top">

**Live progress and console**<br>
Pass or fail on every node as the run goes, a progress bar, and Karate's log with errors and warnings in colour. Search the log and step through its errors.

</td>
</tr>
<tr>
<td valign="top">

**Failure details**<br>
The error, the file and line, and every step with its time, log and the `#` comments above it. An outline's examples as a table with each row's result. A failed `karate-config.js` is said once, and failures that share an error are grouped so you can step through them. Open the test or its failure in VS Code.

</td>
<td valign="top">

**HTTP inspector**<br>
Every call of the run, grouped by status class. Headers, bodies as pretty JSON or XML, timing, retries. Copy any call as cURL.

</td>
</tr>
<tr>
<td valign="top">

**History and reports**<br>
The last 20 runs of each project. Open Karate's HTML report of any of them, save it as a zip (with JUnit XML when `karate-pom.json` asks for it), re-run a run on its environment and tags, or re-run only what failed.

</td>
<td valign="top">

**Flaky tests**<br>
A test that passed in one run and failed in another, with the same project files and environment, gets a flaky mark and the runs that prove it.

</td>
</tr>
<tr>
<td valign="top">

**Updates in the app**<br>
Grace checks for a new version at startup. Update in one click, or let Grace update itself. Your data is backed up first, and you can roll back to an earlier version.

</td>
<td valign="top">

**Made for long sessions**<br>
Grace uses next to no CPU while it waits. During a run, Java works at below-normal priority with a 2 GB memory cap and spends little CPU compiling itself, so your IDE and browser stay responsive. A home screen with your recent folders, light and dark themes that follow Windows or your choice, resizable panels, bottom tabs you can reorder, and a window that opens where you left it.

</td>
</tr>
<tr>
<td valign="top">

**Step debugger**<br>
Set breakpoints on steps, start a debug run from any row, and when it pauses read the scenario's variables. Continue, step over or step into a called feature.

</td>
<td valign="top">

**Watch mode**<br>
Turn on **Watch** and each saved `.feature` file runs its tests that match your filter again. A save during a run waits for it to end. Saved per project.

</td>
</tr>
</table>

### Every HTTP call, by status

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/images/http-dark.png">
  <img src="docs/images/http-light.png" alt="The HTTP tab: thirteen calls grouped by status class, with the request and response of a 404 call in the details pane">
</picture>

### History and re-runs

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/images/history-dark.png">
  <img src="docs/images/history-light.png" alt="The History tab: two runs with their threads, what ran, the result and the duration, each with Report, Re-run and Re-run failed buttons">
</picture>

## Get started

1. Download `Grace-win32-x64-<version>.zip` from the [latest release](https://github.com/hpagniez/grace/releases).
2. Unzip it anywhere and start `Grace.exe`.
3. Click **Open folder…** and pick a folder that holds Karate `.feature` files.

Grace runs on Windows 11 (x64). The app is not code-signed yet (see the [code signing policy](#code-signing-policy)), so Windows SmartScreen can stop the first start: click **More info**, then **Run anyway**.

> [!NOTE]
> Every version is a release candidate of 1.0.0 for now. From 1.0.0-rc.6, Grace updates itself: click **Update** in the status bar, or open **Settings › Updates**. On 1.0.0-rc.5 or older, download the new zip instead. Settings and run history from an older release candidate may not carry over.

## Code signing policy

Free code signing provided by [SignPath.io](https://about.signpath.io/), certificate by [SignPath Foundation](https://signpath.org/). Signing starts once SignPath Foundation accepts Grace; until then, releases are unsigned.

- **What is signed:** `Grace.exe` in the release zip, built from this repository by the [`release` workflow](.github/workflows/release.yml). The bundled Java runtime keeps the Eclipse Foundation's signature; Electron's DLLs ship as Electron publishes them ([ADR 0024](docs/decisions/0024-sign-grace-exe-with-signpath-foundation.md)).
- **Committers and reviewers:** [hpagniez](https://github.com/hpagniez). Changes from anyone else come as pull requests they review.
- **Approvers:** [hpagniez](https://github.com/hpagniez) approves every release signing by hand.

### Privacy policy

Grace sends no data about you or your projects to anyone. It connects to other systems only:

- to check GitHub for a new version at startup (turn it off in **Settings › Updates**) or when you click **Check for updates**, and to download the version you choose;
- when your Karate tests call the systems they test.

## How it works

Grace drives the open-source Karate command line. It does not patch or replace Karate.

```mermaid
flowchart LR
  UI["Grace window"] <-->|"typed IPC"| Main["Electron main process"]
  Main -->|"starts, no shell"| Karate["Bundled Java 21<br/>+ karate.jar"]
  Karate -->|"karate-events.jsonl"| Main
  Karate -->|"HTML report"| Runs[("Run folder<br/>in Grace's data")]
```

1. **Discover.** A dry run lists every scenario and its tags without running them.
2. **Run.** Grace starts Karate with your selection, tags, environment and thread count.
3. **Follow.** Grace reads Karate's event stream as it is written, for live progress, and its output, for the console.
4. **Inspect.** After the run, Grace reads the HTTP calls from the step logs and keeps the run folder for the history.

[docs/inventory/karate-core.md](docs/inventory/karate-core.md) lists every open-source Karate v2 feature and how Grace uses it.

## Roadmap

Next on the list:

- **Trends**: pass rate and time per test over the stored runs ([#93](https://github.com/hpagniez/grace/issues/93)).
- **Called features in Details**: the steps of a called feature under the step that called it ([#234](https://github.com/hpagniez/grace/issues/234)).
- **Build from Grace**: build a project's Java test code from the "not built" notice ([#233](https://github.com/hpagniez/grace/issues/233)).
- **Import OpenAPI**: generate features from an OpenAPI spec ([#69](https://github.com/hpagniez/grace/issues/69)).
- **More project setup**: multi-module Maven and Gradle projects ([#64](https://github.com/hpagniez/grace/issues/64)).

The full backlog is one [issue](https://github.com/hpagniez/grace/issues) per feature, on the [Grace project](https://github.com/users/hpagniez/projects/2) board.

## Build from source

You need Windows 11 and Node 24.

```bash
npm install
npm run vendor
npm start
```

`npm run vendor` downloads the bundled Java runtime and Karate jar, once.

| Command | What it does |
|---|---|
| `npm start` | Run the app in dev mode |
| `npm test` | Unit tests (Vitest) |
| `npm run e2e` | Package the app, then run the Playwright end-to-end tests |
| `npm run make` | Build the Windows zip in `out/make/zip/win32/x64/` |

Grace is built with Electron, SolidJS 2, TanStack Router and Tailwind CSS v4.

## Built by AI agents

AI coding agents write most of this repository. Each feature starts as a spec with acceptance criteria in [docs/specs/](docs/specs/), each hard-to-reverse choice gets a decision record in [docs/decisions/](docs/decisions/), and each packaged release is checked and recorded in [docs/checks/](docs/checks/). [AGENTS.md](AGENTS.md) is the agents' brief: the commands, the git flow and the rules for driving Karate.

## The name

Grace is named after Grace Hopper, whose team taped a moth into a logbook in 1947 as the "first actual case of bug being found".

## Licence

Grace is under the [MIT licence](LICENSE).

Grace is not affiliated with or endorsed by Karate Labs Inc. Karate is a trademark of Karate Labs Inc.

## Bundled software

The Windows zip bundles programs that are not part of Grace, each under its own licence:

- [Electron](https://www.electronjs.org/), with Chromium and Node.js. The `LICENSE` and `LICENSES.chromium.html` files next to `Grace.exe` are Electron's.
- The [Eclipse Temurin](https://adoptium.net/) Java runtime, under the GPLv2 with the Classpath Exception; licence texts in `resources/licenses/temurin`. Its source is the repository and commit named by `SOURCE_REPO` and `SOURCE` in `resources/jre/release`.
- [Karate](https://github.com/karatelabs/karate), under the MIT licence in `resources/licenses/karate-LICENSE.txt`. The libraries inside `karate.jar` keep their own licences, in the jar's `META-INF`.
