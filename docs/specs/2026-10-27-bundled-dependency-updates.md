# Bundled dependency updates

Date 2026-10-27. Status: approved. Covers [feature](../features.md) F33.

## Intent

The app ships a Temurin 21 JRE, the Karate jar and Karate's licence file ([ADR 0006](../decisions/0006-bundle-jre-and-karate-jar.md)). Their URLs and SHA-256 are pinned by hand in `scripts/fetch-vendor.mjs`. Nobody is told when a new version comes out, and Dependabot cannot see these pins ([ADR 0007](../decisions/0007-github-actions-ci-cd-on-the-free-private-plan.md) records that they stay manual). A Java security fix or a Karate bug fix reaches users only when someone remembers to look.

The person wants these updates to arrive the way Dependabot's do: a PR shows up on its own, with the new pin and checksum, and CI proves the app still works before anyone merges it.

## Who and what success looks like

The maintainer, who reviews and merges PRs, mostly written by agents.

Success:
- Karate 2.1.5 is released. Within a week a draft PR "build(deps): bump karate from 2.1.4 to 2.1.5" is open on `develop`. It holds the new URL and SHA-256, the event fixtures recorded again with 2.1.5, and a note on whether Karate's `RunCommand.java` changed.
- The maintainer marks it ready for review. `checks` and `test`, e2e included, run on the bundled 2.1.5. Green means merge.
- Temurin 21.0.13 comes out. A separate draft PR bumps the JRE alone, so a Karate break never holds back a Java security fix.
- A week with no new version opens nothing.

## Goals, non-goals, anti-goals

**Goals:** the behaviour in the Requirements section below.

