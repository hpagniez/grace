# Offer the sub-folder that holds the Karate project

Date 2026-11-04. Status: draft. Covers [#232](https://github.com/hpagniez/grace/issues/232), a later slice of [#64](https://github.com/hpagniez/grace/issues/64). Narrows R4.1 of [Project libraries](2026-11-02-project-libraries.md).

## Intent

Some repositories keep their Karate project in a sub-folder, beside files of other tools. Seen on a work repository: its root holds a `pom.xml` for another test tool, and its Karate project is in `karate/`. Opening the root, Grace reads a Maven project with no `src/test`, shows an empty tree, and first runs Maven on that `pom.xml` to list libraries no test uses. The person has to guess that they opened the wrong folder.

When the opened folder holds no test, Grace should look a little way down for folders that look like a Karate project, offer each one in the empty tree ("Open karate/"), and open the one picked like any project.

## Goals, non-goals, anti-goals

- Goals: an empty tree offers the Karate projects a few folders down; picking one opens it like any project (recent list, saved settings); a folder with nowhere to look for tests runs no build tool.
- Non-goals: multi-module builds, where a module's tests need a sibling module (the [project setup](2026-11-02-project-setup.md) and [project libraries](2026-11-02-project-libraries.md) specs keep them out); running the tests of several sub-folders as one project; offering a sub-folder when the opened folder has tests; a setting for the depth.
- Anti-goals: any cost when the opened folder has tests; a walk that slows down opening a big repository; opening a folder the window names that Grace did not offer.

## Constraints

- Follow the Karate CLI first (ADR 0026): the CLI has no such search. Run in a folder with no tests, it finds 0 scenarios. This is Grace's own way, on top of the CLI's.
- Idle costs nothing (ADR 0023): the walk runs once per empty discovery, never on a timer.
- The renderer sends no path Grace did not give it, as with the recent list.

### What exists today

- `detectLayout` (`src/main/karate/command.ts`) reads a folder with `pom.xml` as Maven and one with `build.gradle(.kts)` as Gradle. Their feature roots are whichever of `src/test/java` and `src/test/resources` exist, or the `karate-pom.json` `paths`. A Maven folder with neither has no feature roots, and `discover` (`src/main/runner.ts`) builds an empty tree without a dry run.
- `open` and `refresh` list the libraries of a Maven or Gradle folder before the dry run (`listLibraries`), whatever its feature roots.
- `discover` publishes a `project` event: project, tree, error. Main keeps the last one and replays it to a window that reloads.
- The tree with no test shows `EmptyState` "No feature files found" and "Grace looks for .feature files under the project folder." (`src/renderer/Tree.tsx`).
- `SKIPPED_DIRS` (`src/main/fingerprint.ts`) lists `.git`, `node_modules` and `.gradle`.

## Approaches considered

1. **The offer on the `project` event. Chosen by the owner.** Main walks before it publishes an empty tree and adds the folders found to that event, only when it found some. The empty tree and its offer show together, and a reloaded window gets the offer back with the replayed event. Tests that compare the event do not change.
2. **A separate event after the tree.** The `project` event stays as it is, but the empty tree shows a moment before its offer, and the new event needs its own clearing when another project opens and its own replay.

## Requirements

### R1. When Grace looks

1. After a discovery whose tree has no test and no error, on open, Refresh or an environment switch, Grace looks for Karate projects below the open project's folder. This includes a project with no feature roots, which gets no dry run.
2. A discovery with tests, one that failed, or one cancelled starts no walk.

### R2. What Grace looks for

1. Folders 1 to 3 levels below the project's folder; a folder directly in it is level 1. The project's folder itself is never offered.
2. A folder looks like a Karate project when it holds `karate-pom.json` or `karate-config.js`, or holds `pom.xml`, `build.gradle` or `build.gradle.kts` beside a `src/test` folder with a `.feature` file anywhere under it.
3. Grace does not look inside a folder it offers.
4. Grace skips folders whose name, in any case, is in `SKIPPED_DIRS`, `target` or `build`, at any level and under `src/test` too. It follows no link or junction.
5. Grace stops at 10 folders found, after reading 2,000 folders, or after 2 seconds, and offers what it found by then. A folder it cannot read is skipped.
6. The folders are ordered by level, then by name.

### R3. The offer

1. The empty tree keeps its two lines and shows, below them, one button per folder found: `Open <folder>/`, the folder relative to the project's folder with `/` between names, for example `Open karate/` or `Open tests/api/`.
2. When Grace found none, the empty tree is as today.
3. A tree with tests never shows the offer.

### R4. Opening an offered folder

1. Picking a folder opens it as the folder dialog would: Grace discovers its tests, puts it first in the recent list, saves it as the last project, and loads its own saved filter, environment, threads and watch choice.
2. The window sends only the folder as shown, relative to the project. Grace opens it only if it is one it offered with the tree last sent; otherwise it shows the notice "Invalid request" and opens nothing.
3. While Grace is busy (a run, a discovery), picking a folder is refused as any open is.

### R5. No library listing with nowhere to look for tests

1. On open and Refresh, Grace does not start Maven or Gradle to list the libraries of a project that has no feature roots: no `src/test/java`, no `src/test/resources`, and no `karate-pom.json` `paths` kept. It shows no library notice for it. This narrows project libraries R4.1.
2. Opening a sub-folder with feature roots lists its libraries as project libraries R4 says.

### R6. Docs

1. The README's project line says that when the opened folder has no test, Grace offers the folders below it that look like a Karate project.

## Cost

- **Idle:** nothing. No timer, poll or watcher; the offer is a few strings in the last `project` event.
- **Open, Refresh or environment switch with tests:** nothing new. No walk runs.
- **Discovery with no test:** one asynchronous walk on the main process, at most 3 levels down and under each `src/test` found, bounded to 2,000 folders read and 2 seconds. Each folder costs one `readdir`. A typical repository has tens of folders in 3 levels: a few milliseconds. The main thread never blocks.
- **Saved:** a Maven or Gradle folder with nowhere to look for tests no longer starts its build tool on open and Refresh (R5): seconds to a minute of Java and its memory, and the network if Maven downloads.
- **Memory:** at most 10 relative paths. **Start time:** unchanged. **Size:** about 2 KB of code; no dependency.

## Design decisions already made

By the owner:

- Look 3 levels down (R2.1).
- Skip the library listing when the folder has nowhere to look for tests, not only when it has no `src/test` (R5.1): a `karate-pom.json` with `paths` elsewhere still gets its libraries.
- The offer rides on the `project` event (approach 1).

The builder's calls, open to the owner's correction:

- At most 10 folders, 2,000 folders read, 2 seconds (R2.5): the same time bound as the fingerprint walk. More than 10 Karate projects in one repository is a multi-module case this spec leaves out.
- `target` and `build` are skipped everywhere, not only beside a build file as the fingerprint does (R2.4): compiled copies such as `target/test-classes/karate-config.js` would otherwise look like projects.
- The walk also runs after an environment switch or Refresh that comes back empty (R1.1): one code path for every discovery, and it costs only when there is no test.
- A dry run that failed shows its error and no offer (R1.2): the folder has tests Karate could not read, so the fix is in the folder, not below it.

## Left to the builder

- Where the walk lives: a function in `src/main/karate/` beside `detectLayout`, returning relative paths with `/`.
- The `project` event gets an optional `subfolders` field, set only when Grace found some; the renderer's state keeps it, cleared by every `project` event without it.
- A new IPC call for R4 checked against the last `project` event, and a `Cancel` that arrives during the walk is honoured before the tree is published.

### Tests

- Unit tests of the walk: each marker of R2.2, the depth limit, skipped folders, no look inside an offered folder, links not followed, the found and folder limits, the order.
- Runner unit tests: an empty discovery publishes `subfolders`, a discovery with tests does not; opening an offered folder opens it, opening one not offered is refused; no build tool starts for a Maven folder with no feature roots.
- A renderer unit test of the empty tree with and without folders.
- One e2e test on the packaged app: a folder with a `pom.xml` and a `karate/` project; the tree offers `Open karate/`; a click shows `karate/`'s tests and puts it first in the recent list.

## Acceptance criteria

```gherkin
Feature: Offer the sub-folder that holds the Karate project

  Background:
    Given a folder "repo" holding a "pom.xml" and no "src" folder
    And "repo/karate" holding "karate-config.js" and "tests/basic.feature" with 2 scenarios

  Scenario: The empty tree offers the Karate project below
    When the person opens "repo"
    Then the tree says "No feature files found"
    And it shows the button "Open karate/"
    And Grace did not start Maven

  Scenario: Picking the offer opens the sub-folder as a project
    Given the person opened "repo"
    When they click "Open karate/"
    Then the tree shows "basic.feature" with its 2 scenarios
    And "repo/karate" is first in the recent list

  Scenario Outline: What looks like a Karate project
    Given "repo/<folder>" holds <files>
    When the person opens "repo"
    Then the tree <offers> "Open <folder>/"

    Examples:
      | folder        | files                                                | offers         |
      | api           | "karate-pom.json"                                    | offers         |
      | gradle-tests  | "build.gradle" and "src/test/java/a/b.feature"       | offers         |
      | maven-tests   | "pom.xml" and "src/test/resources/c.feature"         | offers         |
      | tool          | "pom.xml" and "src/test/java/Helper.java" only       | does not offer |
      | a/b/c         | "karate-config.js"                                   | offers         |
      | a/b/c/d       | "karate-config.js"                                   | does not offer |
      | node_modules/x | "karate-config.js"                                  | does not offer |
      | target/test-classes | "karate-config.js"                             | does not offer |

  Scenario: Grace does not look inside an offered folder
    Given "repo/karate/nested" holds "karate-pom.json"
    When the person opens "repo"
    Then the tree shows "Open karate/" only

  Scenario: A folder with tests gets no offer
    Given "repo/src/test/java/x.feature" with 1 scenario
    When the person opens "repo"
    Then the tree shows "x.feature"
    And it shows no "Open" button

  Scenario: A failed dry run gets no offer
    Given "repo/src/test/java/broken.feature" that does not parse
    When the person opens "repo"
    Then the tree shows the dry run's error
    And it shows no "Open" button

  Scenario: Grace opens only a folder it offered
    Given the person opened "repo"
    When the window asks to open "../elsewhere"
    Then Grace shows the notice "Invalid request"
    And "repo" stays open

  Scenario: A Maven project with nowhere to look for tests lists no libraries
    Given a Maven folder with no "src/test" and no "karate-pom.json"
    When the person opens it, or refreshes it
    Then Grace does not start Maven
    And no library notice shows
```
