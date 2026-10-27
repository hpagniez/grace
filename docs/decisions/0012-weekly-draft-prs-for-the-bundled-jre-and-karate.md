---
status: accepted
date: 2026-10-27
decision-makers: Tusquito
---

# Open weekly draft PRs for a newer bundled JRE or Karate jar

## Context and Problem Statement

The app bundles a Temurin 21 JRE, the Karate jar and Karate's licence ([ADR 0006](0006-bundle-jre-and-karate-jar.md)). Their URLs and SHA-256 were constants in `scripts/fetch-vendor.mjs`, bumped by hand. Dependabot cannot see them: Adoptium is no Dependabot ecosystem, and Karate 2.x has no fat jar on Maven Central. [ADR 0007](0007-github-actions-ci-cd-on-the-free-private-plan.md) accepted that "they stay manual", so nobody is told when a Java security fix or a Karate release comes out.

The repo is private on GitHub Free with no secrets. A PR opened with `GITHUB_TOKEN` starts no `pull_request` workflow (GitHub blocks recursive runs), so a bot PR gets no CI on its own. How do these updates arrive like Dependabot's, with CI proving the app still works?

Spec: [docs/specs/2026-10-27-bundled-dependency-updates.md](../specs/2026-10-27-bundled-dependency-updates.md).

## Considered Options

1. Our own weekly workflow that opens a draft PR per dependency. The maintainer's "Ready for review" click is a new event made by a person, so it starts `checks` and `test`. No secret.
2. The same workflow with a fine-grained token or a GitHub App in the repo secrets, so CI starts when the PR opens.
3. The Renovate app with a regex rule over the pins. Its PRs start CI, but it cannot read the Adoptium checksum or re-record the fixtures, and it is a third-party app with write access.

## Decision Outcome

Chosen: option 1. One click per bump, no write token to create, renew or keep safe.

- Pins live in `scripts/vendor.json` (`jre`, `karate`, `license`: `version` where it applies, `url`, `sha256`, `file`), so a script rewrites data, not code. `npm run vendor` replaces a JRE unpacked from another zip (marker `vendor/.jre-sha256`) and a jar that does not match its pin. The `vendor/` cache keys hash `vendor.json` too.
- `scripts/bump-vendor.mjs` (`npm run vendor:bump -- <jre|karate>`) moves one pin to the newest release of the same major: Adoptium's latest GA Windows x64 JRE for Java 21, with Adoptium's checksum; Karate's newest non-draft, non-pre-release `v2.x.y`, with GitHub's `sha256:` asset digest and the LICENSE at that tag. Versions compare as numbers. Missing data fails and writes nothing.
- `.github/workflows/vendor-updates.yml` runs every Monday and by hand. The JRE job and the Karate detection run on Linux. Only a Karate bump uses Windows, to record the event fixtures again with the new bundled Java and jar (`npm run fixtures`).
- Permissions: the workflow default is `contents: read`. Only the Linux jobs that push and open PRs get `contents: write` and `pull-requests: write`, widening ADR 0007's rule as ADR 0011 does. The Windows job runs upstream binaries nobody has reviewed yet, so it keeps the read-only token and no git credentials (`persist-credentials: false`). It hands the recorded `*.jsonl` files to a Linux job as an artifact kept one day, an exception to ADR 0007's "no other artifacts" (a few hundred KB). That job bumps the pin itself, refuses to go on if the newest version changed in between, copies only the fixture files the repo already has, then commits and opens the PR.
- `scripts/bump-pr.mjs` names the branch `feature/bump-<jre|karate>-<version>` (`+` as `_`). A version is handled once any PR from its branch exists, in any state: closing a bump PR skips that version. A branch pushed without a PR gets its PR on the next run. The new PR is a draft into `develop`, with a checklist (release notes, whether `RunCommand.java` changed, `RESERVED_KARATE_KEYS`). Older bump drafts of the same dependency that hold only `github-actions[bot]` commits are closed as superseded; any other is left open and named in the new PR.
- `checks.yml` also runs on `ready_for_review`, so the click starts both workflows.
- The repo setting "Allow GitHub Actions to create and approve pull requests" must be on, or `gh pr create` fails.
- New majors (Java 25, Karate 3) stay manual: they can change the CLI, the event file or the JVM options.

### Consequences

- Good: a new JRE or Karate release reaches `develop` as a reviewed, tested PR within a week, with its checksum from the publisher.
- Good: a Karate bump carries fixtures recorded by that Karate, so the parser tests prove the step-log format still parses.
- Good: replaces ADR 0007's "Dependabot does not update the JRE and Karate pins … they stay manual". ADR 0007 is not edited.
- Bad: one click per bump before CI runs.
- Bad: the workflow runs only from the default branch, so its first real run is after merge.
- Bad: about one Linux minute per job a week, plus about 15 Windows minutes per Karate bump (roughly monthly).
- Bad: the repo setting also lets workflows approve PRs. Without branch protection an approval gates nothing, so this changes nothing today; revisit with branch protection.
- Bad: every Karate bump's fixture commit is mostly noise (time stamps, durations, ports). The tests on it are the proof, not the diff.
- Neutral: every existing `vendor/` has no marker, so its next `npm run vendor` unpacks the JRE once more.
