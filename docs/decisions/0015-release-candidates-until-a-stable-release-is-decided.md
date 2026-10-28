---
status: accepted
date: 2026-10-28
decision-makers: Tusquito
---

# Release candidates only until a stable release is decided

## Context and Problem Statement

`main` was tagged `v1.0.0` and `v1.1.0`, and the `release` workflow published both as stable GitHub Releases, `v1.1.0` as Latest. The app is still in development: AGENTS.md drops backward compatibility, and settings, run history and data folders may change without migration. A stable SemVer version promises the opposite. How should versions say "not stable yet"?

## Considered Options

1. Every version is a release candidate of the first stable release, `1.0.0-rc.N`, in the branch, `package.json` and tag, published as a GitHub pre-release.
2. Keep `vX.Y.Z` tags and only flag each GitHub Release as pre-release.
3. Restart at `0.y.z`, SemVer's range for initial development.

## Decision Outcome

Chosen: option 1. The first stable release is `1.0.0`, and every release before it is a candidate for it. The `-rc.N` suffix is in every place the version shows: the tag, the zip name, the release title and the app's `package.json`. A flag on the GitHub Release (option 2) is lost as soon as the zip is passed on. Option 3 works too, but `1.0.0-rc.N` names the release it leads to: stable `1.0.0` is the same version without the suffix.

- Release and hotfix branches are `release/vX.Y.Z-rc.N` and `hotfix/vX.Y.Z-rc.N`, e.g. `release/v1.0.0-rc.3`. This amends the pattern of [ADR 0009](0009-release-and-hotfix-branches-named-after-their-tag.md); the rest of 0009 still holds. `scripts/check-pr.mjs` rejects a plain `release/vX.Y.Z`.
- `package.json` carries the same version, e.g. `1.0.0-rc.3`, so the `release` workflow's tag check still compares like for like. `develop` holds the last released candidate until a release branch bumps it.
- The `release` workflow runs only on `v*.*.*-rc.*` tags and creates the GitHub Release with `--prerelease`. A plain `vX.Y.Z` tag builds nothing.
- Each release or hotfix bumps `N`: the next one is `1.0.0-rc.3`.
- The two stable releases were retagged on the same commits: `v1.0.0` became `v1.0.0-rc.1` and `v1.1.0` became `v1.0.0-rc.2`, recreated as pre-releases with the same zips and notes. The old tags and releases are deleted. The apps inside those zips still report `1.0.0` and `1.1.0`.
- The first stable release, `1.0.0`, needs a new decision record that supersedes this one and lifts the `-rc.N` rule in `check-pr.mjs` and `release.yml`.

### Consequences

- Good: no build can be mistaken for a stable release, and GitHub shows no release as Latest.
- Good: a stable release cannot happen by accident; it takes a reviewed change to the check and the workflow.
- Neutral: Electron Packager turns `1.0.0-rc.3` into the Windows file version `1.0.0.3` (it reads `0-rc` as `0`). The product version keeps the full string.
- Bad: links to the old `v1.0.0` and `v1.1.0` tags and releases are broken.
