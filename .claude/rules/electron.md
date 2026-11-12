---
paths:
  - "src/**"
  - "e2e/**"
  - "bench/**"
  - "test/**"
  - "*.config.*"
  - "package.json"
---

# Electron Grace

Electron Grace, the app that ships until the GPUI cut-over (`.claude/rules/gpui.md`): Electron + SolidJS 2 RC + Tailwind CSS v4, packaged with Electron Forge. Pin exact versions (no `^`) and upgrade the Solid set together.

- Keep `contextIsolation` on, `nodeIntegration` off, `sandbox` on. The renderer reaches Node only through the typed preload API.
- Forge dev reloads the renderer once after start: main-process code that runs per page load must be idempotent.
- Java and Karate are bundled, not taken from the user's machine, decided in [ADR 0006](../../docs/decisions/0006-bundle-jre-and-karate-jar.md). Run Karate as `java -cp "karate.jar;<project classpath roots>" io.karatelabs.Main run ...`, not `java -jar`, so `classpath:` resolves in Maven and Gradle layouts.
- End-to-end tests: Playwright Test (`@playwright/test` 1.64.1) with `_electron`, decided in [ADR 0004](../../docs/decisions/0004-playwright-for-end-to-end-tests.md). Launch the built app (`electron-forge package`, then `_electron.launch({ args: ['.'] })`), not the packaged exe: its `EnableNodeCliInspectArguments: false` fuse blocks Playwright; keep that fuse off. Run `npx electron --version` first so the binary download does not hit the test timeout. Reference: [Playwright spike](../../docs/research/2026-10-24-playwright-spike/README.md).
- Unit tests: Vitest (`vitest` 5.0.5, `jsdom` 30.1.3, `@solidjs/testing-library` 1.0.0-rc.2), decided in [ADR 0005](../../docs/decisions/0005-vitest-for-unit-tests.md). Copy the [Vitest spike](../../docs/research/2026-10-24-vitest-spike/README.md) config: `*.test.ts` runs in Node, `*.test.tsx` in jsdom. The jsdom project needs `resolve.conditions: ['browser', 'development']` and `test.server.deps.inline: [/solid/]`, or Solid loads its server build. Testing-library 0.8 is Solid 1; do not use it.

## Commands

| Command | What it does |
|---|---|
| `npm start` | Run the app in dev mode |
| `npm run bench` | Package the app and measure it: memory, CPU, start, smoothness, size. `-- --a <exe>` compares it with another build; `-- --release` adds the release-only measures (the release benchmark itself is in [Git workflow](../../docs/process/release.md)). Writes `bench-out/bench.json` and `bench.md` |
| `npm run make` | Build the Windows zip in `out/make/zip/win32/x64/` |
| `npm run screens:readme` | Package the app, then retake the README's screenshots in `docs/images/` |

On a machine with more than one screen, every e2e window opens on a screen that is not the main one, out of the way of the person working: `launch()` in `e2e/helpers.ts` moves it there before it shows. Launch the app in a test only through `launch()`. Only `window.spec.ts` keeps its windows on the main screen (`primaryScreen`), since it tests where the app places them. The bench opens its windows on the first screen that is not the main one too, so run the release benchmark with the same screen layout as the `machine` in `bench/limits.json`.

- End-to-end tests before a PR is ready: run those of the app the PR changes (`npm run e2e` for `src/`). Small: the spec files of the touched area. Feature and Project: the whole suite once, and paste its summary in the PR. CI runs no e2e on PRs into `develop`; PRs into `main` run it on CI once ready.
- A PR that changes what the app shows carries before and after screenshots (a Small PR only when the look changes). They live on the never-merged branch `feature/pr-screenshots`, as `pr-<N>/<name>-before.png` and `-after.png`, linked by `https://raw.githubusercontent.com/hpagniez/grace/<sha>/...`. Take GPUI Grace's on Linux under Xvfb, or on [the owner's machine](../../docs/process/owner-machine.md) with his go.
- A PR that adds a dependency states its size.

## Solid 2 rules

Most training data is Solid 1. Do not write Solid 1 code:

- Import the renderer from `@solidjs/web`, not `solid-js/web`. Stores come from `solid-js`, not `solid-js/store`. `jsxImportSource` is `@solidjs/web`. Vite plugin is `@solidjs/vite-plugin`, not `vite-plugin-solid`.
- No `createResource`: use an async `createMemo`. `Suspense` → `Loading`, `ErrorBoundary` → `Errored`.
- `onMount` → `onSettled`. `createEffect` takes a tracked compute function and an untracked apply function. No `on`, no `batch` (use `flush()`).
- A write that feeds an async memo is held until the memo resolves (dev warning `SILENT_HOLD`). Show stale values with `isPending`, `latest` or `<Loading on>`.
- Store setters receive a draft; no path setters, no `produce`. `mergeProps` → `merge`, `splitProps` → `omit`, `unwrap` → `snapshot`.
- `Index` → `<For keyed={false}>`. Default (keyed) `For` children get `(item, index)` with `item` the raw value and `index` an accessor. With `keyed={false}` they get `item` as an accessor and `index` as a plain number. `Context.Provider` → the context itself.
- No `use:` directives, `classList`, `on:`, `attr:`, `bool:`. Use `onClick`, not `onclick`.

Source: https://v2.solidjs.com/migration/from-solid-1

## UI conventions

Spec: [docs/specs/2026-10-29-premium-ui.md](../../docs/specs/2026-10-29-premium-ui.md).

- Colours only through the semantic tokens in `src/index.css`. Tailwind's default palette is removed: no raw palette class (`zinc-500`), no `dark:`, no `title=`. `src/renderer/palette.test.ts` enforces it.
- The two themes differ only in colours: the dark block of `src/index.css` may override only colour tokens, shadows and the backdrop, never a size, spacing or radius. `src/renderer/tokens.test.ts` enforces it.
- Shared class strings live in `src/renderer/buttons.ts` and `src/renderer/ui.ts`. Reuse them.
- Icons only through `Icon` with `lucide` icon data. Statuses only through `StatusMark`.
- Tooltips only through `Tooltip`, or `Cut` for text that may be cut. In a row, set `side` so a one-line tooltip stays on its row; a click on any tooltip closes it.
- A dialog keeps its content through its 100 ms exit (`--duration-fast`), then drops it.
