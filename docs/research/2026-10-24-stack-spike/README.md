# Stack spike: Electron Forge 8 + Solid 2 RC + TanStack Router + Tailwind v4

Date 2026-10-24. Windows 11, Node 24.15. Proves [ADR 0001](../../decisions/0001-solid-router-spa-without-tanstack-start.md) builds, runs and packages. Throwaway; the files here are a reference for the real scaffold (`node_modules`, `out/`, lockfile and lint configs not kept).

## Result

| Item | Result |
|---|---|
| Forge 8 Vite + TS scaffold, renderer swapped to Solid 2 | works |
| `@tanstack/solid-router` with hash history, `/` and `/runs/$id` | works |
| Tailwind v4 via `@tailwindcss/vite` | works |
| Signal + async `createMemo` inside `Loading` | works |
| Typed IPC: invoke + main→renderer stream; `contextIsolation`, no `nodeIntegration`, `sandbox` on | works |
| `@kobalte/core@2.0.0-alpha.4` Button | works with `overrides` |
| `npm run start` (dev) | works, about 2.5 s warm |
| `tsc --noEmit` | clean (TypeScript 6.0.3; 7.0.2 also clean) |
| `electron-forge package` | works: 21 s, `out/` 357 MB, `app.asar` 152 KB, renderer JS 46 KB gzip |

Checked with an auto-check in [src/main.ts](src/main.ts) (`SPIKE_AUTOCHECK=1`): main reads the DOM via `executeJavaScript`, clicks the Kobalte button and the router link, logs, then quits. Re-run on the packaged exe on 2026-10-24: Kobalte click changed `doubled: 2` → `doubled: 22`, route switched to `#/runs/42`, `nodeInRenderer: false`.

## Versions

The pinned set in ADR 0001 is still the newest. Extra pins: `vite` 8.3.5, `typescript` 6.0.3. See [package.json](package.json).

## Snags and fixes

- Scaffold: `npx create-electron-app@8.0.1 init <dir> --template=vite --typescript`. There is no `vite-typescript` template.
- The template writes `^` ranges. Strip them.
- Main and preload build to `.cjs` (`.vite/build/main.cjs`, `preload.cjs`).
- `@tanstack/solid-router@2.0.0-rc.9` declares unused Solid-1 peer deps: `npm ls` exits 1 (`ELSPROBLEMS`). Runtime is fine.
- Kobalte alpha peers exactly `solid-js@2.0.0-rc.16`. Fix in `package.json`:
  ```json
  "overrides": { "@kobalte/core": { "solid-js": "$solid-js", "@solidjs/web": "$@solidjs/web" } }
  ```
- Electron 45 downloads its binary on first launch, and again after each `npm ci`.
- In dev, Forge reloads the renderer once after start. Main-process code that runs on each page load must be idempotent.
- Solid 2 holds a signal write that feeds an async memo until the memo resolves (dev warning `SILENT_HOLD`). Use `isPending`, `latest` or `<Loading on>` to show stale values.
- The template ships `oxlint` and `oxfmt`; `oxfmt` 0.41.0 pulls 2 critical `npm audit` advisories (dev-only).

## Not verified

The window was never looked at by a person. `make` / Squirrel installer, HMR, Kobalte beyond Button, and whether `package` re-downloads Electron on a fresh install.
