---
status: built
issue: 150
summary: "Ship only the en-US Chromium locale and drop the other 54 locale files to make the download smaller"
---

# Ship only the en-US locale

Date 2026-11-02. Covers [#150](https://github.com/hpagniez/grace/issues/150).

## Intent

Electron ships Chromium's interface strings for 55 languages, one `locales/<language>.pak` file each. Grace's own interface is English only, and it shows no Chromium text of its own: it has no application menu (`Menu.setApplicationMenu(null)`) and opens only the operating system's file dialogs. The other 54 files are dead weight in a download and an install that people repeat at every update ([ADR 0019](../decisions/0019-update-the-zip-install-from-github-releases.md)). Download and install size come third in [ADR 0023](../decisions/0023-run-lightly-as-a-release-requirement.md)'s order, and this change costs nothing above them.

## Who and what success looks like

The person who downloads Grace, or updates it in one click.

Success: the zip and the unpacked folder are smaller by the size of the 54 files, and nothing in the app looks or behaves differently.

## Goals, non-goals, anti-goals

**Goals:** the behaviour in the Requirements section below.

**Non-goals:**
- Translating Grace, or choosing a language.
- Trimming other Electron files (`chrome_100_percent.pak`, `vk_swiftshader.dll`, `ffmpeg.dll`, and so on).
- Doing the same on another platform: Grace ships for Windows only.

**Anti-goals** (failure even if everything else works):
- The updater refuses a Grace folder, or a zip, because of the missing files.
- Grace fails to start, or shows blank text, on a machine whose language is not English.

## Constraints

- The packaged folder is checked by the in-app updater against `resources\app-files.json`, a list of the folder's top-level names ([ADR 0019](../decisions/0019-update-the-zip-install-from-github-releases.md)). `locales` stays a top-level name, so the list does not change. The zip's name and layout do not change.
- `forge.config.mts` already has a `postPackage` hook that writes `app-files.json` and copies the licences. The change goes there.

### What exists today

- `locales/` holds 55 files, 48.8 MB unpacked (51.2 MB on disk in blocks). `en-US.pak` is 0.6 MB.

## Approaches considered

- **A. Delete the files in the `postPackage` hook (chosen).** Four lines in the hook that already runs on the packaged folder. The packager and Forge have no option for it; their `ignore` option filters the app's own source, not Electron's files.
- **B. A packager `afterExtract` hook.** The same effect, a second hook beside the one that exists.
- **C. Keep the files.** Keeps today's size.

## Requirements

### R1. One locale

1. After packaging, `locales/` holds `en-US.pak` and no other file.
2. The rest of the folder, `resources/app-files.json` included, is as without the change.

### R2. Chromium falls back

1. A machine whose language is not English starts Grace and shows its window and its text. Chromium picks `en-US` when the file for the machine's language is missing.
2. Two things follow from that fallback, and neither shows in Grace: `navigator.language` and the default locale of `Intl` read `en-US` instead of the machine's language, and Chromium's own strings (a form field's validation bubble, say) are English. Grace formats with explicit locales only (`Intl.RelativeTimeFormat("en")` in `src/renderer/lastChecked.ts`; durations by hand in `nodes.ts`), so no text or number changes. A later feature must name its locale and not read the default one.
3. File dialogs are drawn by Windows and keep the machine's language.

## Cost

Measured with `npm run make` on `develop` at `bd788b98` and on this branch, same machine, same day:

- Size: smaller. The zip 218,731,755 bytes → 206,167,177 (−12.6 MB, −5.7 %); the unpacked folder 538,710,404 → 488,107,477 (−50.6 MB, −9.4 %). The zip differs by the 54 `locales/*.pak` files and nothing else; `resources/app-files.json` is identical. The updater also downloads and unpacks 12.6 MB less at every update. `size.unpacked` and `size.zip` in `bench/limits.json` drop to 489.1 and 207.2 MB (the new size plus the 1 MB noise).
- Memory, CPU, start time: none. Chromium loads one locale file at start, and loads `en-US.pak` as before when the machine's language is English; where it is not, it now loads `en-US.pak` instead of that language's file, of the same order of size.
- Idle stays at zero: the change runs at package time and adds no timer, poll or process.

## Acceptance criteria

```gherkin
Feature: Only the en-US locale

  Scenario: The package holds one locale
    Given I run "npm run package"
    Then "locales" holds "en-US.pak" and no other file

  Scenario: The updater contract does not change
    Given I run "npm run make"
    Then the zip is named "Grace-win32-x64-<version>.zip" with the app at its root
    And "resources/app-files.json" lists the same top-level names as before

  Scenario: Grace starts on a machine with another language
    Given the packaged app
    When I start it with "--lang=fr"
    Then its window shows the home screen in English
    And the page's language is "en-US"
    And a date formats as "2 novembre 2026" for "fr-FR"

  Scenario: A file dialog keeps the machine's language
    Given the packaged app started with "--lang=fr" on a machine whose language is French
    When I click "Open folder…"
    Then Windows' folder dialog opens in French
```
