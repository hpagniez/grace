---
status: accepted
date: 2026-10-24
decision-makers: Tusquito
---

# Use Vitest for unit tests

## Context and Problem Statement

[ADR 0004](0004-playwright-for-end-to-end-tests.md) picks Playwright for end-to-end tests and leaves unit tests open. Much of the app is pure logic that needs fast tests without launching Electron: parsing `karate-events.jsonl`, building the Karate command line, mapping exit codes. Solid components also need tests below the end-to-end level. Which framework runs unit tests?

## Considered Options

1. Vitest.
2. Jest with `babel-preset-solid` and `ts-jest`.
3. Node's built-in `node:test` runner.

## Decision Outcome

Chosen: option 1. The [Vitest spike](../research/2026-10-24-vitest-spike/README.md) ran it against the ADR 0001 stack on Windows: a Node test on a real Karate event sample and a Solid 2 component test in jsdom both pass. Vitest reuses the project's Vite and `@solidjs/vite-plugin`, so TypeScript, JSX and Solid compile the same way as in the app, with no second build pipeline. Option 2 needs its own Babel and TypeScript transforms beside Vite, and its ESM support is still behind a flag. Option 3 has no JSX or Solid transform and no DOM environment.

Pinned versions (npm, 2026-10-24), dev dependencies: `vitest` `5.0.5`, `jsdom` `30.1.3`, `@solidjs/testing-library` `1.0.0-rc.2`.

How to use it:

- Copy [vitest.config.mts](../research/2026-10-24-vitest-spike/vitest.config.mts) from the spike. It has two projects: `*.test.ts` runs in Node, `*.test.tsx` runs in jsdom with the Solid plugin.
- The jsdom project needs `resolve.conditions: ['browser', 'development']` and `test.server.deps.inline: [/solid/]`, or Solid loads its server build and `render` throws.
- Use `@solidjs/testing-library` 1.x. Version 0.8 targets Solid 1.
- Feed parser tests from the real event fixtures in the [CLI spike samples](../research/2026-10-24-karate-cli-spike/samples/), not hand-written JSON.

### Consequences

- Good: one transform pipeline for app and tests; Vite and the Solid plugin upgrade together with the tests.
- Good: fast (about 1.5 s for the spike) and Jest-compatible API, well known to agents.
- Bad: `@solidjs/testing-library` 1.x is a release candidate, like the rest of the Solid 2 set. Upgrade it with the Solid set and re-run the tests.
- Bad: jsdom is not Chromium. Layout, CSS and real IPC stay with the Playwright tests.
