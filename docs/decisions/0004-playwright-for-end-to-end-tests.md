---
status: accepted
date: 2026-10-24
decision-makers: Tusquito
---

# Use Playwright for end-to-end tests of the Electron app

## Context and Problem Statement

The app has no test framework yet. Agents build most of it, so they need a way to prove a change works in the real window: renderer, preload, IPC and main process together. Which tool drives the Electron app in tests?

## Considered Options

1. Playwright Test (`@playwright/test`) with its `_electron` API.
2. WebdriverIO with `wdio-electron-service`.
3. No end-to-end tests; only the main-process auto-check pattern from the stack spike (`executeJavaScript` from `main.ts`).

## Decision Outcome

Chosen: option 1. The [Playwright spike](../research/2026-10-24-playwright-spike/README.md) ran it against the ADR 0001 stack on Windows: launch, locators, clicks, routing, renderer and main-process `evaluate` all work. It reuses the project's Electron binary (no browser download), auto-waits, and gives traces and screenshots on failure. Electron's docs list it as a supported option. Option 2 adds a WebDriver layer and more config for the same coverage. Option 3 puts test code in the shipped main process and has no assertions or reports.

Pinned version (npm, 2026-10-24): `@playwright/test` `1.64.1`, dev dependency.

How to use it:

- Test the built app, not the packaged exe: build with `electron-forge package`, then `_electron.launch({ args: ['.'] })` from the project root (`main` points at `.vite/build/main.cjs`).
- Do not turn on the `EnableNodeCliInspectArguments` fuse to make the packaged exe testable. Playwright needs `--inspect`; shipped builds must not accept it.
- Run `npx electron --version` before the tests so the first-launch binary download does not eat the test timeout.

### Consequences

- Good: one tool covers renderer UI, IPC and main process, and agents can run it without a person watching the window.
- Good: same API as Playwright browser tests; well known to agents.
- Bad: `_electron` is marked experimental (supports Electron v21+). An Electron or Playwright bump can break it; upgrade both together and re-run the suite.
- Bad: the packaged exe, with its fuses and `asar`, is not covered. A packaging-only bug needs a manual smoke test.
- Unit tests (pure main-process logic such as parsing `karate-events.jsonl`) are not decided by this record.
