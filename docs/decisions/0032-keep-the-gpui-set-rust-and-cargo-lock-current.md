---
status: accepted
date: 2026-11-10
decision-makers: Tusquito
---

# Keep the GPUI set, Rust and Cargo.lock on their newest releases

## Context and Problem Statement

GPUI Grace pins `gpui-pre`, GPUI Kit and the Rust toolchain exactly, and carries two patches to `gpui-pre-windows` as a patched copy under `[patch.crates-io]` ([ADR 0031](0031-move-grace-to-rust-and-gpui.md), [patches/](../../patches/README.md)). The [migration guide](../specs/2026-11-08-gpui-migration.md) let these pins move only between phases (R9.4). `gpui-pre` and GPUI Kit release every Monday and Rust every six weeks. On 2026-11-10 Grace was one release behind `gpui-pre` (0.3.11, 0.3.12 out) and GPUI Kit (0.9.0, 0.9.1 out), and two behind Rust (1.97.0, 1.99.1 out). Nothing reported it: Dependabot watches npm and GitHub Actions, not Cargo or `rust-toolchain.toml`.

The owner's rule, 2026-11-10: everything is always up to date, and tested with the newest upstream changes against our patches and versions. A patch rebased only at the end of a phase finds out late that upstream broke it, or made it unneeded.

How do new releases reach `develop` as tested PRs, with the patches rebased on them?

Spec: [docs/specs/2026-11-10-cargo-updates.md](../specs/2026-11-10-cargo-updates.md).

## Considered Options

1. Dependabot's `cargo` ecosystem.
2. Renovate.
3. Our own weekly workflow, as `vendor-updates` ([ADR 0012](0012-weekly-draft-prs-for-the-bundled-jre-and-karate.md)): it moves the set, rebases the patches, updates the lock, opens a draft PR and starts `test` on it.

## Decision Outcome

Chosen: option 3. Dependabot and Renovate would move the `gpui-pre` pins without rebuilding the patched copy, so that `patches/check.sh` fails and Cargo builds an unpatched crate, and they do not know that GPUI Kit pins one `gpui-pre` exactly. They would also move the crates Grace pins with `=` to the copy GPUI already brings (`windows`, `libc`, `x11rb`, `atspi`), putting a second copy in the exe.

- `scripts/bump-cargo.mjs` (`npm run cargo:bump`) reads crates.io's index and Rust's stable channel. It takes the newest GPUI Kit (`gpui-component`, `gpui-base` and `gpui-kit-assets` at one version) and the `gpui-pre` that Kit pins exactly; a newer `gpui-pre` waits for a Kit that takes it. It downloads that `gpui-pre-windows`, checks it against crates.io's checksum, applies each patch of `patches/series` in order, writes each patch again as `git diff` writes it, and sets `check.sh`'s version and checksum. It moves `rust-toolchain.toml` to the newest stable Rust, then runs `cargo update`, which moves every crate in `Cargo.lock` to the newest release its requirement allows. Its report names what moved, with Zed's, GPUI Kit's and Rust's changes; the hunks `patch` applied with fuzz; the dependencies of Grace held behind a newer release; and the `=` pins the update brought a newer copy of into the lock.
- A patch that does not apply, or that the new release already has, stops only the GPUI set: an issue says which and why, and the toolchain and the lock still move.
- `.github/workflows/cargo-updates.yml` runs it every Tuesday at 06:00 UTC, after Monday's `gpui-pre` and GPUI Kit, and by hand. Its one Linux job holds `contents`, `pull-requests`, `issues` and `actions: write`. It runs Cargo's resolver only, never the new releases' code. It commits on `feature/bump-cargo-<date>`, opens a draft PR into `develop`, closes older bump drafts that hold only the bot's commits, and dispatches `test.yml` on the branch: a run started with `GITHUB_TOKEN` through `workflow_dispatch` runs, where a PR opened with it starts nothing (ADR 0012). That run builds and tests the new releases with the patches on Linux and Windows: clippy, `cargo test`, `patches/check.sh`, GPUI Grace's e2e on both, and `gpui-idle`, which measures the minimised wake-ups on Windows with `npm run bench:gpui` and fails above 5 a second.
- `checks` still starts when a person marks the PR ready for review.
- The patch files keep their names across versions; `patches/series` lists them in order, and only `check.sh` names the version.
- Replaces the migration guide's R9.4 "upgraded only between phases": the pins follow the newest releases, as one set. A bump that collides with a phase's open PRs merges after them.
- Direct dependencies that a new major leaves behind (`cargo update` cannot move them) are listed in the PR, not bumped: most are held at GPUI's copy on purpose.

### Consequences

- Good: a new `gpui-pre`, GPUI Kit or Rust reaches `develop` as a tested PR within a week, and a patch that upstream breaks or makes unneeded shows up that week.
- Good: every bump runs GPUI Grace's Windows e2e and minimised idle, which a PR into `develop` runs only by hand.
- Bad: a PR a week to review and merge, often with a few fixes: Rust 1.99 flagged 15 redundant imports, a new GPUI may change an API.
- Bad: about 30 Linux and 120 Windows minutes of Actions a week (free on a public repository). The `gpui-idle` job also runs on every other run of `test.yml` by hand.
- Bad: a CI VM has no GPU, so `gpui-idle` proves the patch still puts GPUI's vsync thread to sleep, not the reference machine's numbers. Those still need a person, before a release.
- Neutral: the workflow runs only from the default branch, so its first real run is after merge.
