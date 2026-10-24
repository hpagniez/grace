# Vitest spike: unit tests for main-process logic and Solid 2 components

Date 2026-10-24. Windows 11, Node 24.15. Ran Vitest 5.0.5 against a copy of the [stack spike](../2026-10-24-stack-spike/README.md) (Vite 8.3.5, `@solidjs/vite-plugin` 3.0.0-next.51, Solid 2.0.0-rc.18). Supports [ADR 0005](../../decisions/0005-vitest-for-unit-tests.md). Throwaway; [vitest.config.mts](vitest.config.mts) and the two tests in [test/](test/) are a reference for the real setup.

Added dev dependencies, exact versions: `vitest` 5.0.5, `jsdom` 30.1.3, `@solidjs/testing-library` 1.0.0-rc.2 (the first line that declares Solid 2 peers; `latest` 0.8.10 is Solid 1).

## Result

| Item | Result |
|---|---|
| `events.test.ts`, Node environment: parse a real `karate-events.jsonl` sample, count depth-0 `SCENARIO_EXIT` | passes |
| `counter.test.tsx`, jsdom: render a Solid 2 component with `createSignal`, click, assert the update | passes |
| `npx vitest run`, both projects | 2 files, 2 tests, about 1.5 s |
| `tsc --noEmit` with the tests in place | passes |

## Snags

- Vitest resolves Solid's server build by default, so `render` throws `Client-only API called on the server side`. The jsdom project needs `resolve.conditions: ['browser', 'development']`.
- That is not enough on its own: Vitest externalizes `node_modules`, and externalized packages (`@solidjs/testing-library`) still load the server build through Node resolution. Add `test.server.deps.inline: [/solid/]`.
- Those two settings would also hand browser builds to main-process tests. The config splits two projects by file name: `*.test.ts` runs in Node, `*.test.tsx` runs in jsdom with the Solid plugin.

## Not verified

Coverage (`@vitest/coverage-v8`). Kobalte components and TanStack Router under jsdom. Async memos and `Loading` in component tests. macOS, Linux, CI.
