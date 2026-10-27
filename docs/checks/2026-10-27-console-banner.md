# App check: console banner (F24)

Date: 2026-10-27
Branch: `feature/console-banner`
Machine: Windows 11 Pro 10.0.26300 (x64), Node 24, bundled Temurin 21.0.12.1+1 and Karate 2.1.4.
Spec: [Console banner](../specs/2026-10-27-console-banner.md).

## Automated part (run by an agent)

| Check | Result |
|---|---|
| `npm test` | 684/684 passed, including `src/renderer/bannerText.test.ts` (15), `src/renderer/Banner.test.tsx` (7), a new test in `src/renderer/Console.test.tsx` and 2 in `scripts/fetch-vendor.test.ts` |
| `npm run e2e` | 103/103 passed, including the new `e2e/console.spec.ts` (2) |
| `npm run typecheck`, `npm run lint` | passed |
| `node scripts/fetch-vendor.mjs` after the pin refactor | `vendor ready`: the jar's SHA-256 still matches |
| After merging `develop` with F33 (versions now from `scripts/vendor.json`; the pin refactor and its 2 tests dropped): `npm test`, `npm run typecheck`, `npm run lint`, `npm run package` then `npx playwright test e2e/console.spec.ts` | 718/718 passed; passed; passed; 2/2 passed |

Also run by the agent: the built app started at 1100×700 in dark and light mode, with no project, with the clean sample project open (`9 scenarios in 3 features`), and after Run all. The banner, its colours, the darker dark-mode panel and the output under the banner looked as the spec says.

Not covered by the automated part: the cursor's blink, Windows' animation setting, copying the console, and how it reads to a person.

## By hand (done by a person)

Build with `npm run package`, then start `out\karate-kid-win32-x64\karate-kid.exe`. Write the result under each step: passed, or what you saw instead.

1. With Windows in dark mode, start the app with no project. The console shows `karate-kid 1.1.0 · Karate 2.1.4 · Java 21.0.12.1+1`, `project  none`, and `Open a Karate project folder` with a blinking block cursor. The console is visibly darker than the tree and details above it.
   - Result: passed (checked by Tusquito, 2026-10-27).
2. Open a project. While it is found, the state line says `Finding tests…`. Then the banner shows the project folder and `tests    <n> scenarios in <m> features`, the values lined up under each other, and `Ready. Press Run all, or ▶ on a test.` with `Ready.` in green.
   - Result: passed (checked by Tusquito, 2026-10-27).
3. Press Run all. `Starting Karate…` shows until Karate's first line arrives, then the output streams under the banner, which stays at the top. Scroll up during the run: the console stops following; scroll back to the bottom: it follows again.
   - Result: passed (checked by Tusquito, 2026-10-27).
4. Press Run all again. The first run's output is gone; the banner shows once, at the top.
   - Result: passed (checked by Tusquito, 2026-10-27).
5. Select the whole console text and paste it into a text editor: the banner lines come first, followed by Karate's output.
   - Result: passed (checked by Tusquito, 2026-10-27).
6. Turn off Settings, Accessibility, Visual effects, Animation effects, and restart the app: the cursor stays still. Turn it back on.
   - Result: passed (checked by Tusquito, 2026-10-27).
7. Switch Windows to light mode and restart: the console is the same dark panel as before F24 on the white window, and the banner is readable.
   - Result: passed (checked by Tusquito, 2026-10-27).
