# Playwright spike: end-to-end tests for the Electron app

Date 2026-10-24. Windows 11, Node 24.15. Ran `@playwright/test` 1.64.1 against a copy of the [stack spike](../2026-10-24-stack-spike/README.md) (Electron 45.0.0, Forge 8.0.1). Supports [ADR 0004](../../decisions/0004-playwright-for-end-to-end-tests.md). Throwaway; [playwright.config.ts](playwright.config.ts) and [e2e/app.spec.ts](e2e/app.spec.ts) are a reference for the real setup.

## Result

| Item | Result |
|---|---|
| `_electron.launch({ args: ['.'] })` on the built app (`.vite/build` + `.vite/renderer` after `electron-forge package`), dev Electron binary | works, test passes in about 1.5 s |
| Renderer: locators, click a Kobalte button, router link to `#/runs/42`, auto-waiting `expect` | works |
| Renderer `evaluate`: no `require` or `process` (sandbox on) | works |
| Main `electronApp.evaluate(({ app }) => ...)` | works |
| Packaged exe (`executablePath`) with the Forge template fuses | **fails**: `electron.launch: Timeout 15000ms exceeded` |
| Same exe repackaged with `EnableNodeCliInspectArguments: true` | works |
| `npx playwright install` (browser download) | not needed; Playwright drives the project's Electron binary |

The spec replaces the stack spike's hand-written `SPIKE_AUTOCHECK` in `src/main.ts`.

## Snags

- Playwright starts Electron with `--inspect=0 --remote-debugging-port=0`. The Forge template sets the fuse `EnableNodeCliInspectArguments: false`, so the packaged exe ignores `--inspect` and the launch hangs. Keep the fuse off in shipped builds (it blocks attaching a debugger to the user's app); test the built app with the dev Electron binary instead.
- Electron 45 downloads its binary on first launch. The first test run spent its whole 30 s timeout on the download. Run `npx electron --version` once before the tests (and after each `npm ci` in CI).
- `locator.textContent()` waits for the element. Reading an element from a route you just left hangs until the test timeout; assert with `expect(...)` on the current page.

## Not verified

macOS and Linux. `make` installers. CI (headless Linux needs `xvfb-run`). Running real Karate (needs a JDK) from a test.