**Non-goals:**
- New major versions: Java 25 or a later LTS, Karate 3. They can change the CLI, the event file or the JVM options, so they stay a manual change. The workflow neither bumps nor reports them.
- Showing the bundled Java and Karate versions in the app (noted for F35 bug reports).
- Dependencies Dependabot already covers: Electron (and its Chromium), npm packages, GitHub Actions.
- Re-checking automatically that `RESERVED_KARATE_KEYS` (`src/shared/env.ts`) still lists every system property Karate reads. The PR body asks the reviewer to check it.
- A token secret or GitHub App so CI starts without a click (approach 2, not chosen).
- Merging on its own. The maintainer may add the `merge-when-green` label ([ADR 0011](https://github.com/Tusquito/karate-kid/pull/28), open) like on any PR.
- Other operating systems' JREs.

**Anti-goals** (failure even if everything else works):
- A bump PR whose CI tests the old JRE or jar: a stale cache, or `npm run vendor` keeping what it already unpacked.
- A pin whose checksum was not checked against the publisher's published value (Adoptium) or GitHub's digest of the uploaded asset (Karate).
- A bump to an older version, a pre-release, or a new major.
- A pile of duplicate PRs for the same version.
- A Karate bump merged with fixtures still recorded by the old Karate.

## Constraints

- No new dependency. The bump script uses Node 24 built-ins (`fetch`, `node:crypto`), like `fetch-vendor.mjs`.
- The repo is private on GitHub Free with no secrets. A PR opened with `GITHUB_TOKEN` starts no `pull_request` workflow (GitHub's rule against recursive runs). A person's "Ready for review" click is a new event made by that person, so it does start them.
- Repo setting today: `can_approve_pull_request_reviews: false` ("Allow GitHub Actions to create and approve pull requests" is off). With it off, `gh pr create` fails with `GITHUB_TOKEN`. The maintainer turns it on in Settings → Actions → General before the workflow can open PRs.
- Windows minutes are the scarce budget (ADR 0007): about 1,000 a month. Only the fixture re-recording needs Windows (bundled Windows JRE).
- Commits follow ADR 0003, and `scripts/check-pr.mjs` checks them on bump PRs too. Branches are `feature/*` into `develop` (git flow).
- `workflow_dispatch` and `schedule` workflows run only from the default branch (`develop`), so the workflow can be tried only once merged.

### What exists today

- `scripts/fetch-vendor.mjs` holds three pins as constants: `JRE` (`url`, `sha256`, `file: "jre.zip"`), `JAR` (`file: "karate-2.1.4.jar"`) and `LICENSE` (Karate's `LICENSE` at the tag, from raw.githubusercontent.com). `main()`:
  - skips the JRE when `vendor/jre/bin/java.exe` exists, whatever its version;
  - copies the jar only when `vendor/karate.jar` is missing, then `verifySha256` throws on a mismatch;
  - copies licence files into `vendor/licenses/` without clearing it first.
  So after a pin changes, `npm run vendor` keeps the old JRE without a word and fails on the old jar.
- `test.yml` and `release.yml` cache `vendor/` (minus `.download`) with the key `vendor-<os>-hashFiles('scripts/fetch-vendor.mjs')`.
- The app and the tests use fixed names: `vendor/jre`, `vendor/karate.jar` (`src/main/vendor.ts`, `forge.config.mts` `extraResource`). Nothing else depends on the version numbers.
- `scripts/capture-fixtures.mjs` (`npm run fixtures`) uses only Node built-ins and the vendored Java. It runs the bundled Karate on `test/fixtures/projects/` and the CLI spike project, sanitises paths (fixture root to `C:\fixture`, `tmpdir()` and `homedir()` to `C:\sanitised`), and throws if a line still holds `AppData` or the user name. It copies four spike samples as they are. `src/main/karate/http.test.ts` checks the recorded HTTP calls (F11 acceptance: 12 calls in `tests/http.feature`, 13 with `shared.feature`).
- `checks.yml` runs on `pull_request` with the default types (opened, synchronize, reopened), not on `ready_for_review`. `test.yml` runs on ready_for_review too, and runs e2e only when the PR is not a draft.
- Upstream data, checked 2026-10-27:
  - Adoptium: `GET https://api.adoptium.net/v3/assets/latest/21/hotspot?os=windows&architecture=x64&image_type=jre` returns the latest GA build with `release_name` (`jdk-21.0.12.1+1`), `version_data`, `binary.package.{name, link, checksum}`. The checksum is the SHA-256 of the zip and matches today's pin.
  - Karate: `GET https://api.github.com/repos/karatelabs/karate/releases` lists `v2.1.0` to `v2.1.4`, about one a month, each with `karate-<ver>.jar` and an asset `digest` of `sha256:<hex>`. The 2.1.4 digest matches today's pin. Karate 2.x has no fat jar on Maven Central (only the thin `karate-core`), so Dependabot's Maven support cannot track it.

## Approaches considered

1. **Our own weekly workflow that opens draft PRs.** Detects on Linux, re-records the fixtures on Windows only for a Karate bump, opens one draft PR per dependency. The maintainer's "Ready for review" click starts full CI. No secret. **Chosen by the person.**
2. The same workflow, with a fine-grained token or a GitHub App in the repo secrets, so CI starts when the PR opens. No click, but a write token to create, renew and keep safe.
3. The Renovate app with a regex rule over the pins. Its PRs start CI, but it cannot read the Adoptium checksum or re-record fixtures, and it is a third-party app with write access.

## Requirements

### R1. Pins in a data file

1. The three pins move from `fetch-vendor.mjs` into `scripts/vendor.json`, so a script can rewrite them without editing code. Each entry keeps `url`, `sha256` and `file`. The JRE and Karate entries also get `version`: the Temurin version as Adoptium writes it after `jdk-` (`21.0.12.1+1`), and Karate's without the `v` (`2.1.4`). The licence entry has no version: its URL names the Karate tag.
2. `fetch-vendor.mjs` reads `vendor.json` and behaves as today when the pins match what is in `vendor/`.
3. The `vendor/` cache keys in `test.yml` and `release.yml` hash `scripts/vendor.json` as well as `scripts/fetch-vendor.mjs`.

### R2. `npm run vendor` follows a changed pin

1. When `vendor/jre` was unpacked from a different zip than the pinned one, `npm run vendor` replaces it. It knows which zip by a marker outside `vendor/jre`, for example `vendor/.jre-sha256` holding the zip's SHA-256, written after a successful unpack. The marker stays out of the packaged app. A `vendor/jre` with no marker (unpacked before F33) counts as different, so it is replaced once.
2. When `vendor/karate.jar` does not match the pinned SHA-256, `npm run vendor` replaces it with the verified download instead of failing.
3. `vendor/licenses/` is rebuilt from scratch on each run, so no file from an older JRE or Karate stays.
4. Every download is still checked against its pin before use (`verifySha256`). A mismatch on a fresh download still fails.

### R3. The bump script

`node scripts/bump-vendor.mjs <jre|karate>`, also `npm run vendor:bump -- <jre|karate>`. Runs locally and in CI.

1. **JRE.** Reads Adoptium's latest GA Windows x64 JRE for Java 21 (endpoint above). Parses the version. If it is strictly newer than the pinned `version`, it rewrites the JRE entry: `version`, `url` = `binary.package.link`, `sha256` = `binary.package.checksum`. `file` stays `jre.zip`.
2. **Karate.** Reads Karate's GitHub releases. Keeps releases that are not drafts or pre-releases and whose tag matches `v2.<minor>.<patch>`. Picks the highest. If it is strictly newer than the pinned `version`, it rewrites:
   - the Karate entry: `version`, `url` and `file` (`karate-<ver>.jar`) from the asset, `sha256` from its `digest` (the part after `sha256:`);
   - the licence entry: `url` = `https://raw.githubusercontent.com/karatelabs/karate/v<ver>/LICENSE`, `sha256` computed from that download.
3. Versions compare as numbers, part by part, never as text (`2.1.10` > `2.1.9`; `21.0.13+11` > `21.0.12.1+1`). Same or older: no change. A different major is never picked.
4. It fails, with a message naming what is missing, and changes nothing when: an API call fails; the Adoptium answer has no `checksum` or `link`; the Karate release has no `karate-<ver>.jar` asset or no `sha256:` digest; a value is not the expected shape (64 hex characters for a SHA-256, an `https://` URL on the expected host).
5. It writes `vendor.json` back with the same layout (2-space JSON and a final newline, as oxfmt formats it), so the diff shows only changed values.
6. It reports the result on stdout and, when `GITHUB_OUTPUT` is set, as step outputs: `changed` (`true`/`false`), `from`, `to`.
7. GitHub API calls send `Authorization: Bearer $GITHUB_TOKEN` when that variable is set (rate limits). Locally they work without it.
8. For a Karate bump, it can write the PR body (R5.3): it reads `GET /repos/karatelabs/karate/compare/v<from>...v<to>` and says whether any changed file path ends with `/RunCommand.java`. If the compare lists 300 files (GitHub's cap), it says "unknown, check by hand" instead of "unchanged".

### R4. The workflow

`.github/workflows/vendor-updates.yml`.

1. Triggers: `schedule` once a week, and `workflow_dispatch`.
2. A concurrency group for the whole workflow with `cancel-in-progress: false`, so two runs never push at once and a run is never cut off between its push and its PR.
3. Permissions: `contents: write` (push the branch), `pull-requests: write` (open, comment on and close PRs). Nothing else.
4. JRE and Karate are handled separately: one failing never stops the other.
5. Detection runs on `ubuntu-latest`. Only a Karate bump uses `windows-latest`, to re-record the fixtures with the new bundled Java and jar (`npm run vendor`, then `npm run fixtures`). No `npm ci` is needed: both scripts use Node built-ins only.
6. On the Windows runner, `TEMP` and `TMP` point to `${{ runner.temp }}` for the fixture step. The runner's default temp folder is an 8.3 short path (`C:\Users\RUNNER~1\...`), which the fixture sanitiser may miss in its long form and then reject as a personal path.
7. For each bump, starting from the commit the run is on (`develop` for the schedule; the chosen branch for a dispatch, which the by-hand check uses):
   1. The branch is `feature/bump-<jre|karate>-<to>` (a `+` in a JRE version is kept or replaced as git and GitHub allow; the builder picks). If a PR from that branch exists in any state (open, closed or merged), stop: that version was handled. Closing a bump PR is how the maintainer skips a version; it never reopens, whether or not its branch is deleted.
   2. If the branch exists on the remote but has no PR (a run failed between push and PR, for example while the repo setting was off), skip to step 5 and open the PR on it.
   3. Otherwise create the branch. Commit `scripts/vendor.json` as `build(deps): bump temurin jre from <from> to <to>` or `build(deps): bump karate from <from> to <to>`. For Karate, commit the re-recorded `test/fixtures/events/` as a second commit, `test(fixtures): record events with karate <to>`. Every recording changes the files (time stamps, durations, the mock's random port), so this commit is always made.
   4. Commits are made as `github-actions[bot]`. Push the branch.
   5. Open a draft PR into `develop` with the first commit's header as its title and the body of R5.
   6. Older open bump PRs of the same dependency (head `feature/bump-<same dependency>-<v>` with `<v>` lower than `<to>`):
      - one that is still a draft and holds only `github-actions[bot]` commits is closed with the comment "Superseded by #<new>", and its branch deleted. Nothing of a person's is lost;
      - any other (marked ready, or with a commit by someone else, such as a parser fix) stays open. The new PR's body names it: "#<n> is still open and has changes by people: carry them over or close it."
      - A PR with a version equal to or higher than `<to>` is never touched.
8. A step that fails makes the run fail, which GitHub shows on the Actions tab and emails to the maintainer, as for any failed scheduled run.

### R5. The PR

1. It is a draft. It starts no CI on its own (Constraints).
2. Its body says, first: "Mark this PR ready for review to run `checks` and `test` (e2e included) on the new version. To skip this version, close this PR."
3. Karate PR body also has:
   - links to the release notes (`https://github.com/karatelabs/karate/releases/tag/v<to>`) and to the compare page;
   - whether `RunCommand.java` changed (R3.8), with the line: "Check the flags in AGENTS.md (Karate integration rules) against it.";
   - "Fixtures recorded again with Karate <to> (second commit; time stamps, durations and ports always differ). `src/main/karate/http.test.ts` checks the HTTP calls (F11).";
   - "Check that `RESERVED_KARATE_KEYS` in `src/shared/env.ts` still lists every system property Karate reads.".
4. JRE PR body also has a link to the Adoptium release (`https://github.com/adoptium/temurin21-binaries/releases/tag/jdk-<to>`, `+` encoded as `%2B`).

### R6. CI on the bump PR

1. `checks.yml` also runs on `ready_for_review`, so the maintainer's click starts both `checks` and `test`. On other PRs this only re-runs the Linux checks once when a draft is marked ready.
2. Nothing else changes in `test.yml`: the PR touches `scripts/`, so `test` runs, and the vendor cache misses because `vendor.json` changed (R1.3).

### R7. Docs

1. AGENTS.md, Pull requests and CI: the Dependabot line no longer says the pins are manual. It names `vendor-updates` in the workflow table, says its PRs open as drafts and run CI once marked ready, and says pins live in `scripts/vendor.json`. The Karate integration rule on `npm run fixtures` notes that the bump workflow already re-records them.
2. A new decision record, `docs/decisions/0012-weekly-draft-prs-for-the-bundled-jre-and-karate.md`: the workflow, its permissions (another workflow with `contents: write`, widening ADR 0007 as ADR 0011 does), the repo setting it needs, the draft-then-click CI start, the move of the pins to `vendor.json`. It replaces ADR 0007's "Dependabot does not update the JRE and Karate pins … they stay manual" consequence. ADR 0007 is not edited.
3. `docs/features.md`: F33 row to `specced` with this spec, then `done` when it ships.
4. A dated check record in `docs/checks/` for the by-hand checks (Acceptance criteria).

## Design decisions already made

- Updates arrive as PRs on their own, like Dependabot's (the person).
- Approach 1: our own workflow, draft PRs, the maintainer's "Ready for review" click starts CI, no secret (the person).
- Major versions stay manual (backlog notes).

The spec author's calls:
- Weekly, not monthly like Dependabot. Detection costs about a Linux minute a week, and a Java security fix arrives within a week of its release (R4.1).
- One PR per dependency (R4.4), so a Karate break never holds back a JRE fix.
- Supersede older bump PRs of the same dependency rather than updating them in place (R4.7.6). A branch per version is never force-pushed, and the PR's diff always matches its title. Only bot-only drafts with a lower version are closed, so the workflow never deletes a person's work and a by-hand test on an old pin never closes a real PR (builder check).
- A version counts as handled once any PR from its branch exists, in any state; a branch with no PR gets its PR on the next run (R4.7.1, R4.7.2). A failed PR creation heals itself, and closing a PR skips its version for good (builder check).
- A Karate bump gets a second commit for the fixtures (R4.7.3), because it is a separate logical change (ADR 0003). It is always made: no normaliser strips time stamps and ports to detect "no change". The tests on the new recordings are the proof, not the diff (builder check).
- R2 is checked by hand (Acceptance criteria), not by an automated test: a test would need `fetch-vendor.mjs` rebuilt around injected paths and downloads, and a fake JRE zip, for logic that runs once per bump (builder check).
- The pins move to a JSON file (R1.1) rather than having the script edit JavaScript with regular expressions.
- `npm run vendor:bump` exists locally too, so a person or agent can bump by hand with the same checks.
- A JRE marker file instead of reading `vendor/jre/release`: the marker ties the folder to the exact zip, not only to a version string.

## Left to the builder

- The schedule's day and hour.
- Job layout: one detect job with outputs and a job per dependency, or a job per dependency that detects itself. R4.4 and R4.5 must hold.
- Whether R3.8's body is written by the bump script or by the workflow from the script's outputs.
- The shape of the JRE marker (R2.1), as long as it is outside `vendor/jre` and `vendor/licenses`.
- Unit test layout for the script: version parsing and comparison, picking the release from an API payload, the rewrite of `vendor.json`, the failure cases of R3.4. Payloads are inline objects or small fixtures, never live network calls.

## Known risks

- The workflow only runs from `develop`, so its first real run is after merge. The by-hand check covers it (Acceptance criteria).
- Karate may change its step-log text. The re-recorded fixtures then make `http.test.ts` fail on the bump PR. That is the proof working: an agent or person fixes the parser on the bump branch.
- Fixture recording on the GitHub Windows runner has not run before. R4.6 covers the known path difference. Others will show as a failed run, not as a bad PR.
- Recording may be slow on the runner (it starts Java about seven times). It is counted against Windows minutes only for Karate bumps, about once a month.
- GitHub's asset `digest` field is fairly new. If Karate's releases stop carrying it, the script fails (R3.4) rather than pin an unchecked jar.
- The new checks run on `ready_for_review` costs one extra Linux run per PR.

## Acceptance criteria

```gherkin
Feature: Bundled dependency updates

  Background:
    Given scripts/vendor.json pins Temurin 21.0.12.1+1 and Karate 2.1.4

  Scenario: Nothing new, nothing opened
    Given Adoptium's latest Java 21 JRE is 21.0.12.1+1
    And Karate's newest 2.x release is v2.1.4
    When the vendor-updates workflow runs
    Then no branch is pushed and no PR is opened

  Scenario: A new Karate release opens a draft PR
    Given Karate v2.1.5 is released with karate-2.1.5.jar and its sha256 digest
    When the vendor-updates workflow runs
    Then a draft PR "build(deps): bump karate from 2.1.4 to 2.1.5" targets develop from feature/bump-karate-2.1.5
    And scripts/vendor.json pins karate-2.1.5.jar with the release's digest and the LICENSE at v2.1.5
    And the event fixtures are recorded again with Karate 2.1.5
    And the PR body says whether RunCommand.java changed

  Scenario: A new JRE opens its own PR
    Given Adoptium's latest Java 21 JRE is 21.0.13+11
    When the vendor-updates workflow runs
    Then a draft PR "build(deps): bump temurin jre from 21.0.12.1+1 to 21.0.13+11" targets develop
    And scripts/vendor.json pins its link and Adoptium's checksum
    And no Karate file changes in that PR

  Scenario: Marking the PR ready runs full CI
    Given a draft bump PR opened by the workflow
    When the maintainer marks it ready for review
    Then checks and test run on its head commit
    And test runs e2e with the newly pinned JRE and jar

  Scenario Outline: A handled version is not opened again
    Given a PR from feature/bump-karate-2.1.5 that is <state>
    And Karate v2.1.5 is still the newest release
    When the vendor-updates workflow runs
    Then no new PR is opened for Karate 2.1.5

    Examples:
      | state                         |
      | open                          |
      | closed, its branch kept       |
      | closed, its branch deleted    |

  Scenario: A pushed branch without a PR gets its PR
    Given the branch feature/bump-karate-2.1.5 exists with no PR
    When the vendor-updates workflow runs
    Then a draft PR from feature/bump-karate-2.1.5 is opened

  Scenario: A newer version supersedes a bot-only draft
    Given an open draft PR from feature/bump-karate-2.1.5 with only github-actions[bot] commits
    And Karate v2.1.6 is released
    When the vendor-updates workflow runs
    Then a draft PR from feature/bump-karate-2.1.6 is opened
    And the 2.1.5 PR is closed with "Superseded by #<new>" and its branch deleted

  Scenario: An older bump PR with a person's work stays open
    Given an open PR from feature/bump-karate-2.1.5 with a commit by the maintainer
    And Karate v2.1.6 is released
    When the vendor-updates workflow runs
    Then a draft PR from feature/bump-karate-2.1.6 is opened and its body names the 2.1.5 PR
    And the 2.1.5 PR stays open with its branch

  Scenario Outline: Versions that are never picked (unit)
    Given the pinned Karate version is 2.1.4
    When the newest release is <release>
    Then the pin does not change

    Examples:
      | release                     |
      | v2.1.4                      |
      | v2.1.3                      |
      | v3.0.0                      |
      | v2.2.0 marked as prerelease |
      | v2.2.0.RC1                  |

  Scenario: Numbers compare as numbers (unit)
    Given the pinned Karate version is 2.1.9
    When the newest release is v2.1.10
    Then the pin moves to 2.1.10

  Scenario Outline: Missing data fails and changes nothing (unit)
    Given <missing>
    When the bump script runs
    Then it exits with an error naming what is missing
    And scripts/vendor.json is unchanged

    Examples:
      | missing                                                |
      | the Adoptium answer has no checksum                    |
      | the Karate release has no karate-<ver>.jar asset        |
      | the jar asset has no sha256 digest                     |
      | the releases API answers 500                           |

  Scenario: npm run vendor follows a changed pin (by hand)
    Given vendor/ holds the JRE and jar of the old pins
    When the pins change and npm run vendor runs
    Then vendor/jre is unpacked from the new zip
    And vendor/karate.jar matches the new SHA-256
    And vendor/licenses holds only the new licence files

  Scenario: The bump script reproduces today's pins (by hand)
    Given scripts/vendor.json is set back to Karate 2.1.3 and an older JRE
    When "npm run vendor:bump -- karate" and "npm run vendor:bump -- jre" run
    Then scripts/vendor.json equals today's pins exactly

  Scenario: First real run after merge (by hand)
    Given vendor-updates.yml is on develop and the repo allows Actions to create PRs
    And a throwaway branch pins Karate 2.1.3
    When the workflow is dispatched on that branch
    Then it opens a draft PR bumping Karate to 2.1.4 with fixtures recorded on the Windows runner
    And marking it ready runs checks and test green
    And the maintainer closes that PR without merging
```
