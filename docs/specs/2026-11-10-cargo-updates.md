---
status: built
issue: 328
summary: "Open a PR on its own when a new GPUI, GPUI Kit or Rust release comes out, tested with the patches, and merge bump PRs once tests pass"
---

# Cargo updates

Date 2026-11-10. Approved by the owner's rule of 2026-11-10 ("everything must always be up to date and tested with the latest changes against our patches and versions"). Decision: [ADR 0032](../decisions/0032-keep-the-gpui-set-rust-and-cargo-lock-current.md). Updated 2026-11-11 by the owner's rule that bump PRs merge on their own once all tests pass: [ADR 0033](../decisions/0033-merge-the-weekly-cargo-bump-prs-on-green.md), requirements 6 and 9.

## Intent

GPUI Grace pins `gpui-pre`, GPUI Kit and Rust exactly and carries patches to `gpui-pre-windows` ([patches/](../../patches/README.md)). New releases of all three come out often, and nothing reported them. The owner wants them on `develop` as soon as they are out, tested with the patches, the way `vendor-updates` brings the JRE and Karate ([bundled dependency updates](2026-10-27-bundled-dependency-updates.md)), and merged with nobody's action once every test passes.

## Who and what success looks like

The maintainer, who reviews and merges PRs, mostly written by agents.

- `gpui-pre` 0.3.13 and GPUI Kit 0.9.2 come out on a Monday. On Tuesday a PR "build(deps): bump gpui-pre 0.3.13, gpui kit 0.9.2" is opened on `develop`, with the patched copy rebuilt from 0.3.13, both patches written again, `check.sh` and `Cargo.lock` moved, and `checks`, `test` and `perf` run on it, Windows e2e and minimised idle included. When all pass, it is merged that morning.
- A test fails on the new release. The PR stays open with a comment that links the failed run, and the maintainer is told by the failed `cargo-updates` run.
- Upstream changes the code a patch touches. The PR still comes, with Rust and the lock only, and an issue names the patch and its rejected hunks.
- Upstream fixes what a patch did. The issue says that the release already has it, so the patch can go.
- A week with nothing new opens nothing.

## Requirements

1. `scripts/bump-cargo.mjs` moves the pins of `gpui-pre` and `gpui-pre-platform` to the `gpui-pre` the newest GPUI Kit pins exactly, and `gpui-component`, `gpui-base` and `gpui-kit-assets` to that Kit, never backwards. It reports a newer `gpui-pre` that no Kit takes yet.
2. It rebuilds `patches/gpui-pre-windows` from crates.io's `.crate` of that version, refusing one whose SHA-256 is not the index's, with the files `patches/README.md` names left out and each patch of `patches/series` applied in order. Each patch is written again as `git diff` writes it, and `check.sh` gets the version and checksum.
3. A patch that does not apply, or that the release already has (it applies in reverse), changes nothing in `patches/` or the pins. The PR says why, and an open issue titled "Rebase the GPUI patches on gpui-pre X" is reused or opened.
4. It moves `rust-toolchain.toml` to the newest stable Rust, then runs `cargo update`, and fails when `Cargo.lock` does not take `gpui-pre-windows` from `patches/`.
5. The PR body lists what moved with links to Zed's compare (from the crate's `zed-rev`), GPUI Kit's notes and compare, and Rust's notes; the hunks applied with fuzz; the lock's moves; Grace's dependencies held behind a newer release; and its `=` pins the update brought a newer copy of into the lock.
6. `cargo-updates.yml` runs it every Tuesday and by hand. With changes, it commits on `feature/bump-cargo-<date>` as `github-actions[bot]`, opens a PR into `develop`, ready for review and `Part of #328`, closes older bump PRs that hold only the bot's commits (and names any other in the new PR), and dispatches `checks.yml`, `test.yml` and `perf.yml` on the branch. A branch that already has a PR is left alone.
7. `test.yml` on a run by hand adds `gpui-idle`: `npm run bench:gpui` on Windows, its table in the job summary, failing when `idle.wakeups.minimised` is over 5 a second.
8. The commit title passes `scripts/check-pr.mjs`.
9. `cargo-updates.yml`'s `merge` job then waits for the newest run by hand of each of those three workflows on the PR's commit. When all three passed, it comments their links on the PR and merges it with a merge commit, deleting the branch, only if the PR is still on that commit. A failed or cancelled run, a refused merge, or 110 minutes without all three done, leaves the PR open with a comment that says why, and fails the job. `checks.yml` runs by hand as on a PR into `develop`.

