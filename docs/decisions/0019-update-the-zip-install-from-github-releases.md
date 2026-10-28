---
status: accepted
date: 2026-10-28
decision-makers: Tusquito
---

# Update the zip install in place from GitHub Releases

## Context and Problem Statement

Grace ships as `Grace-win32-x64-<version>.zip` on GitHub Releases ([ADR 0007](0007-github-actions-ci-cd-on-the-free-private-plan.md), [ADR 0015](0015-release-candidates-until-a-stable-release-is-decided.md)). People unzip it into a folder of their choice. Nothing tells them a new version exists, and a new version means downloading 218 MB and unzipping it again by hand.

[#77](https://github.com/hpagniez/grace/issues/77) asks for updates in one click: a stable or pre-release channel, a manual check, the version and an out-of-date marker always visible, a startup prompt that remembers a skipped version, an opt-in auto-update at startup, and installing any published version, newer or older. Data is backed up before every switch, and a rollback restores the copy saved for the older version.

The repository is now public ([ADR 0018](0018-make-the-repository-public.md)), so the releases API needs no token. Every release is a pre-release, the zip carries the JRE and Karate jar ([ADR 0006](0006-bundle-jre-and-karate-jar.md)), and the app is not code-signed. How should a new version reach an installed Grace?

Spec: [docs/specs/2026-10-28-app-updates.md](../specs/2026-10-28-app-updates.md).

## Considered Options

1. Grace's own updater over the zip. Main lists releases from the GitHub API, downloads the chosen zip next to the install, checks it against GitHub's SHA-256 asset digest, unpacks it with Windows' `tar.exe`, then quits. A PowerShell script waits for it to exit, backs up the data, restores a saved copy on a rollback, swaps the folders and starts the result.
2. A Squirrel.Windows installer (`@electron-forge/maker-squirrel`) and Electron's `autoUpdater`, fed by update.electronjs.org or a static feed.
3. electron-builder's `electron-updater` with an NSIS installer.
4. An update notice only: tell people a version exists and link the release page.

## Decision Outcome

Chosen: option 1. It is the only option that installs any version in both directions and fits the backup rule without patching around a framework, and it keeps the zip and its folder.

- Option 2 moves forward to the newest version only, so picking a version and rolling back would need option 1's code anyway. update.electronjs.org most likely ignores pre-releases, which every release is, and Squirrel.Windows is barely maintained. It also changes how Grace is installed (`%LocalAppData%`, `Setup.exe`).
- Option 3 has channels and downgrades, but brings the electron-builder toolchain into a Forge project, replaces the zip with an installer, and still has no "install this version".
- Option 4 is not one click.

### Details

- No new dependency: Node's `fetch` and `crypto`, `%SystemRoot%\System32\tar.exe` (bsdtar reads zip; Git's GNU tar does not), and Windows PowerShell 5.1, run with `-ExecutionPolicy Bypass` for that process only.
- Trust: the download must come from `https://github.com/hpagniez/grace/releases/download/v<version>/` and match the size and `sha256` digest the API gives for that asset. Releases are immutable. Without code signing, the trust root is the GitHub account; a signed build needs its own record.
- The swap renames the install folder to `<install>.old` and the unpacked app into its place, on the same drive, after every Grace process has exited. A failed rename puts the old folder back. Files are copied or restored only while no Grace process runs.
- Grace updates only a folder that holds nothing but its own files, checked against `resources\app-files.json`, a list of top-level names the build writes. A zip unpacked straight into `Downloads` is refused rather than moved and later deleted with the person's files.
- The switch script writes `update-result.json` into the data folder; the started version reads it and reports the result.
- Data backups live in `<userData>\backups\<version>\`; the 3 most recent are kept.

### The updater is a contract between versions

A version's updater installs the next one, and the next one reads what the previous one's script wrote. From the first version that ships the updater on, these do not change without a new record, pre-releases included: the tag `v<version>`, the asset name `Grace-win32-x64-<version>.zip`, the app at the zip's root, `resources\app-files.json`, `update-result.json`, the backup layout, and the `updates` key in `settings.json`.

### Backward compatibility after 1.0.0

AGENTS.md's "No backward compatibility" rule holds until the first stable release, `1.0.0`, and keeps the repository free of compatibility code while the data formats still move. From `1.0.0` on, people depend on updates, so every version must read the data of every earlier stable version. Rolling back to an older version is not promised to read newer data; Grace warns before a rollback and restores the copy saved for that version. AGENTS.md says so. The record that supersedes ADR 0015 for `1.0.0` inherits this.

### Consequences

- Good: one click to update, roll back or install any version, with the data of the version being left always backed up first.
- Good: the install keeps its folder, so shortcuts and pinned taskbar icons keep working.
- Good: no installer, no new dependency, no new release artefact; the `release` workflow stays as it is.
- Neutral: every switch downloads the full zip (218 MB) and needs about 4 times that on the drive while it runs.
- Neutral: the folder keeps its name, so a folder Windows' "Extract all" named `Grace-win32-x64-1.0.0-rc.4` still has that name after an update to a later version.
- Bad: the swap is Grace's own code, and its packaged path cannot run in e2e (ADR 0004); it is covered by unit tests of the script and a by-hand check per release.
- Bad: Grace cannot update itself from a folder whose parent it cannot write to (`Program Files`), or where PowerShell scripts are blocked by policy. It says so and links the release page.
- Bad: versions released before the updater (`1.0.0-rc.1` to `1.0.0-rc.3`) never learn of updates, and cannot be installed from the list.
