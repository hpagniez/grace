# Check: only the en-US locale (#150)

Date: 2026-11-02
Build: `feature/en-us-locale`, `develop` at `bd788b98` plus the `postPackage` change. Baseline: the same commit without it.
Machine: the reference machine of the [baseline](2026-10-31-bench-baseline.md) (Windows 11 Pro, Ryzen 7 5800X, 32 GB), French system language.
Spec: [docs/specs/2026-11-02-en-us-locale.md](../specs/2026-11-02-en-us-locale.md).

**Result: the zip is 12.6 MB smaller and the unpacked folder 50.6 MB smaller; the updater's contract and the app's behaviour are unchanged.**

## Size

`npm run make` on each build, one after the other:

| | Baseline | With the change | Change |
|---|---|---|---|
| `Grace-win32-x64-1.0.0-rc.8.zip` | 218,731,755 B | 206,167,177 B | −12,564,578 B (−5.7 %) |
| Unpacked `Grace-win32-x64` | 538,710,404 B | 488,107,477 B | −50,602,927 B (−9.4 %) |
| Files in `locales/` | 55 | 1 (`en-US.pak`) | −54 |

Listing both zips with `tar -tf`: they differ by the 54 `locales/*.pak` files and nothing else; the zip's name and root are the same. `resources/app-files.json` is byte-identical (it lists top-level names, and `locales` stays one).

## The packaged app on a machine in another language

`Grace.exe` from `out/Grace-win32-x64`, started with `--lang=fr` and `--lang=ja` and a fresh data folder, attached over CDP (`--remote-debugging-port`):

- It starts, shows the home screen with its English text ("Open a Karate project", "Open folder…"), no blank text or missing glyph.
- `navigator.language` and `Intl.DateTimeFormat().resolvedOptions().locale` read `en-US`; the baseline build reads `fr`. Explicit locales still format: `toLocaleDateString("fr-FR", { dateStyle: "long" })` gives `2 novembre 2026`, `(1234567.89).toLocaleString("fr-FR")` gives `1 234 567,89`, `toLocaleString("de-DE")` gives `2.11.2026, 14:05:00`. Grace reads no default locale (`grep` for `toLocale`, `Intl`, `navigator.language` finds only `Intl.RelativeTimeFormat("en")`).
- The folder dialog ("Open folder…") opens and is in French, the machine's language: Windows draws it.
- The spell checker: Grace turns it off on its inputs (`spellcheck={false}`) and Electron on Windows uses the system's; neither reads `locales/`. Not exercised further.

## Tests

- `npm test`: 97 files, 1,513 tests passed.
- `npm run e2e`: 169 passed (8.8 min). It runs the built app (`.`), not the packaged one, so it does not cover the missing files; the packaged launches above do.

## Needs a person

- The update from a build that ships all locales to one that ships only `en-US` (in-app updater, by hand): not run; the folder check reads only top-level names, which are unchanged.
