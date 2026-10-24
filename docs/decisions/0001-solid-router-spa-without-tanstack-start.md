---
status: accepted
date: 2026-10-24
decision-makers: Tusquito
---

# Use a TanStack Router SPA over Electron IPC, without TanStack Start

## Context and Problem Statement

The requested stack is Electron + TanStack Start + SolidJS v2 RC + Tailwind CSS. TanStack Start is a server-first framework (SSR, server functions, server routes). In Electron, the main process is already a Node backend that can spawn Karate, read files and watch reports. Does Start add anything in a desktop app?

## Considered Options

1. TanStack Router (`@tanstack/solid-router`) + Vite as a pure SPA; all backend work in the Electron main process behind typed IPC.
2. TanStack Start in SPA or static-prerender mode, with all data over IPC.
3. TanStack Start server running inside the main process (community examples: `spa5k/tanstack-start-electron`, `cheriot/electron-tanstack-demo`, both React).

## Decision Outcome

Chosen: option 1. Option 2 builds Start's server layer and then uses none of it. Option 3 duplicates IPC with HTTP, needs auth on a local port, and has no Solid or Solid 2 reference project.

Pinned versions (npm, 2026-10-24), upgraded together:

| Package | Version |
|---|---|
| `solid-js`, `@solidjs/web` | `2.0.0-rc.18` |
| `@tanstack/solid-router` | `2.0.0-rc.9` |
| `@solidjs/vite-plugin` | `3.0.0-next.51` |
| `tailwindcss`, `@tailwindcss/vite` | `4.3.3` |
| `electron` | `45.0.0` |
| `@electron-forge/cli`, `@electron-forge/plugin-vite` | `8.0.1` |

### Consequences

- Good: fewest moving parts; no local server port to secure.
- Good: file-based routing still available via `@tanstack/router-plugin`.
- Bad: no SSR or server functions. A desktop app does not need them.
- Bad: Solid 2 is a release candidate. UI libraries are alpha (Kobalte `2.0.0-alpha.4`, exact peer on `rc.16`) or untested. Expect adapters and breakage on RC bumps.
- Fallback if Solid 2 gaps block work: `solid-js@1.9.17` + `@tanstack/solid-router@1.170.43`.

Evidence and sources: [research/2026-10-24-stack.md](../research/2026-10-24-stack.md).
