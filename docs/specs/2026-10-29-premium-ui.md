# Premium look and feel with a defined graphic chart

Date 2026-10-29. Status: approved. Covers [#107](https://github.com/hpagniez/grace/issues/107).

## Intent

Grace works, but it looks like a prototype. It has no design system: `src/index.css` holds two animations, scrollbar colours and cursors, and the components in `src/renderer/` pick raw Tailwind palette classes one by one (about 150 `zinc`, 45 `red`, 21 `sky`, 21 `green`, 16 `amber`, 2 `emerald`, plus 74 `dark:` variants). There is no chosen typeface (the system font) and no icon set (`▸`, `▶`, `×` as text). Each screen solves the same problem (a status, a chip, an empty list) in its own way.

Karate's own GUI runners are paid. Grace is free, and it should not look like the cheap option. This spec defines a graphic chart (colours, type, spacing, icons, motion, brand) that every screen follows, built as tokens, and a screen-by-screen pass that gives each screen a clear hierarchy, honest empty, loading and error states, and full keyboard and contrast accessibility in both themes.

## Who and what success looks like

People who run Karate API suites every day, often for hours, with the app beside an editor; some in Windows' dark theme, some in light.

Success:
- Open Grace in either Windows theme and it looks deliberate: one typeface, one icon set, one set of colours taken from the logo, the same component for the same job on every screen.
- Tell a test's status at a glance and without colour: every status has its own icon shape and an accessible name, and the word where there is room.
- Every text and control passes WCAG 2.2 AA contrast in both themes; every control shows where the keyboard focus is.
- Pick Light, Dark or Follow Windows in Settings, and the whole window, title bar included, follows at once.
- A person who knows Bruno or a JetBrains IDE finds the layout familiar and calmer.

## Goals, non-goals, anti-goals

**Goals:** the graphic chart (R1 to R7), the theme setting (R8), shared styles (R9), accessibility (R10) and the screen pass (R11 to R22) below.

**Non-goals:**
- New features or new data on any screen. The pass changes how things look, are labelled and are ordered, not what Grace does.
- A new layout. The toolbar, progress, filter bar, tree beside details, and bottom panel stay where they are.
- Arrow-key navigation in the tree or tables (a feature of its own), a resizable tree/details split, a frameless window with a custom title bar.
- Animating height (tree expansion, panels). Motion stays on transforms, opacity and colour.
- A designed Windows high-contrast (forced colours) theme. It must stay usable (status shapes stay distinct), nothing more.
- Recolouring the logo or the mascot, a mascot that reacts to results, a mascot in empty states (ADR 0013 keeps it to the home screen and the README).
- A component library (see Approaches).

**Anti-goals** (failure even if everything else works):
- A status shown by colour alone.
- Any text under WCAG 2.2 AA contrast in either theme.
- The wrong theme flashing at start before the Appearance setting applies.
- A raw palette class (`zinc-500`, `red-600` …) left in `src/renderer/`.
- A font or icon fetched from the network at run time: Grace runs offline.
- Motion when Windows asks for reduced motion.
- A `data-testid`, an accessible name an e2e test relies on, or a keyboard path lost on the way.

## Constraints

- Electron 45 (Chromium), SolidJS 2 RC, Tailwind CSS v4. Solid 2 rules of `AGENTS.md`.
- New dependencies: the typeface (ADR 0021) and the icon set (ADR 0022), pinned exact, nothing else.
- Fonts and icons are bundled into the app; the CSP stays `default-src 'self'`.
- `contextIsolation`, `sandbox` and `nodeIntegration: false` stay. The Appearance setting reaches main through a typed preload call.
- Every save of `settings.json` goes through `update()` and keeps the other keys.
- No backward compatibility (`AGENTS.md`): no migration, no fallback to the old classes.
- Other branches change `src/renderer/` in parallel. The app-updates work (PR #104 and its build) adds a status bar, a Settings page and an update dialog; the build syncs with `develop` before each PR and styles whatever has landed.

### What the app does today

- `src/index.css`: `@import "tailwindcss"`, a `blink` and a `slide` animation in `@theme`, thin scrollbars coloured `zinc-400`/`zinc-500`, cursor rules. Dark mode is Tailwind's default `dark:` variant, which follows `prefers-color-scheme`, which follows Windows.
- `src/main.ts`: the window's `backgroundColor` is `#18181b` or `#ffffff` from `nativeTheme.shouldUseDarkColors`. `index.html` has an inline `#startup` screen with the same two colours and a CSS spinner.
- `src/renderer/buttons.ts` is the only shared style: `btn`, `btnSmall`, `primary` (green), `danger` (red). Everything else is inline class strings. `nodes.ts` maps a status to a coloured dot (`STATUS_DOT`) and a word (`STATUS_LABEL`); `FLAKY_TONE` is the amber of everything flaky.
- Statuses (`src/shared/model.ts`): `notRun`, `pending`, `running`, `passed`, `failed`, `interrupted`; steps add `skipped`. Run outcomes: `finished`, `cancelled`, `failed`. HTTP groups: `1xx` … `5xx`, `none`.
- Dialogs are native `<dialog>` with `showModal()` (`PropertiesDialog`, `ClearHistory`). The environment box is an `<input list>` with a `<datalist>`.
- Tooltips are native `title` attributes (9 in `src/renderer/`); `History.test.tsx`, `Home.test.tsx`, `HttpCalls.test.tsx` and `Tree.test.tsx` read some of them.
- Tests that read classes: `Banner.test.tsx` (`text-sky-400`, `text-emerald-400`), `Console.test.tsx` (`text-red-400`, `text-amber-400`), `FilterBar.test.tsx` (`border-green-600`, `border-red-600`, `opacity-40`), `HttpCalls.test.tsx` (badge colours), `Tree.test.tsx` (animation classes), `e2e/console.spec.ts` (console colour equals the dark app background), `e2e/tags.spec.ts` (`opacity-40`), `e2e/scrollbar.spec.ts`.

## Approaches considered

The direction was chosen in the brainstorm from four mockups of the same running suite, after reading Bruno and three Dribbble shots the person picked, and the advice of two design tools (ui-ux-pro-max: true dark, "code dark + run green", JetBrains Mono or IBM Plex; taste-skill, from a study of real product screens: colour on data not chrome, flat surfaces, near-black primary buttons, no glass, glow or gradient).

**Direction:**
1. **A, Logbook:** Bruno sharpened. Panels a step darker than the window, brand green only on Run and on passed, figures in the mono, a selection marked by one thin line. **Chosen for the dark theme's colours.**
2. B, Out of Dark: black with one soft green glow, floating rounded panels, pastel figure tiles. Most brand-forward, hard to carry into a light theme.
3. C, Aurora: frosted glass over a violet gradient (OBS concept). Costs GPU in long runs, AA contrast on glass is hard to hold, and both tools flag it as a dated look.
4. **D, Parchment sheet:** light first; a parchment frame holds the toolbar, the work sits on a white sheet with 16 px corners, a near-black primary with the green play mark. **Chosen for the light theme's colours, and for the structure of both themes** (see Design decisions already made).

A and D take their colours from the logo (ADR 0013: green is a passing test, off-white is logbook paper): the logo is not recoloured. The [mockup](2026-10-29-premium-ui-mockup.png) shows the chosen pair on the same screen, with IBM Plex and Lucide; where it and the requirements below differ, the requirements win. The mockup predates two decisions the person made after seeing the built app: the dark theme has the same structure as the light one (the inset, rounded sheet), and the primary buttons use the progress bar's passed green in both themes. Its dark half (flush panels) and its near-black Run all show neither.

**How colours reach components:**
1. **Semantic tokens whose values switch with the theme.** `@theme` defines `--color-surface`, `--color-ink`, `--color-failed` …; `:root` holds the light values and a `prefers-color-scheme: dark` block the dark ones. A component writes `bg-surface text-ink` once; no `dark:` class remains. The Appearance setting only sets `nativeTheme.themeSource`. **Chosen.**
2. Light and dark token pairs with `dark:` (`bg-surface dark:bg-surface-dark`): two classes per element, and a missed `dark:` breaks one theme silently.
3. Component classes in CSS (`@layer components`): moves styling away from components. Kept only as shared TypeScript class strings (R9), as `buttons.ts` does today.

**Component library:** none. Kobalte's Solid 2 build is `2.0.0-alpha.4`, pinned to `solid-js@2.0.0-rc.16` (Grace is on rc.18) and proven only for its Button. Native `<dialog>`, the Popover API, `<input list>` and ARIA patterns already cover Grace's needs. **Chosen: native elements.**

**Typeface:** IBM Plex Sans + IBM Plex Mono (**chosen**), Geist + Geist Mono, Public Sans + JetBrains Mono, Segoe UI Variable + Cascadia Mono. See ADR 0021.

**Icon set:** Lucide (**chosen**), Carbon, Phosphor, Tabler. See ADR 0022.

## Requirements

### R1. Colour tokens

1. `src/index.css` defines every colour as a semantic token in `@theme` and removes Tailwind's default palette (`--color-*: initial`), so a raw palette class compiles to nothing. `transparent`, `current`, `white` and `black` stay.
2. Light values sit on `:root`; dark values in `@media (prefers-color-scheme: dark) { :root { … } }`. No component uses `dark:`. The two themes differ only in colours: the dark block may set `color-scheme`, `--color-*` tokens, `--shadow-float` and `--backdrop`, and nothing else (no size, spacing, radius or layout token). A unit test lists every custom property of the block and fails on any other.
3. The tokens, with their values (light = direction D's colours, dark = direction A's colours):

| Token | Use | Light | Dark |
|---|---|---|---|
| `ground` | window frame: toolbar row, status bar, behind the sheet | `#f3efe6` | `#151821` |
| `surface` | panels, the sheet, rows | `#ffffff` | `#101217` |
| `raised` | dialogs, menus, popovers, tooltips | `#ffffff` | `#1b1e26` |
| `sunken` | console, code and log boxes, request and response bodies | `#f8f6f1` | `#0b0d11` |
| `hover` | a row or ghost control under the pointer | `#f5f2eb` | `#191c23` |
| `selected` | the selected tree row, HTTP call, settings entry | `#ece7dc` | `#1f232c` |
| `line` | structural borders and row rules | `#e5e0d5` | `#262a33` |
| `control` | borders of inputs, checkboxes, radios (3:1) | `#8a8478` | `#6a707e` |
| `ink` | text | `#151821` | `#f1efe9` |
| `muted` | secondary text, meta, placeholders | `#5b5e66` | `#9fa2ac` |
| `accent` | the active tab line, the selection bar, the included tag chip, the splitter on hover and drag | `#151821` | `#3fcf6e` |
| `on-accent` | text and icons on `accent` | `#ffffff` | `#06210f` |
| `primary` | the primary buttons' fill: `Run all`, `Open folder…`, a dialog's confirm button; the same green as the progress bar's passed segment, so equal to `passed` in each theme | `#1b7a4c` | `#3fcf6e` |
| `on-primary` | text and icons on `primary` | `#ffffff` | `#151821` |
| `primary-hover` | `primary` under the pointer (solid) | `#156640` | `#5fd98a` |
| `on-danger` | text and icons on a `failed` fill (the `danger` button) | `#ffffff` | `#151821` |
| `brand` | the logo's strike | `#3fcf6e` | `#3fcf6e` |
| `focus` | the focus ring | `#151821` | `#3fcf6e` |
| `passed` | passed; HTTP 2xx | `#1b7a4c` | `#3fcf6e` |
| `failed` | failed, errors, destructive actions; HTTP 5xx and no response | `#c22a2f` | `#f2666f` |
| `skipped` | not run, pending, skipped; HTTP 1xx | `#64676f` | `#9a9ea8` |
| `running` | running, the waiting stripe; HTTP 3xx | `#2a68b0` | `#62a8f2` |
| `cancelled` | interrupted, cancelled, warnings, notices, flaky; HTTP 4xx | `#8f5a00` | `#e5aa45` |

4. Each status has a solid tint token for badges, error boxes, notices and the flaky chip: its colour mixed at 12 % (light) or 16 % (dark) into `surface`, written out as a value, never as an opacity modifier, so a tinted badge reads the same on `surface`, `selected`, `ground` or `hover`. Its text is the status token itself.

| Token | Light | Dark |
|---|---|---|
| `passed-tint` | `#e4efea` | `#183025` |
| `failed-tint` | `#f8e5e6` | `#341f25` |
| `skipped-tint` | `#ecedee` | `#26282e` |
| `running-tint` | `#e5edf6` | `#1d2a3a` |
| `cancelled-tint` | `#f2ebe0` | `#322a1e` |

5. Contrast, checked by a unit test that reads the values from `src/index.css` (R1.3, R1.4): `ink` and `muted` at 4.5:1 or more on `ground`, `surface`, `sunken`, `hover`, `selected` and `raised`; each status token at 4.5:1 on `surface` and on its own tint, and 3:1 on `selected`; `on-accent` at 4.5:1 on `accent`; `on-primary` at 4.5:1 on `primary` and on `primary-hover`; `primary` equal to `passed`; `on-danger` at 4.5:1 on `failed`; `control` and `focus` at 3:1 on `surface` and `ground`. All pass with the values above (`on-primary` 5.34:1 on `primary` and 6.98:1 on `primary-hover` in light, 8.75:1 and 9.94:1 in dark; light lowest: `passed` on its tint 4.53:1; dark lowest: `failed` on its tint 5.05:1; `on-danger` 5.73:1 light, 5.83:1 dark).
6. No text is dimmed with `opacity` except disabled controls (R10.4).
7. Scrollbars: thumb `control` at 60 %, track transparent.
8. `src/shared/theme.ts` holds the two `ground` values for main's `backgroundColor` and `index.html`'s startup screen; a unit test checks they equal the CSS tokens.

### R2. Type

1. UI text: IBM Plex Sans (variable, weights 400 to 600). Code, paths, HTTP, console, step text and figures that update: IBM Plex Mono (400, 500). Fallbacks: `"Segoe UI Variable", system-ui, sans-serif` and `"Cascadia Mono", Consolas, monospace`. Bundled with the app (ADR 0021): Latin and Latin Extended only, `font-display: block` so text never repaints in another face after the first frame.
2. The scale, set as Tailwind's `--text-*` tokens:

| Class | Size / line height | Use |
|---|---|---|
| `text-2xs` | 11 / 16 | chips, badges, count pills (the floor: nothing smaller) |
| `text-xs` | 12 / 16 | meta, table cells, console, code boxes |
| `text-sm` | 13 / 20 | body: tree, details, controls, lists |
| `text-base` | 14 / 20 | buttons in dialogs, the project name |
| `text-lg` | 16 / 24 | details title, dialog and settings titles |
| `text-xl` | 20 / 28 | the home screen heading |

3. Weights: 400 body, 500 labels and active tabs, 600 titles and the primary button (500 in the mono, which has no 600). No 700.
4. Every count, duration, time and progress figure uses `tabular-nums`. Durations and elapsed time are in the mono.
5. Sentence case everywhere; no uppercase tracked labels.

### R3. Spacing, radius, borders, elevation, density

1. Spacing stays on Tailwind's 4 px scale. Density: toolbar and filter controls 28 px tall; tree rows, small buttons and chips 24 px; HTTP and history rows 28 px; bottom panel tab bar 32 px; panel padding 12 px; gaps 4, 8, 12, 16.
2. Radius tokens: `rounded-sm` 4 px (rows, chips, tags), `rounded-md` 6 px (buttons, inputs), `rounded-lg` 8 px (boxes: error, code, flaky proof), `rounded-xl` 12 px (dialogs, menus, popovers), `rounded-full` (status badges, count pills).
3. The sheet, the same in both themes: everything under the toolbar row (progress, filter bar, notice, tree, details, bottom panel) sits on a `surface` sheet inset 8 px from the window's left, right and bottom edges (above the status bar when there is one), with a 16 px radius and a 1 px `line` edge, on the `ground` frame. Set by three tokens, `--sheet-inset`, `--sheet-radius` and `--sheet-side`, which the dark block does not override (R1.3).
4. Borders: 1 px `line` between areas and under rows; a border or a shadow at rest, never both; what floats (dialogs, menus, popovers, tooltips) has both.
5. Elevation: only what floats casts a shadow (dialogs, menus, popovers, tooltips): `--shadow-float`, light `0 12px 32px rgb(21 24 33 / 0.14), 0 1px 2px rgb(21 24 33 / 0.06)`, dark `0 16px 40px rgb(0 0 0 / 0.5)`. A dialog's backdrop is `ink` at 40 % (light) or black at 60 % (dark).

### R4. Icons

1. Lucide (ADR 0022), drawn from its icon data by one `Icon` component; only the icons used are bundled. Outline, `stroke-width` 2, `currentColor`.
2. Sizes: 16 px in the toolbar, buttons, tabs and dialogs; 14 px in the tree, chips, inline text and status marks; the home screen's open button 16 px.
3. A decorative icon is `aria-hidden`. An icon-only button has an `aria-label` and a tooltip with the same words.
4. Status marks, one shape each, through one `StatusMark` component that also carries the accessible name (`STATUS_LABEL`):

| Status | Icon | Colour |
|---|---|---|
| not run | `circle` | `skipped` |
| pending | `circle-dashed` | `skipped` |
| running | `loader-circle`, turning | `running` |
| passed | `circle-check` | `passed` |
| failed | `circle-x` | `failed` |
| interrupted, cancelled | `circle-slash` | `cancelled` |
| skipped (a step) | `circle-minus` | `skipped` |

5. Action icons: Home (the logo mark, R6.2), Open `folder-open`, Refresh `refresh-cw`, Properties `sliders-horizontal`, Report `file-text`, Re-run failed `rotate-ccw`, Cancel `square`, Run and Run all `play`, expand `chevron-right` (turns 90° when open), folder `folder`/`folder-open`, feature `file-text`, Console `square-terminal`, History `history`, HTTP `globe`, Settings `settings`, Copy `copy` (then `check` once copied), Remove `x`, flaky `shuffle`, warning `triangle-alert`, error `circle-alert`.
6. The builder checks every name exists in the pinned Lucide version and may swap a missing one for the closest shape, recorded in the plan.

### R5. Motion

1. Tokens: `--ease-standard: cubic-bezier(0.2, 0, 0, 1)`; `--duration-fast: 100ms` (hover, press, colour); `--duration-base: 160ms` (chevron turn, tab line, dialog and popover enter); exits 100 ms. The progress fill keeps its 300 ms ease-out; the spinner turns once a second; the waiting stripe and the console cursor keep their timings.
2. Only `transform`, `opacity` and colours move. A dialog enters with a fade and a scale from 0.98; its backdrop fades. Tree children appear without a height animation; only the chevron turns.
3. Under `prefers-reduced-motion: reduce`, one rule in `src/index.css` turns every transition and animation off. The spinner then shows still, the stripe stays still, the cursor stays solid (as today).

### R6. Logo and mascot

1. The logo files and the mascot do not change (ADR 0013).
2. The toolbar starts with the logo mark (`assets/icon-small.svg`, 20 px) as the Home button, with the accessible name and tooltip "Home".
3. The startup screen shows the logo (`assets/icon.svg`, 48 px), "Grace" and the spinner, on `ground` in `ink`, in the active theme.
4. The mascot stays on the home screen only, as today.
5. Empty states show a 24 px Lucide icon in `muted`, one sentence and, when there is one, the action that fills them. No mascot, no illustration.

### R7. Statuses never by colour alone

1. Every status in the tree, details, steps, progress, history, HTTP list and flaky proof is shown by a `StatusMark` (R4.4) with an accessible name, or by a badge with the icon and the word.
2. HTTP status codes show the number in a tinted badge (R1.4) of their group's colour; the group's name is in the tooltip and the call details.
3. Tag chips show their state by shape too: an included tag has a `check` icon and the `accent` fill; an excluded tag has a `minus` icon, an outline and a strike-through; an unfiltered tag is plain. The spoken names (`SPOKEN` in `FilterBar.tsx`) stay.

### R8. Theme: Follow Windows, Light or Dark

1. Settings › Appearance (a new entry in the Settings page that app-updates adds) has a `Theme` radio group: **Follow Windows** (default), **Light**, **Dark**. A change applies at once, with no restart, and is saved.
2. `settings.json` gets one key, `appearance`: `{ theme: "system" | "light" | "dark" }`. A missing or malformed key or value reads as `"system"`. Saved through `update()`.
3. Main sets `nativeTheme.themeSource` from the saved value before the window is created, so the first frame, the startup screen and the title bar are already in the right theme, and again on each change. On each `nativeTheme` `updated` event main sets the window's background colour to the new `ground` (R1.7).
4. New typed preload calls: `getAppearance()` and `setAppearance(theme)`. Main rejects any other value.
5. If the Settings page is not on `develop` when the build starts, R8 waits for it (see Delivery); everything else does not.

### R9. Shared styles and components

1. Shared class strings live in two files. `src/renderer/buttons.ts` keeps its name and exports (`btn`, `btnSmall`, `primary`, `danger`) with token values, and adds `btnIcon` and `ghost`. A new `src/renderer/ui.ts` holds the rest: `input`, `chip` (with its states), `tab`, `badge`, `box` (error, code), `dialog`. Every component uses them for those jobs. Keeping `buttons.ts` spares the branches built in parallel a broken import.
2. Components: `Icon` (R4.1), `StatusMark` (R4.4), `EmptyState` (R6.5), `Notice` (R20), `Tooltip` (R9.4). `nodes.ts` drops `STATUS_DOT` and `FLAKY_TONE`; `STATUS_LABEL` stays.
3. A unit test fails if any `.ts` or `.tsx` file in `src/renderer/` holds a raw palette class (`slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose` followed by `-` and a number), a `dark:` variant, or a `title` attribute (R9.4).
4. Styled tooltips. One `Tooltip` replaces every native `title` in the renderer:
   - It shows 500 ms after the pointer rests on its target, and at once when the target gets keyboard focus (`:focus-visible`). It hides when the pointer leaves both the target and the tooltip, on blur, on a scroll that moves its target, on a press of its target or a click inside it, and on Escape, which closes only the tooltip, not a dialog under it (WCAG 2.2 "Content on hover or focus": dismissible, hoverable, persistent). After a press it does not reopen at the spot just pressed. One tooltip at a time.
   - It is a `popover="manual"` element in the top layer, so it shows over dialogs. It sits 6 px below its target, flips above when there is no room, and stays 8 px inside the window. In a row a one-line tooltip stays on its row: it sits 6 px beside its target (left or right, flipping when there is no room), or over its own cut text, where a click within the text reaches the text; an `over` tooltip that wraps onto a second line can reach the next row. A click on any tooltip closes it.
   - Style: `raised`, `ink` text at 12 px, `rounded-md`, 1 px `line`, `--shadow-float`, padding 4 × 8 px, at most 320 px wide, wrapping. It fades in over `--duration-fast`, with no motion under reduced motion.
   - `role="tooltip"`; the target points to it with `aria-describedby` when its text adds to the target's accessible name. An icon-only button's name stays its `aria-label`.
   - A disabled control still shows its tooltip on hover (the tooltip listens on a wrapper, since Chromium sends no pointer events to a disabled button).
   - Used for: every icon-only button, the logo Home button, truncated text (the project path, test names, URLs, "What ran"), status marks without their word, HTTP badges (R7.2), missing recent folders, and the reasons on disabled controls that have one today.

### R10. Keyboard and accessibility

1. Every interactive element shows a focus ring on `:focus-visible`: 2 px `focus`, offset 2 px; inset (offset −2 px) on rows inside scrolling lists. No `outline: none` without this ring.
2. A focused row is never hidden under a sticky header (`scroll-margin`), per WCAG 2.2 "Focus not obscured".
3. Pointer targets are at least 24 × 24 px (WCAG 2.2 "Target size"), the tree's run button and the recent list's remove button included.
4. Disabled controls are at 40 % opacity and keep their accessible names; their tooltip says why when it is not obvious (as today).
5. Nothing that works by keyboard today stops working; no new keyboard trap.

### R11. Startup screen

`index.html`'s `#startup`: the logo, "Grace" in Plex at 14 px, the spinner, centred on `ground` with `ink`, in the theme main set (R8.3). Its colours come from the same values as R1.7. Under reduced motion the spinner is still (as today).

### R12. Home

1. Layout as today (mascot, heading, sentence, Open button, recent list), left-aligned in a 540 px column, on `ground` in both themes (no sheet: there is no project).
2. First run (no recent folders): the heading "Open a Karate project", the sentence "Pick the folder that holds your .feature files. Grace brings its own Java and Karate.", and the primary button `Open folder…` with the `folder-open` icon.
3. Recent folders: a titled list ("Recent folders") in one `surface` box with row rules. Each row: `folder` icon, folder name (500), its path in the mono `muted` below. A missing folder: name in `muted`, a `cancelled` badge with `triangle-alert` and "Not found", row disabled with the tooltip "Folder not found". The remove button is an icon button (`x`, "Remove from list"), shown on every row, 24 × 24.
4. A notice (folder not found and the like) shows as a `Notice` above the column (R20).

### R13. Toolbar

1. One 44 px row on `ground`, controls 28 px tall, in this order: the logo Home button · the project (folder name in 500, layout chip, full path in the mono `muted`, truncated from the start with the full path in the tooltip) · space · `Env` box · `Threads` box · `Properties…` · a 1 px divider · Refresh (icon only) · `Open…` · divider · `Report` · `Re-run failed` with its count in a pill once it is above 0 · `Cancel` · `Run all` (primary, rightmost).
2. `Run all` is the only `primary` button on the screen: `primary` fill (the progress bar's passed green) with `on-primary` text and an `on-primary` `play` icon, in both themes.
3. Each button keeps its `data-testid`, its label and its disabled rules.

### R14. Progress

1. A 6 px rounded bar with 2 px gaps between segments: `passed`, `failed`, then `cancelled` for interrupted tests; the waiting stripe in `running` (as today).
2. Below it, one line: `done / total` in the mono; `StatusMark` + count + word for passed and failed; elapsed time in the mono; the run's environment chip; the outcome with its icon (`Running…` with the turning `loader-circle`, `Finished` with `circle-check`, `Cancelled` with `circle-slash`, `Run failed` with `circle-alert`).
3. Before any run of the project, the line reads `<total> tests · Not run yet` instead of `0 / <total>` with zero counts. The `data-testid`s and the `progressbar` values stay.

### R15. Filter bar

1. Tag chips (R7.3), 24 px, `rounded-sm`; `Clear` as a ghost button; the match counter in `muted`; `Hide non-matching` as a native checkbox with `accent-color: var(--color-accent)` and a visible label.
2. A non-matching tree row is de-emphasised without opacity (R1.6): its label, icons and status mark in `muted`, its tags in the plain chip. It keeps `data-match="false"`, which tests check instead of the `opacity-40` class.

### R16. Tree

1. Rows 24 px, `rounded-sm`, 16 px indent per level. In each row: the chevron (folders, features, outlines), the `StatusMark`, a `folder` or `file-text` icon for folders and features, the label, a feature's title in `muted`, tags as chips, the flaky chip (`shuffle` + "flaky" in `cancelled`), and the run button.
2. Hover: `hover` background. Selected: `selected` background and a 2 px `accent` bar on the left edge. Focus: the inset ring (R10.1).
3. The run button (`play`, "Run <name>") is 24 × 24, `muted` until the row is hovered, focused or selected, then `passed`; disabled while busy. It is always in the tab order.
4. Empty and loading: `Finding tests…` with the turning `loader-circle`; `No feature files found` as an `EmptyState` (`file-search`) with the sentence "Grace looks for .feature files under the project folder."; a dry-run error as a `Notice` of type error with the message.

### R17. Details

1. Leaf: the name (`text-lg`, 600), a status badge (icon + word, tinted), then a meta line in `muted`: location in the mono, duration in the mono, tags as chips.
2. A failed test's error: a `failed` tinted box (`rounded-lg`) titled with `circle-alert` and "Error", the message in the mono below.
3. Steps: one row each with a 14 px `StatusMark`, the keyword in 500, the text in the mono, the duration right-aligned in the mono; the `Background` chip; the log toggle as a ghost button with a turning `chevron-right` and "Log"; the log in a `sunken` box. "Steps are available when the feature finishes" in `muted` with the `loader-circle` while it runs.
4. Group: the name, its status badge, its location, then the counts as one line of `StatusMark` + figure + word per status, figures in `tabular-nums`.
5. Nothing selected: an `EmptyState` (`mouse-pointer-click`) with "Select a test to see its details".
6. The flaky proof boxes: `cancelled` tint, `rounded-lg`, the heading with `shuffle`, each run row with its `StatusMark`.

### R18. Bottom panel

1. The splitter: a 1 px `line` with a 6 px hit area; on hover and drag the line becomes 2 px `accent`. Keyboard resizing stays as today.
2. Tabs: 32 px, each with its icon (R4.5), label, and count in a pill; the active tab in `ink` 500 with a 2 px `accent` line under it; others in `muted`. `Clear history` stays at the right of the History tab.
3. Console: `sunken` background in both themes (it now follows the theme), the mono at 12 px. The banner (F24): "Grace" in `accent` weight 500 (dark) or `ink` 500 (light; the mono has no 600, R2.1), labels in `muted`, "Ready." in `passed`. Lines with ` ERROR ` in `failed`, ` WARN ` in `cancelled`.
4. History: a table with 12 px `muted` sentence-case headers on `surface` (sticky, no band), 28 px rows with `line` rules, no zebra; Threads and Duration right-aligned in `tabular-nums`; Result with a `StatusMark` and its words; tag cells as R7.3 chips (included and excluded); the row actions as small ghost buttons with icons (`file-text` Report, `play` Re-run, `rotate-ccw` Re-run failed). Empty: an `EmptyState` (`history`) with "No runs yet. Runs appear here when they finish."
5. HTTP: the group filters as chips with counts (pressed: `selected` fill and `ink` text, plus the check icon); rows 28 px with the status badge (R7.2), the method in the mono 500, the URL in the mono, the test in `muted`, attempt and via in `muted`, the time right-aligned. Selected row: `selected` and the `accent` bar. Empty states as today's sentences in an `EmptyState` (`globe`).
6. Call details: the method and URL as the title in the mono; the status badge, the time; the test and its location in `muted`; `Copy as cURL` as a `btnSmall` with `copy`, turning to `check` and "Copied" for 2 s; `Request` and `Response` as 12 px 500 section titles; headers and bodies in `sunken` boxes; the no-response error in a `failed` box.

### R19. Dialogs

1. Properties and Clear history (and the app-updates dialogs once they land) use the `dialog` style: `raised`, `rounded-xl`, `--shadow-float`, 1 px `line`, 20 px padding; the title in `text-lg` 600; the footer right-aligned with the primary or destructive button first and `Cancel` last, as today; the backdrop of R3.5; the enter motion of R5.2.
2. Properties: the environment list as a selectable list (`selected` + `accent` bar), inputs in the `input` style (`control` border, `surface` fill, 28 px), errors in `failed` with `circle-alert` under the field they belong to, the secrets note in `muted` with a `lock` icon.
3. Clear history: `Delete` in the `danger` style (`failed` fill, `on-danger` text).

### R20. Notices and inline messages

1. One `Notice` component for the app-wide notice bar and the tree's error: a tinted box (`cancelled` for warnings, `failed` for errors) with its icon (`triangle-alert`, `circle-alert`) and the text, `role="status"` (warning) or `role="alert"` (error). It keeps `data-testid="notice"`.
2. Inline field messages (environment hint and error, property errors, add-environment error): 12 px, the icon, `muted` for hints, `failed` for errors, linked with `aria-describedby` as today.

### R21. Status bar and Settings (from app-updates)

Once on `develop`: the status bar is 24 px on `ground` with `muted` 12 px text and 14 px icons, the update marker as a `running` tinted badge. The Settings page uses a left list in the R18.5 row style and sections with `text-lg` titles; its controls use R9. Settings › Appearance (R8) follows the same style.

### R22. Before and after record

`docs/checks/<date>-premium-ui.md` holds screenshots of each screen in both themes, before (from `develop` before the first build commit) and after: startup, home (first run; with recent folders, one missing), project idle, a run in progress, a finished run with a failed test selected, a group selected, the filter bar on, Console, History, HTTP with call details, Properties dialog, Clear history dialog, a notice, and Settings once it exists.

## Design decisions already made

- Dark theme from direction A, light theme from direction D; colours from the logo, the logo unchanged (the person).
- IBM Plex Sans + IBM Plex Mono; Lucide icons (the person).
- Settings › Appearance with Follow Windows (default), Light, Dark (the person).
- Native elements, no component library (the person).
- Semantic tokens whose values switch with the theme, no `dark:` (the person).
- Styled tooltips replacing the native `title` (the person, after the builder check).
- The two themes differ only in colours: the dark theme has the light theme's structure, the inset rounded sheet included, and the dark block of `src/index.css` overrides colour tokens only (the person, after seeing the built app).
- The primary buttons use the progress bar's passed green in each theme (`#1b7a4c` with white text in light, `#3fcf6e` with ink text in dark), not near-black and not the logo's green in light (the person, after seeing the built app).

The spec author's calls: the token values and the contrast check (R1); the type scale with a 13 px body (R2); the sheet's three tokens (R3.3); the icon names (R4); the logo as the Home button (R6.2); the console following the theme (R18.3), which replaces `e2e/console.spec.ts`'s check that it matches the dark background; "Not run yet" before the first run (R14.3); `buttons.ts` kept beside a new `ui.ts` (R9.1); the delivery order.

Resolved by the builder check (2026-10-29), as the spec author's calls: solid tint tokens instead of opacity modifiers, so badges pass AA on every background (R1.4); `on-danger` for the dark Delete button, where white on `failed` is 3.0:1 (R1.3, R19.3); non-matching tree rows in `muted` instead of 40 % opacity, which dropped text to 1.9:1 (R1.6, R15.2); the build PR waiting for app-updates (Delivery).

## Left to the builder

- How `Icon` turns Lucide's icon data into Solid 2 JSX (Lucide's own `lucide-solid` targets Solid 1), as long as only used icons are bundled.
- The font packaging (Fontsource packages or files in `assets/fonts/` with the OFL), as long as ADR 0021 holds.
- How the before and after screenshots are taken (a Playwright script against the built app is expected), and the exact list of states, as long as R22's screens are all there.
- Copy of tooltips, as long as the accessible names e2e tests use stay.
- Small spacing and size adjustments that keep R2 and R3's scales.

## Delivery

1. This spec with ADRs 0021 and 0022: a draft PR into `develop`, `Part of #107`.
2. The build, on `feature/premium-ui`: tokens, fonts, icons, shared styles, the screen pass, the Appearance setting (R1 to R22). The app-updates work (branch `feature/app-updates`, not yet on `develop`) adds files with raw palette classes that R1.1 and R9.3 would break, so the build PR opens after app-updates is on `develop`: the build syncs, restyles those files (R21), adds Settings › Appearance (R8), and its PR `Closes #107` with the before and after record. Work on the branch does not wait; only the PR does. If app-updates is still not on `develop` when everything else is done, the person decides: wait, or open the PR without R8 and R21 as `Part of #107` and have app-updates restyle its own files.

## Acceptance criteria

Every scenario is automated with Playwright against the built app, each with a fresh app data folder, unless marked **(unit)** or **(by hand)**. Unit scenarios run in Vitest; by-hand scenarios run on the packaged app and go into the check record of R22.

```gherkin
Feature: Premium look and feel

  Background:
    Given Grace is built with the bundled Java and Karate on Windows 11

  # R1: tokens

  Scenario: Every token pair meets WCAG 2.2 AA in both themes (unit)
    Given the light and dark token values in src/index.css
    Then ink and muted reach 4.5:1 on ground, surface, sunken, selected and raised
    And each status colour reaches 4.5:1 on surface and on its own tint, and 3:1 on selected
    And on-accent reaches 4.5:1 on accent
    And on-primary reaches 4.5:1 on primary and on primary-hover
    And control and focus reach 3:1 on surface and ground

  Scenario: The dark theme changes colours only (unit)
    Given the dark block of src/index.css
    Then it declares no custom property except --color-* tokens, --shadow-float and --backdrop
    And primary equals passed in each theme

  Scenario Outline: The structure and the primary button do not change with the theme
    Given Windows is in <mode> mode
    When I open the http sample project
    Then the sheet is inset 8 px, has 16 px corners and a 1 px edge
    And "Run all" is filled with <fill>, the passed green, with <text> text

    Examples:
      | mode  | fill    | text    |
      | light | #1b7a4c | #ffffff |
      | dark  | #3fcf6e | #151821 |

  Scenario: No raw palette class or dark: variant is left (unit)
    When the test scans every .ts and .tsx file in src/renderer
    Then it finds no class like "zinc-500" or "red-600" and no "dark:" variant

  Scenario: Main and the startup screen use the CSS ground colours (unit)
    Then the ground values in src/shared/theme.ts equal the light and dark ground tokens in src/index.css

  Scenario Outline: The window follows the Windows theme
    Given Windows is in <mode> mode and Appearance is "Follow Windows"
    When I open the http sample project
    Then the window background is <ground>
    And the toolbar text colour is <ink>

    Examples:
      | mode  | ground  | ink     |
      | light | #f3efe6 | #151821 |
      | dark  | #151821 | #f1efe9 |

  # R2: type

  Scenario: Grace draws its own typefaces offline
    Given the network is unreachable
    When I open a project
    Then the tree text is drawn in "IBM Plex Sans"
    And the console text is drawn in "IBM Plex Mono"
    And no request leaves the app for a font or an icon

  # R4, R7: status shapes

  Scenario: Each status has its own icon and name
    When I run a project with a passing, a failing and a never-run test
    Then the passing row shows the "circle-check" mark named "Passed"
    And the failing row shows the "circle-x" mark named "Failed"
    And the never-run row shows the "circle" mark named "Not run"

  Scenario: A cancelled run marks its interrupted tests
    When I cancel a run while a test is running
    Then that test shows the "circle-slash" mark named "Interrupted"
    And the progress line shows "Cancelled" with its icon

  Scenario: Tag chip states differ by shape, not only colour (unit)
    Given the filter includes "smoke" and excludes "wip"
    Then the "smoke" chip shows the check icon and is named "@smoke, included"
    And the "wip" chip shows the minus icon, is struck through and is named "@wip, excluded"

  Scenario: An HTTP status shows its code in a badge with its group in the tooltip (unit)
    Given calls answered 200, 404, 503 and one with no response
    Then each row's badge shows "200", "404", "503" and "—"
    And their tooltips read "2xx Success", "4xx Client error", "5xx Server error" and "No response"

  # R5: motion

  Scenario: Reduced motion stops every animation
    Given the window emulates prefers-reduced-motion: reduce
    When a run is in progress
    Then the running mark, the waiting stripe and the console cursor do not move
    And no element has a running CSS transition or animation

  # R6: brand

  Scenario: The logo is the Home button
    Given a project is open
    Then the toolbar starts with the Grace logo named "Home"
    When I click it
    Then the home screen shows

  Scenario: The startup screen shows the logo in the active theme (by hand)
    Given Appearance is "Dark" and Windows is in light mode
    When I start Grace
    Then the startup screen shows the logo, "Grace" and the spinner on #151821
    And the title bar is dark from the first frame

  # R8: Appearance

  Scenario: Choosing Dark overrides a light Windows
    Given Windows is in light mode
    When I open Settings › Appearance and choose "Dark"
    Then the window background becomes #151821 without a restart
    And after a restart it is still #151821

  Scenario: Follow Windows is the default
    Given a fresh app data folder
    When I open Settings › Appearance
    Then "Follow Windows" is selected

  Scenario: A malformed appearance setting reads as Follow Windows (unit)
    Given settings.json holds "appearance": { "theme": "purple" }
    Then the saved theme reads as "system"

  Scenario: Main rejects an unknown theme (unit)
    When the renderer calls setAppearance("sepia")
    Then main refuses it and the saved theme does not change

  # R9.4: tooltips

  Scenario: A tooltip shows on keyboard focus and closes with Escape
    Given a project is open
    When I move the keyboard focus to the Refresh button
    Then a tooltip reading "Refresh" shows at once
    When I press Escape
    Then the tooltip closes and the focus stays on Refresh

  Scenario: A tooltip shows after a pause on hover, also on a disabled button
    Given a run is in progress
    When I rest the pointer on the disabled Refresh button for 500 ms
    Then a tooltip reading "Refresh" shows
    When I move the pointer onto the tooltip
    Then it stays open

  Scenario: Escape in a dialog closes the tooltip first
    Given the Properties dialog is open and a tooltip shows inside it
    When I press Escape
    Then the tooltip closes and the dialog stays open

  Scenario: No native title attribute is left (unit)
    When the test scans every .tsx file in src/renderer
    Then it finds no title attribute

  # R10: keyboard and accessibility

  Scenario: Every control shows the focus ring
    Given a project is open
    When I press Tab through the toolbar, the filter bar, the tree, the details and the bottom panel
    Then every focused element shows a 2 px outline in the focus colour

  Scenario: Targets are at least 24 by 24
    Given a project is open with a recent folder list
    Then the tree's run buttons, the tab buttons, the filter chips and the recent list's remove buttons are at least 24 px wide and high

  Scenario: Existing keyboard paths still work
    Given the e2e suite of develop
    Then every existing e2e test passes, its selectors unchanged except where this spec changes the design

  # R12, R16, R17, R18: states

  Scenario: First run shows the first-run home screen
    Given a fresh app data folder
    When I start Grace
    Then the home screen shows the mascot, "Open a Karate project" and "Pick the folder that holds your .feature files. Grace brings its own Java and Karate."
    And "Open folder…" is the only primary button

  Scenario: A missing recent folder says so in words
    Given the recent list holds a folder that no longer exists
    When the home screen shows
    Then its row shows "Not found" with the warning icon and is disabled

  Scenario: A project with no feature files shows an empty state
    When I open a folder with no .feature files
    Then the tree shows "No feature files found" and "Grace looks for .feature files under the project folder."

  Scenario: Before any run the progress line says so
    When I open a project that has never run
    Then the progress line shows "<total> tests · Not run yet"

  Scenario: Details with nothing selected
    When I open a project and select nothing
    Then the details show "Select a test to see its details"

  Scenario: Empty history and HTTP tabs
    When I open a project that has never run
    Then the History tab shows "No runs yet. Runs appear here when they finish."
    And the HTTP tab shows "No HTTP calls yet. Calls appear when a feature finishes."

  # R19: dialogs

  Scenario: Dialogs keep their order and keyboard behaviour
    When I open Clear history
    Then "Delete" comes before "Cancel" and "Cancel" has the focus
    When I press Escape
    Then the dialog closes

  # R22

  Scenario: Before and after screenshots exist (by hand)
    Then docs/checks holds a dated record with before and after screenshots of every screen of R22, in light and dark
```
