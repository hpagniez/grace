---
status: proposed
date: 2026-10-29
decision-makers: Tusquito
---

# Bundle IBM Plex Sans and IBM Plex Mono as Grace's typefaces

## Context and Problem Statement

Grace draws its text in whatever system font Chromium picks, and its code, paths and console in Chromium's default monospace. [#107](https://github.com/hpagniez/grace/issues/107) asks for a graphic chart with a UI typeface and a monospace face, bundled with the app because it runs offline. A typeface is hard to change later: every size, line height and density in the spec ([2026-10-29-premium-ui](../specs/2026-10-29-premium-ui.md)) is tuned to it.

Which typefaces does Grace bundle, and how?

## Considered Options

1. IBM Plex Sans + IBM Plex Mono.
2. Geist + Geist Mono.
3. Public Sans + JetBrains Mono.
4. Segoe UI Variable + Cascadia Mono, the faces of Windows 11, bundled nothing.

The four were compared in the brainstorm on the same mockups, in the dark and light themes.

## Decision Outcome

Chosen: option 1, by the person.

- One superfamily: text and code share a skeleton, so the tree, details and console read as one system. Distinct figures (`0`, `1`, `l`, `I`) and tabular figures in both.
- Licence: SIL Open Font License 1.1. Bundling and redistributing inside an MIT app is allowed; the licence text ships with the app's other licences.
- Packaged as npm dependencies, pinned exact: `@fontsource-variable/ibm-plex-sans` (variable weight axis) and `@fontsource/ibm-plex-mono` (400 and 500). Only the Latin and Latin Extended subsets are imported. Vite copies the woff2 files into the build; nothing is fetched at run time, so the CSP stays `default-src 'self'`.
- Fallbacks: `"Segoe UI Variable", system-ui, sans-serif` and `"Cascadia Mono", Consolas, monospace`.

Why not the others:

- Option 2 is the crispest at small sizes, but it is the face of many current developer tools; Grace would look like a kit.
- Option 3 needs two families, and Public Sans is plainer than Plex at the same cost.
- Option 4 costs nothing, but is the least distinctive, and Cascadia Mono comes with Windows Terminal, not with every Windows 11, so it would be bundled anyway.

### Consequences

- Good: one look on every machine, offline; the type scale of the spec holds everywhere.
- Good: tabular figures for counts, durations and times.
- Bad: about 200 KB more in the app.
- Bad: two more dependencies to keep up to date (Dependabot sees them).