## Acceptance criteria

```gherkin
Feature: Cargo updates

  Scenario: a new GPUI set and Rust
    Given Grace pins gpui-pre 0.3.11, GPUI Kit 0.9.0 and Rust 1.97.0
    And GPUI Kit 0.9.1 pins gpui-pre =0.3.12, and Rust 1.99.1 is the newest stable
    When cargo-updates runs
    Then a PR "build(deps): bump gpui-pre 0.3.12, gpui kit 0.9.1, rust 1.99.1" is open on develop, ready for review
    And patches/gpui-pre-windows is crates.io's 0.3.12 with every patch of patches/series
    And patches/check.sh passes
    And checks, test.yml with gpui-idle, and perf run on its branch

  Scenario: every test passes
    Given cargo-updates opened a bump PR and started checks, test and perf on its commit
    When all three end in success
    Then a comment on the PR links the three runs
    And the PR is merged into develop with a merge commit, and its branch deleted

  Scenario: a test fails
    Given cargo-updates opened a bump PR and started checks, test and perf on its commit
    When test ends in failure
    Then the PR stays open
    And a comment on the PR names test and links its run
    And the cargo-updates run fails

  Scenario: a push after the runs
    Given all three runs passed on the bump PR's first commit
    And a person pushed a fix to its branch since
    When the merge job merges
    Then GitHub refuses the merge, and the PR stays open with a comment that says why

  Scenario: a patch no longer applies
    Given gpui-pre 0.3.13 changes the lines the idle patch touches
    When cargo-updates runs
    Then the GPUI set and patches/ stay as they are
    And an issue "Rebase the GPUI patches on gpui-pre 0.3.13" holds the rejected hunks
    And the PR moves Rust and Cargo.lock only, and names the issue

  Scenario: upstream has the fix
    Given gpui-pre-windows 0.3.13 already holds the idle patch's change
    When cargo-updates runs
    Then the issue says that the release already has the idle patch, and to drop it

  Scenario: GPUI Kit is behind gpui-pre
    Given gpui-pre 0.3.13 is out and the newest GPUI Kit pins =0.3.12
    When cargo-updates runs
    Then Grace stays on gpui-pre 0.3.12
    And the PR says that gpui-pre 0.3.13 waits for GPUI Kit

  Scenario: nothing is new
    Given every pin is the newest release and cargo update moves nothing
    When cargo-updates runs
    Then no branch, PR or issue is made

  Scenario: the idle patch breaks
    Given a bump in which GPUI's vsync thread wakes on every vsync while minimised
    When test.yml runs on its branch
    Then gpui-idle fails, with the wake-ups a second in its error
```

The rebase, the picking of the set, the pins, the report, the title, the PR's opening and the merge on green are tested in `scripts/bump-cargo.test.ts`. The workflow runs only from the default branch, so its first run is after merge.

## Cost

The workflow adds nothing to the app: no memory, CPU, start or size, idle or during a run. Each bump it opens may move GPUI Grace's numbers; its `gpui-idle` run reports size, start, idle and minimised wake-ups on a CI VM, and the reference machine's bench still runs before a release. CI only: a few Linux minutes a week for the bump, and on each bump a `test` run by hand of about 30 Linux and 120 Windows minutes, `gpui-idle`'s 30 included, a `perf` run of about 20 Windows minutes, a `checks` run of a minute, and the `merge` job's wait of 30 to 60 Linux minutes (free on a public repository).
