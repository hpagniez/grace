# App updates

Date 2026-10-28. Status: approved. Covers [#77](https://github.com/hpagniez/grace/issues/77). Decision: [ADR 0019](../decisions/0019-update-the-zip-install-from-github-releases.md).

## Intent

Grace ships as a zip on GitHub Releases. Once someone unzips it, nothing tells them a new version exists, and getting it means finding the release page, downloading 218 MB, and unzipping over the old folder by hand. Fixes and new features never reach most people who already use it.

This spec makes Grace show its version, notice when a newer one exists, and install any published version, newer or older, in one click. It backs up the person's data before every switch, so going back to an older version starts from the data that version last saw.

## Who and what success looks like

The walking skeleton's user: a tester or developer who unzipped Grace once and runs an API suite with it every day. They do not follow the repository.

Success:
- They start Grace after a release and are told about it once, with a one-click way to get it.
- They pick whether they want pre-releases or only stable releases.
- They can always see which version they run, and whether it is the newest on their channel.
- A version that breaks something is one click from being rolled back, with the data that older version had.
- If they ask for it, Grace updates itself at startup without asking.

## Goals, non-goals, anti-goals

**Goals:** the behaviour in the Requirements section below.

**Non-goals:**
- Code signing, an installer, or a move away from the zip (ADR 0019).
- Delta or partial downloads. Every install downloads the full zip.
- macOS and Linux. Grace is built for Windows only.
- Updating the bundled JRE or Karate on their own. They come with the app's version.
- Checking for updates while Grace runs. Checks happen at startup and when asked for.
- Downloading in the background to install later, or installing on quit.
- Release notes inside the app. "What's new" opens the release page.
- A backup or restore button for data outside a version switch.

**Anti-goals** (failure even if everything else works):
- An update that leaves Grace unable to start, or no Grace at all in the install folder.
- Data lost by an update, or by a rollback the person was not warned about.
- Grace going online on its own when "Check for updates" at startup is off.
- Running a downloaded file whose checksum was not checked.
- A startup modal on every start for a version the person chose to skip.
- An error dialog because GitHub is unreachable.

## Constraints

- No new dependency. Node's `fetch` and `crypto`, Electron's `shell`, Windows' own `tar.exe` and PowerShell.
- The renderer stays sandboxed: `contextIsolation`, `sandbox`, `nodeIntegration: false` and the CSP `default-src 'self'` stay. Every network request and file operation runs in the main process or the switch script; the renderer reaches them through new typed preload calls.
- GitHub's REST API without a token: 60 requests an hour per IP address.
- Every release so far is a GitHub pre-release (ADR 0015), so GitHub's "latest release" endpoint returns nothing. Grace lists all releases and picks itself.
- e2e tests launch the built app unpackaged (`_electron.launch({ args: ['.'] })`), never the packaged exe (AGENTS.md, ADR 0004). The swap of a packaged install cannot run in e2e.
- The updater is a contract between versions: a version's update code installs the next one, and the next one reads what the previous one's switch script wrote. See R11.

### What the app does today

- `package.json` `version` is the release version (`1.0.0-rc.3`); `__KK_VERSIONS__.app` carries it to the renderer, and the console banner shows `Grace <version> · Karate … · Java …`.
- The `release` workflow builds `Grace-win32-x64-<version>.zip` and attaches it to a GitHub pre-release tagged `v<version>`. Releases are immutable. The GitHub API returns each asset with `size` and `digest` (`sha256:<hex>`), e.g. rc.3: 218,557,130 bytes.
- The zip holds the packaged app at its root (`Grace.exe`, `resources/app.asar`, `resources/` vendor files, DLLs); unpacked it is about 515 MB. People unzip it into a folder of their choice and start `Grace.exe` from there.
- `C:\Windows\System32\tar.exe` (bsdtar, Windows 10 1803+) reads zip files. Git for Windows' GNU `tar.exe`, often first on `PATH`, does not.
- Grace's data is Electron's `userData` folder (`%APPDATA%\Grace`; `GRACE_USER_DATA` in tests): `settings.json` (last project, recent folders, per-project filters, environment, threads, properties, secrets encrypted with `safeStorage`, window state), `projects/<hash>/runs/` (run history with Karate reports, up to 20 runs a project, ADR 0010), Chromium's `Local Storage/` (the bottom panel height), plus Chromium caches.
- `src/main/settings.ts` writes `settings.json` atomically through `update()` and keeps other keys.
- Main makes no network request today. `shell` is imported but used only to open reports.
- The single-instance lock is per `userData` folder. Quitting disposes the runner, which kills a running JVM tree.
- Views: `App.tsx` shows `Home` with no project open, or the toolbar, tree, details and bottom panel. There is no settings view and no status bar. `PropertiesDialog` is the existing modal pattern.

## Approaches considered

1. **Grace's own updater over the zip.** List releases from the GitHub API, download the chosen zip, check its SHA-256 against GitHub's digest, unpack it next to the install, then quit and let a PowerShell script back up the data, swap the folders and start the new version. Handles any version in both directions, both channels and the backups, with no new dependency. **Chosen.**
2. Squirrel.Windows installer and Electron's `autoUpdater`. Moves forward to the newest version only, so picking a version and rolling back need approach 1's code anyway. Electron's update service most likely ignores pre-releases, and every release is one. Squirrel.Windows is barely maintained.
3. electron-builder's `electron-updater` with an NSIS installer. Has channels and downgrades, but brings the electron-builder toolchain into a Forge project, replaces the zip with an installer, and still has no "install this version".

## Requirements

### R1. Versions and channels

1. A version is `X.Y.Z` or `X.Y.Z-rc.N` with decimal numbers, the only forms ADR 0015 allows. Versions compare by SemVer precedence: `X`, `Y`, `Z` as numbers, then a pre-release below its release (`1.0.0-rc.9` < `1.0.0-rc.10` < `1.0.0`). Anything else from GitHub is ignored, never shown and never installed.
2. Two channels: **stable** (releases not flagged pre-release) and **pre-release** (stable and pre-releases).
3. The channel is saved. Until the person picks one, it is pre-release when the running version is a pre-release, stable otherwise.
4. A release is **installable** when it is not a draft, its tag is `v<version>` with a valid version, the version is at least `FIRST_UPDATER` (a constant: the first release that ships this feature), and it has an asset named exactly `Grace-win32-x64-<version>.zip` with a `size` above 0 and a `digest` of the form `sha256:<64 hex>`.
5. **Latest** is the highest installable version on the channel. Grace is **out of date** when latest is above the running version.

### R2. Checking

1. A check is one `GET https://api.github.com/repos/hpagniez/grace/releases?per_page=100`, without a token, from the main process, with a 15 s timeout. The response is untrusted input: parsed and validated against R1.4 field by field.
2. At startup, if **Check for updates** is on (default: on), Grace runs one check after the window shows. It never delays the window or the project reopening.
3. With it off, Grace makes no network request until the person clicks **Check for updates** in Settings › Updates.
4. One check at a time: a click while a check runs waits for that check.
5. A failed check (offline, timeout, HTTP error, rate limit, unreadable JSON) never shows an error dialog. At startup it shows nothing. In settings it shows `Couldn't check for updates: <reason>` and the time of the last successful check.
6. The time of the last successful check is saved and shown as `Last checked <relative time>`.
7. The result of the last successful check in this session (the installable releases) is kept in memory for the status bar, the modal and the settings page. Nothing from it is saved except the check time.

### R3. Status bar

1. A status bar runs along the bottom of every view: home, project and settings. On its right, in order: the out-of-date marker, `Grace <version>`, `⚙ Settings`.
2. The marker shows only while out of date, the skipped version included: an amber dot, `<latest> available`, and an `Update` link.
3. `Update` installs latest (R8). The marker, the version and `⚙ Settings` open Settings › Updates.
4. Each item is a button reachable by keyboard, with an accessible name (`Grace 1.0.0-rc.4, 1.0.0-rc.5 available`).
5. The console banner keeps its version line.

### R4. Startup modal

1. After the startup check (R2.2), Grace shows the modal once, only when out of date, auto-update is off, and latest is above the skipped version (or none is skipped).
2. Content, per the [mockup](2026-10-28-app-updates-mockup.png) (part 2):
   - Title `Grace <latest> is available`.
   - `You have <running>. Download: <size> MB. Your data is backed up before Grace restarts.` The size is the asset's, rounded to whole MB.
   - `What's new`: opens `https://github.com/hpagniez/grace/releases/tag/v<latest>` in the default browser. The URL is built from the validated version, never taken from the response.
   - A checkbox `Update automatically at startup from now on`, unticked.
   - Buttons `Skip this version` and `Update and restart` (primary, focused).
3. `Skip this version` saves latest as the skipped version and closes the modal. The modal shows again only for a version above it.
4. `Update and restart` saves auto-update on if the checkbox is ticked, then installs latest (R8). With `Skip this version` the checkbox is ignored.
5. Esc or the close button closes the modal and saves nothing: it shows again at the next start.
6. The modal is a real dialog: focus stays inside it while open and returns where it was when it closes.

### R5. Auto-update

1. **Update automatically, without asking** (default: off) sits under **Check for updates** and is disabled while that is off.
2. With both on, the startup check finding Grace out of date shows no modal: it installs latest straight away (R8), with the progress dialog and its Cancel. Auto-update installs latest even if it was skipped, but never the version saved as `failed` (R10.6).
3. Cancel, or a failure, leaves the running version and shows why (R8.6). The next start tries again, except the start right after a failed switch (R10.7): that one shows the failure and does not auto-update, so a file held open cannot cause a loop of downloads and restarts.

### R6. Settings page

1. `⚙ Settings` replaces the view below the status bar with the Settings page, per the [mockup](2026-10-28-app-updates-mockup.png) (part 4): a header with `← Back` and `Settings`, a left list with one entry, **Updates**, and the section on the right. `← Back` (and Esc) returns to the view the person left, unchanged. An open project stays open and a running run keeps running.
2. The Updates section shows, top to bottom:
   - `Grace <running>`, the marker when out of date, and a primary `Update to <latest>` button.
   - `Last checked …` and a `Check for updates` link (R2).
   - **Channel**: radio buttons `Stable releases only` and `Stable and pre-releases`. A change is saved at once and the marker, list and button follow from the last check, without a new request.
   - **At startup**: the checkboxes of R2.2 and R5.1.
   - **Versions**: the table of R6.3.
3. The table lists every installable release on the channel, newest first, plus the running version if it is not among them. Columns: version, release date, a `pre-release` tag, an `installed` tag on the running version, a `data saved` tag where a backup of that version exists (R9). Rows above the running version have `Install`; rows below have `Roll back`; the running row has none. Without a successful check, the table holds the running version only and says `Check for updates to see other versions`.
4. `Install`, `Roll back`, `Update` and `Update to …` are disabled while a run or a discovery dry run is active, with `Finish or cancel the run first`, since restarting kills it. In a build that is not packaged they are disabled with `Updates install only in the packaged app`.
5. `Install` on a newer version starts R8 directly, unless a backup of that version exists: then it opens the data choice of R7.6 first. `Roll back` opens the confirmation of R7 first.

### R7. Rollback confirmation and data choice

1. Title `Roll back to <version>?`, per the [mockup](2026-10-28-app-updates-mockup.png) (part 5).
2. With a saved copy of that version: `Grace backs up your current data, then restores the data saved when you last used <version> (<date of the copy>).`
3. Without one: `Grace backs up your current data. <version> has no saved data on this computer, so it starts from your current data, which it may not read correctly.`
4. Always, in an amber box: `Going back to an older version may break your data. Runs, settings and secrets added since then are not in the restored copy; they stay in the backup of <running>, and come back if you install <running> again and use its saved data.` (The second sentence only when a saved copy exists.)
5. Buttons `Cancel` (focused) and `Roll back and restart`.
6. **Data choice on an install from the Versions table.** `Install` on a newer version that has a backup (typically after a rollback) asks `Install <version>?` with two choices, `Use the data you had in <version> (<date of the copy>)` and `Keep your current data` (selected), and the buttons `Cancel` and `Install and restart`. The first restores that copy after backing up the current data, as a rollback does (R9.3). The startup modal, the status bar's `Update`, `Update to <latest>` and auto-update always keep the current data and never ask.

### R8. Installing a version

The same steps for an update, a rollback and auto-update. A progress dialog shows them: `Updating to <version>` or `Rolling back to <version>`, a progress bar, the step (`Downloading… 92 of 218 MB`, `Checking…`, `Unpacking…`), and `Cancel` until Grace quits.

1. **Preflight.** The app must be packaged. The install folder is the folder of `process.execPath`. Otherwise nothing is downloaded and the message says why:
   - Its parent must accept a new folder (Grace creates and removes one). Otherwise: `Grace can't update itself in <folder>. Move the Grace folder somewhere you can write to, such as your user folder, or download the new version from the release page.` with the release page link.
   - It must hold only Grace's files, since the swap moves the whole folder and later deletes the old one. Every top-level entry must be listed in `resources\app-files.json`, the list of the packaged folder's top-level names that the build writes (a Forge hook), apart from Windows' `desktop.ini` and `Thumbs.db`. Otherwise: `Grace's folder also holds other files (<up to 3 names>). Put Grace's files in a folder of their own to update it.` This catches a zip unpacked straight into `Downloads` or `C:\Tools`.
   - The drive must have room for the zip plus the unpacked app: Grace asks for 4 times the zip size.
2. **Download.** Into a staging folder next to the install, `<install>.update`, on the same drive so the swap is a rename. A leftover staging folder is removed first. The download is the asset's `browser_download_url`, which must start with `https://github.com/hpagniez/grace/releases/download/v<version>/`. Redirects are followed.
3. **Check.** The file's SHA-256 must equal the asset's digest and its size the asset's size. Otherwise it is deleted and the message says the download was damaged.
4. **Unpack.** `%SystemRoot%\System32\tar.exe -xf <zip> -C <staging>\app`, by absolute path. `<staging>\app\Grace.exe` and `<staging>\app\resources\app.asar` must exist afterwards.
5. **Hand over.** Grace writes the switch plan (R10.1) and the switch script into the staging folder, starts the script detached and hidden (`powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -WindowStyle Hidden -File <staging>\switch.ps1`), then quits the normal way (the runner is disposed as today).
6. **Before hand-over, nothing changes.** Cancel, or a failure in steps 1 to 5, removes the staging folder and leaves the running app, the install folder and the data as they were. The message names the step that failed and links the release page. No error dialog: the progress dialog shows the message with a `Close` button.

### R9. Data backups

1. Before every switch, the switch script copies the data folder to `<userData>\backups\<running version>\`, replacing an older copy of that version. The copy leaves out `backups\` itself, `update-result.json`, and Chromium's caches and lock files (left to the builder: at least `Cache`, `Code Cache`, `GPUCache`, `GrShaderCache`, `ShaderCache`, `GPUPersistentCache`, `DawnGraphiteCache`, `DawnWebGPUCache`, `Crashpad`, `DevToolsActivePort`, `lockfile`, `SingletonLock`).
2. Copies are made and restored only after Grace has exited, so no file is in use or half-written.
3. On a rollback (target below running) with a copy of the target: the data folder's contents, except `backups\`, are replaced by that copy. Without a copy: the data stays.
4. On an update (target above running): the data stays and the new version reads it, unless the person chose that version's saved data (R7.6), which is then restored as in R9.3. Before `1.0.0` it may lose data it cannot read (AGENTS.md); from `1.0.0` it must not (R11).
5. Only the copies of the 3 most recently backed-up versions are kept; the script deletes older ones after a successful switch.
6. A copy that fails (disk full, a file it cannot read) stops the switch before the swap: the old version starts again and reports it (R10.5).

### R10. Switch script

1. The plan is a JSON file next to the script: Grace's process id, running and target versions, install folder, staging folder, data folder, whether it is a rollback, whether to restore the target's saved data (R9.3, R7.6), and the update settings to carry over (R10.7).
2. The script waits until no process runs from the install folder (Grace's main process and its helpers: renderer, GPU, crash handler), up to 60 s. On timeout it does nothing else, writes a failure result and exits.
3. Then, in order: back up (R9.1), restore the target's saved data when the plan says so (R9.3), rename the install folder to `<install>.old`, rename `<staging>\app` to the install folder.
4. If a rename fails (a file held by Explorer or an antivirus), it puts the old folder back in place, as before. If it had already restored the target's data, it puts the running version's data back from the copy it just made (R9.1), so a later try does not overwrite the real backup.
5. It writes `<userData>\update-result.json` (`from`, `to`, `rollback`, `ok`, `error`, `updates`) and starts `Grace.exe` from the install folder: the new version on success, the old one on failure. Grace never ends up with no install folder.
6. **The new version must start.** Once its window has shown, the new version writes `<install>.update\started`. The script waits up to 120 s for that file (a first antivirus scan of an unpacked app can take tens of seconds). Without it, the script switches back: it stops every process running from the install folder (`taskkill /T /F`), renames the install folder to `<install>.failed` and `<install>.old` back to the install folder, puts the running version's data back from the copy it made (R9.1), writes a failure result (`<to> didn't start within 2 minutes`) and starts the old version. That version saves `<to>` as `failed` (R12): auto-update never installs it again; a manual install still can.
7. At start, Grace reads and deletes `update-result.json`:
   - Success: a notice `Updated to <to>.` or `Rolled back to <to>.`; once the script has exited (R10.9), it removes `<install>.old` and `<install>.update` in the background.
   - Failure: a notice `Couldn't switch to <to>: <error>. You are still on <from>.`; once the script has exited, it removes `<install>.failed` and `<install>.update`.
   - After a rollback, the update settings in `settings.json` are the ones the person had before the rollback (carried in `updates`), with auto-update turned off and `<from>` saved as skipped, so the rollback holds until they update by hand.
8. A leftover `<install>.old`, `<install>.failed` or `<install>.update` with no result file and no live script is removed at the next start.
9. While the script runs, `<install>.update\switching` holds its process id. A Grace started from the install folder while that process lives shows `Grace is finishing an update and starts on its own in a moment.` in a message box and quits before opening its window.

### R11. Compatibility from now on

1. AGENTS.md's "No backward compatibility" holds until the first stable release, `1.0.0`. From `1.0.0` on, every version reads the data of every earlier stable version. Rolling back is not promised to read newer data; R7 warns about it.
2. The updater contract holds from `FIRST_UPDATER` on, in every version, pre-releases included: the release tag `v<version>`, the asset name `Grace-win32-x64-<version>.zip` with a GitHub digest, the zip layout (the app at its root), `resources\app-files.json`, `update-result.json`, the `started` and `switching` files, the backup folder layout, and the `updates` key in `settings.json`. Changing any of them strands installed versions; it needs a new decision record.

### R12. Saved settings

`settings.json` gets one key, `updates`: `{ channel: "stable" | "prerelease", checkAtStartup: boolean, autoUpdate: boolean, skipped: string | null, failed: string | null, lastChecked: string | null }`. A missing or malformed key or field reads as its default (channel per R1.3, `true`, `false`, `null`, `null`, `null`). Saved through `update()`.

### R13. Development and tests

1. A build that is not packaged checks, shows the status bar, the modal and the settings page, but never installs (R6.4).
2. `GRACE_UPDATE_FEED`, honoured only when the app is not packaged, replaces the GitHub API with a local file path or a `http://127.0.0.1` URL that returns the same JSON. e2e tests use it, as they use `GRACE_USER_DATA`.

## Acceptance criteria

```gherkin
Feature: App updates

  Background:
    Given Grace 1.0.0-rc.5 is running
    And the releases are 1.0.0-rc.4, 1.0.0-rc.5 and 1.0.0-rc.6, all pre-releases with their zip and digest

  Scenario: The version is always visible
    When Grace shows the home screen, a project or the settings page
    Then the status bar shows "Grace 1.0.0-rc.5" and "⚙ Settings"

  Scenario: Out of date on the pre-release channel
    Given the channel is "Stable and pre-releases"
    When the startup check finishes
    Then the status bar shows "1.0.0-rc.6 available" with an "Update" link

  Scenario: Nothing newer on the stable channel
    Given the channel is "Stable releases only"
    When the startup check finishes
    Then the status bar shows no marker
    And no modal shows

  Scenario: Default channel follows the running version
    Given no channel was ever picked
    When Settings › Updates opens
    Then "Stable and pre-releases" is selected

  Scenario: Startup modal
    Given the channel is "Stable and pre-releases" and nothing is skipped
    When the startup check finishes
    Then a modal says "Grace 1.0.0-rc.6 is available" and "You have 1.0.0-rc.5"
    And it offers "Skip this version" and "Update and restart"

  Scenario: Skipping a version
    Given the startup modal shows 1.0.0-rc.6
    When I click "Skip this version"
    And I restart Grace
    Then no modal shows
    But the status bar still shows "1.0.0-rc.6 available"

  Scenario: A newer version after a skip
    Given 1.0.0-rc.6 is skipped
    And 1.0.0-rc.7 is published
    When Grace starts
    Then the modal shows "Grace 1.0.0-rc.7 is available"

  Scenario: Closing the modal without a choice
    Given the startup modal shows 1.0.0-rc.6
    When I press Esc
    And I restart Grace
    Then the modal shows again

  Scenario: Auto-update skips the modal
    Given "Update automatically, without asking" is on
    When the startup check finds 1.0.0-rc.6
    Then no modal shows
    And the progress dialog shows "Updating to 1.0.0-rc.6"

  Scenario: No network when the startup check is off
    Given "Check for updates" at startup is off
    When Grace starts and stays open
    Then Grace makes no network request

  Scenario: A failed check is quiet
    Given GitHub cannot be reached
    When Grace starts
    Then no modal and no error dialog show
    When I click "Check for updates" in Settings › Updates
    Then the page says "Couldn't check for updates"

  Scenario: Invalid releases are ignored
    Given a release tagged "v2.0.0-beta" and a release with no zip asset
    When the check finishes
    Then neither appears in the version list

  Scenario: Version list
    Given the channel is "Stable and pre-releases"
    When Settings › Updates opens after a check
    Then the list shows 1.0.0-rc.6 with "Install", 1.0.0-rc.5 tagged "installed", and 1.0.0-rc.4 with "Roll back"

  Scenario: No install while a run is active
    Given a run is active
    Then "Install", "Roll back" and "Update" are disabled with "Finish or cancel the run first"

  Scenario: No install from a development build
    Given Grace is not packaged
    Then "Install" is disabled with "Updates install only in the packaged app"

  Scenario: Rollback warning with a saved copy
    Given a backup of 1.0.0-rc.4 exists
    When I click "Roll back" on 1.0.0-rc.4
    Then a dialog asks "Roll back to 1.0.0-rc.4?"
    And it warns "Going back to an older version may break your data."
    And it says the data saved when I last used 1.0.0-rc.4 is restored

  Scenario: Rollback warning without a saved copy
    Given no backup of 1.0.0-rc.4 exists
    When I click "Roll back" on 1.0.0-rc.4
    Then the dialog says 1.0.0-rc.4 starts from my current data

  Scenario: A damaged download changes nothing
    Given the downloaded zip does not match its digest
    When I install 1.0.0-rc.6
    Then the progress dialog says the download was damaged
    And Grace 1.0.0-rc.5 keeps running from the same folder with the same data

  Scenario: An unwritable install folder
    Given Grace runs from a folder whose parent it cannot write to
    When I install 1.0.0-rc.6
    Then nothing is downloaded
    And the message says Grace can't update itself there and links the release page

  # Packaged app, checked by hand (docs/checks)
  Scenario: Update in one click
    When I click "Update and restart" for 1.0.0-rc.6
    Then Grace restarts as 1.0.0-rc.6 from the same folder
    And a notice says "Updated to 1.0.0-rc.6."
    And my projects, history and settings are still there
    And a backup of 1.0.0-rc.5 exists

  Scenario: Rollback restores the older version's data
    Given I used 1.0.0-rc.4, then updated to 1.0.0-rc.5 and ran tests
    When I roll back to 1.0.0-rc.4
    Then Grace restarts as 1.0.0-rc.4 with the data it had before the update
    And a notice says "Rolled back to 1.0.0-rc.4."
    And auto-update is off and 1.0.0-rc.5 is skipped

  Scenario: A failed swap keeps the old version
    Given a file in the install folder is held open by another program
    When I install 1.0.0-rc.6
    Then Grace 1.0.0-rc.5 starts again
    And a notice says "Couldn't switch to 1.0.0-rc.6"

  Scenario: A version that does not start is switched back
    Given 1.0.0-rc.6 installs but its window never shows
    When 2 minutes pass after the switch
    Then Grace 1.0.0-rc.5 starts again from the same folder with its data
    And a notice says "Couldn't switch to 1.0.0-rc.6"
    And auto-update never installs 1.0.0-rc.6

  Scenario: Updating after a rollback offers the newer data
    Given I rolled back from 1.0.0-rc.6 to 1.0.0-rc.5
    When I click "Install" on 1.0.0-rc.6 in the version list
    Then a dialog offers "Use the data you had in 1.0.0-rc.6" and "Keep your current data"
    And "Keep your current data" is selected

  Scenario: A folder shared with other files is not updated
    Given Grace was unzipped straight into a folder that also holds my own files
    When I install 1.0.0-rc.6
    Then nothing is downloaded
    And the message says Grace's folder also holds other files

  Scenario: Only three backups are kept
    Given backups of 1.0.0-rc.2, 1.0.0-rc.3 and 1.0.0-rc.4 exist
    When I update from 1.0.0-rc.5 to 1.0.0-rc.6
    Then backups of 1.0.0-rc.3, 1.0.0-rc.4 and 1.0.0-rc.5 remain
```

## Left to the builder

- The module layout in main and the renderer, the IPC channel names and event shapes.
- The exact list of Chromium folders left out of a backup (R9.1), from what a packaged Grace writes to `%APPDATA%\Grace`.
- How the switch script is tested: Vitest on Windows against temp folders and a stand-in process is expected; the packaged end-to-end path goes in a `docs/checks/` record.
- The look of the status bar, modal, progress dialog and settings page within the mockups and the app's existing Tailwind patterns (`btn`, `primary`, `PropertiesDialog`).
- The `FIRST_UPDATER` value: the version of the release this feature ships in.
- How progress is reported from main to the renderer, and how often.
