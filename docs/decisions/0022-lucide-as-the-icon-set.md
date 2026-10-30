---
status: accepted
date: 2026-10-29
decision-makers: Tusquito
---

# Use Lucide as Grace's icon set

## Context and Problem Statement

Grace has no icons: the tree uses the text characters `▸`, `▾` and `▶`, the recent list `×`, and statuses are coloured dots. [#107](https://github.com/hpagniez/grace/issues/107) asks for one icon set whose licence allows bundling, and its spec ([2026-10-29-premium-ui](../specs/2026-10-29-premium-ui.md)) needs a distinct shape for every test status, so a status is never shown by colour alone. An icon set is hard to change later: every screen draws from it.

Which icon set, and how does Grace draw it?

## Considered Options

1. Lucide (ISC).
2. Carbon (IBM, Apache 2.0), drawn alongside IBM Plex.
3. Phosphor (MIT).
4. Tabler (MIT).

The four were compared in the brainstorm at Grace's real sizes (16 px in the toolbar, 14 px in the tree), in the dark and light themes.

## Decision Outcome

Chosen: option 1, by the person.

- Outline icons on a 24 px grid with round joins, about 1,600 of them, including every shape the spec needs: `circle-check`, `circle-x`, `circle-slash`, `circle-dashed`, `loader-circle`, `play`, `square`, `rotate-ccw` and the rest.
- Licence: ISC. Bundling and redistributing inside an MIT app is allowed; the licence text ships with the app's other licences.
- Packaged as the framework-free `lucide` npm package, pinned exact. Grace imports each icon's data by name and draws it with one small Solid component, so only the icons used are bundled. `lucide-solid` is not used: it targets Solid 1 (`solid-js/web`), and Grace is on Solid 2.
- Icons are inline SVG in `currentColor`, so they take the theme's colours without extra files and the CSP is unchanged.

Why not the others:

- Option 2 matches Plex's geometry best, but its square, technical shapes read colder at 14 px, and the person preferred Lucide.
- Option 3 is softer and has filled twins, but adds weights Grace does not need.
- Option 4 is the largest set, but reads heavier at 14 px.

### Consequences

- Good: one consistent icon language; a shape per status.
- Good: a tree-shaken bundle of a few KB.
- Bad: one more dependency to keep up to date; an icon renamed upstream breaks the build on an upgrade, which is the moment to notice it.
