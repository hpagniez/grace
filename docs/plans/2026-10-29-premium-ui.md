# Premium look and feel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give Grace a token-based graphic chart (colours from the logo, IBM Plex, Lucide, motion, styled tooltips, an Appearance setting) and restyle every screen to it, with before and after screenshots.

**Architecture:** `src/index.css` owns every design value as Tailwind v4 `@theme` tokens whose colour values switch under `prefers-color-scheme: dark`; main sets `nativeTheme.themeSource` from the Appearance setting, so one switch moves the whole window. Shared class strings live in `buttons.ts` and a new `ui.ts`; shared components (`Icon`, `StatusMark`, `EmptyState`, `Notice`, `Tooltip`) replace the per-screen solutions. Screens are restyled one area per task, each with its unit tests updated.

**Tech Stack:** Electron 45, SolidJS 2.0.0-rc.18, Tailwind CSS 4.3.3, Vitest 5 + jsdom, Playwright 1.64.1 `_electron`, `lucide` 1.64.0, `@fontsource-variable/ibm-plex-sans` 5.3.0, `@fontsource/ibm-plex-mono` 5.3.0.

**Spec:** [docs/specs/2026-10-29-premium-ui.md](../specs/2026-10-29-premium-ui.md) (requirements R1 to R22). Read it before any task: token values, icon names, sizes and copy come from it and are not repeated here.

## Global Constraints

- Branch `feature/premium-ui` (cut from `feature/premium-ui-spec`); Conventional Commits, scope `ui` unless the change is in `main`, `ipc` or `deps`; one logical change per commit; every commit message ends with the Claude Code attribution lines.
- Solid 2 rules of `AGENTS.md` (`@solidjs/web`, `onSettled`, two-function `createEffect`, `<For keyed={false}>`, no `classList`).
- Pin dependency versions exactly (no `^`). New dependencies: only `lucide`, `@fontsource-variable/ibm-plex-sans`, `@fontsource/ibm-plex-mono` (ADR 0021, 0022).
- No network at run time: fonts and icons are bundled; CSP `default-src 'self'` unchanged.
- No `dark:` variant and no raw palette class in `src/renderer/` once Task 13 lands; components write tokens only (`bg-surface`, `text-muted`, `border-line`, `text-failed`, `bg-failed-tint` …).
- Every `data-testid`, every accessible name an e2e test uses, and every keyboard path keeps working unless the spec changes it.
- No text dimmed with `opacity` except disabled controls (R1.6). Nothing under 11 px (R2.2). Weights 400, 500, 600 only.
- Motion: `transform`, `opacity` and colours only; everything off under `prefers-reduced-motion: reduce` (R5).
- Never run `npm test` and `npm run e2e` at the same time. Run `npm run typecheck` and `npm run lint` before each commit that touches code.

## Review Focus

1. **Theme change with a project open and a dialog up:** the window, the open dialog and the title bar switch at once, with no reload and no state lost. Test in Task 15.
2. **Long paths and test names:** the toolbar path truncates from its start but keeps its characters in order (no bidi reordering of `D:\` or a trailing `\`), and the full text is in the tooltip. Test in Task 6.
3. **A tooltip near a window edge or on a target inside a scrolled list:** it flips above when there is no room below and never leaves the window. Test of `placeTip` in Task 4.
4. **Escape with nested layers:** a tooltip closes first, then a dialog, then (app-updates) the Settings page; one Escape closes one layer. Tests in Task 4 and Task 15.
5. **An HTTP status outside 100–599 (say 0 or 999):** a neutral `skipped` badge with the code and the tooltip `Status <code>`, never a crash or an empty badge. Test in Task 11.

---

### Task 1: Screenshot harness and the "before" record

**Files:**
- Create: `playwright.screens.config.ts`, `e2e/screens.shots.ts`
- Modify: `package.json` (script `screens`), `.gitignore` if needed
- Create: `docs/checks/2026-10-29-premium-ui/before/*.png`

**Interfaces:**
- Produces: `npm run screens` (packages, then runs `playwright test --config playwright.screens.config.ts`), writing `<SHOTS_DIR>/<theme>-<screen>.png` for `theme` in `light`, `dark`. `SHOTS_DIR` defaults to `docs/checks/2026-10-29-premium-ui/after`.

- [ ] **Step 1: Write `playwright.screens.config.ts`**: `testDir: "e2e"`, `testMatch: "screens.shots.ts"`, `workers: 1`, `timeout: 120_000`. The default `playwright.config.ts` must not pick the file up (its default match is `*.spec.ts`/`*.test.ts`).
- [ ] **Step 2: Write `e2e/screens.shots.ts`** using `launch(undefined, {}, { colorScheme })` and the helpers of `e2e/helpers.ts` (`project`, `openProject`, `launchWithSettings`). One `test` per theme; each saves, at a 1280 × 800 window: `home-first-run`, `home-recent` (one recent folder missing), `project-idle` (`project("http")`), `running` (screenshot while `run-outcome` reads `Running…`), `finished-failed` (a failed test selected, `project("clean")` has one), `group-selected`, `filter-on` (`project("tags")`, one tag included, one excluded), `console`, `history`, `http-call` (HTTP tab, a call selected), `properties-dialog`, `clear-history-dialog`, `notice` (open a missing recent folder). Names are the file names.
- [ ] **Step 3: Run** `SHOTS_DIR=docs/checks/2026-10-29-premium-ui/before npm run screens` (PowerShell: `$env:SHOTS_DIR=...`). Expected: 2 passed, 26 PNG files in `before/`. (The startup screen is too brief to catch; Task 16 checks it by hand.)
- [ ] **Step 4: Look at four of them** (both themes of `finished-failed` and `home-recent`) to confirm they show what their names say.
- [ ] **Step 5: Commit** `test(ui): add a screenshot harness and the before record`.

### Task 2: Tokens, fonts and base styles

**Files:**
- Modify: `src/index.css`, `package.json`, `package-lock.json`, `src/main.ts:78`, `index.html` (startup colours only), `e2e/startup.spec.ts`, `docs/decisions/0021-*.md`, `docs/decisions/0022-*.md` (status `accepted` once the packages work)
- Create: `src/shared/theme.ts`, `src/shared/theme.test.ts`, `src/renderer/tokens.test.ts`

**Interfaces:**
- Produces: `GROUND: { readonly light: "#f3efe6"; readonly dark: "#151821" }`, `type Theme = "system" | "light" | "dark"`, `THEMES: readonly Theme[]`, `parseTheme(x: unknown): Theme` (anything else → `"system"`) in `src/shared/theme.ts`.
- Produces: Tailwind utilities for every token of spec R1.3 and R1.4 (`bg-ground`, `bg-surface`, `bg-raised`, `bg-sunken`, `bg-hover`, `bg-selected`, `border-line`, `border-control`, `text-ink`, `text-muted`, `bg-accent`, `text-on-accent`, `text-on-danger`, `text-brand`, `text-passed` … `bg-passed-tint` …, `outline-focus`), the text scale `text-2xs` … `text-xl` (R2.2), `font-sans`/`font-mono`, `shadow-float`, `ease-standard`, and the CSS variables `--duration-fast` (100ms), `--duration-base` (160ms), `--sheet-inset`, `--sheet-radius`, `--sheet-side` (the sheet's side and bottom border width: 1px light, 0 dark), `--backdrop` (the dialog backdrop: `ink` at 40 % light, black at 60 % dark), used as `duration-(--duration-base)`, `mx-(--sheet-inset)` and so on. One token beyond the spec's table: `--color-play`, the Run button's play glyph, `#3fcf6e` (brand) in light and `#06210f` (on-accent) in dark (R13.2).

- [ ] **Step 1: Install** `npm install --save-exact lucide@1.64.0 @fontsource-variable/ibm-plex-sans@5.3.0 @fontsource/ibm-plex-mono@5.3.0` (runtime `dependencies`).
- [ ] **Step 2: Write the failing tests.** `src/renderer/tokens.test.ts` (node project) reads `src/index.css`, collects `--color-<name>: #rrggbb` from the `@theme` block (light) and from the `prefers-color-scheme: dark` block (dark), and asserts every pair of spec R1.5 with the WCAG formula, for example:

```ts
for (const theme of ["light", "dark"] as const) {
  const t = tokens(theme);
  for (const bg of ["ground", "surface", "sunken", "hover", "selected", "raised"])
    for (const fg of ["ink", "muted"]) expect(contrast(t[fg], t[bg]), `${theme} ${fg}/${bg}`).toBeGreaterThanOrEqual(4.5);
  for (const s of ["passed", "failed", "skipped", "running", "cancelled"]) {
    expect(contrast(t[s], t.surface)).toBeGreaterThanOrEqual(4.5);
    expect(contrast(t[s], t[`${s}-tint`])).toBeGreaterThanOrEqual(4.5);
    expect(contrast(t[s], t.selected)).toBeGreaterThanOrEqual(3);
  }
  expect(contrast(t["on-accent"], t.accent)).toBeGreaterThanOrEqual(4.5);
  expect(contrast(t["on-danger"], t.failed)).toBeGreaterThanOrEqual(4.5);
  for (const bg of ["surface", "ground"]) for (const fg of ["control", "focus"]) expect(contrast(t[fg], t[bg])).toBeGreaterThanOrEqual(3);
}
```

  `src/shared/theme.test.ts`: `parseTheme("dark") === "dark"`, `parseTheme("purple") === "system"`, `parseTheme(undefined) === "system"`; and `GROUND.light`/`GROUND.dark` equal the `ground` values read from `src/index.css` and appear in `index.html`.
- [ ] **Step 3: Run** `npx vitest run src/renderer/tokens.test.ts src/shared/theme.test.ts`. Expected: FAIL (no tokens, no module).
- [ ] **Step 4: Write `src/index.css`.** Keep `@import "tailwindcss"`, the `blink` and `slide` keyframes and the cursor rules. Add, in `@theme`: the light colour tokens of R1.3 and R1.4 (keep Tailwind's default palette for now: Task 13 removes it), `--font-sans: "IBM Plex Sans Variable", "Segoe UI Variable", system-ui, sans-serif`, `--font-mono: "IBM Plex Mono", "Cascadia Mono", Consolas, monospace`, the six `--text-*` sizes with their `--text-*--line-height` (R2.2), `--shadow-float` (light value, R3.5), `--ease-standard`. In `@layer base`: the dark colour values and dark `--shadow-float` on `:root` under `@media (prefers-color-scheme: dark)`; `--sheet-inset: 8px; --sheet-radius: 16px; --sheet-side: 1px` (light) and `0px` (dark); the duration and backdrop variables; `html { background: var(--color-ground); color: var(--color-ink); font-family: var(--font-sans); font-size: 13px }`; `:focus-visible { outline: 2px solid var(--color-focus); outline-offset: 2px }`; scrollbar colours from `--color-control` (R1.7); the reduced-motion rule (R5.3: `*, ::before, ::after { animation-duration: 0.01ms !important; animation-iteration-count: 1 !important; transition-duration: 0.01ms !important }`). `@font-face` rules with `font-display: block` pointing at `../node_modules/@fontsource-variable/ibm-plex-sans/files/ibm-plex-sans-latin-wght-normal.woff2` and `…-latin-ext-wght-normal.woff2` (family `IBM Plex Sans Variable`, `font-weight: 100 700`, the `unicode-range`s copied from that package's `index.css`), and `@fontsource/ibm-plex-mono/files/ibm-plex-mono-latin{,-ext}-{400,500}-normal.woff2` (family `IBM Plex Mono`).
- [ ] **Step 5: Write `src/shared/theme.ts`** (Interfaces above); set `backgroundColor: GROUND[nativeTheme.shouldUseDarkColors ? "dark" : "light"]` in `src/main.ts`; set `#startup`'s background and colour in `index.html` to the `ground` and `ink` values of both themes.
- [ ] **Step 6: Update `e2e/startup.spec.ts`**: expect `background` to be `GROUND.dark` or `GROUND.light`.
- [ ] **Step 7: Run** `npx vitest run src/renderer/tokens.test.ts src/shared/theme.test.ts`, `npm run typecheck`, `npm run lint`. Expected: PASS. Then `npm start` once and check in DevTools that `document.fonts.check("13px 'IBM Plex Sans Variable'")` is true and no request leaves for a font.
- [ ] **Step 8: Mark ADRs 0021 and 0022 `accepted`** (a separate commit: `docs: accept adr 0021 and 0022`).
- [ ] **Step 9: Commit** `feat(ui): add the colour, type and motion tokens and bundle IBM Plex` (with `package.json` and the lock file).

### Task 3: Icon, StatusMark, EmptyState, Notice and shared class strings

**Files:**
- Create: `src/renderer/Icon.tsx`, `src/renderer/StatusMark.tsx`, `src/renderer/EmptyState.tsx`, `src/renderer/Notice.tsx`, `src/renderer/ui.ts`, and a `.test.tsx` for each component
- Modify: `src/renderer/buttons.ts`, `src/renderer/nodes.ts`

**Interfaces:**
- Produces: `Icon(props: { icon: IconNode; size?: 12 | 14 | 16 | 24; class?: string }): JSX.Element` (`IconNode` from `lucide`): an `<svg>` with Lucide's default attributes (`viewBox="0 0 24 24"`, `fill="none"`, `stroke="currentColor"`, `stroke-width="2"`, round caps and joins), `width`/`height` = `size` (default 16), `aria-hidden="true"`, its children built from the `IconNode` as `innerHTML` (the data is static and trusted).
- Produces: `type MarkStatus = Status | "skipped" | "cancelled"`; `MARK: Record<MarkStatus, { icon: IconNode; text: string; label: string }>` with the icons of R4.4 (`notRun` `Circle`, `pending` `CircleDashed`, `running` `LoaderCircle`, `passed` `CircleCheck`, `failed` `CircleX`, `interrupted` and `cancelled` `CircleSlash`, `skipped` `CircleMinus`), `text` the colour class (`text-skipped`, `text-running`, `text-passed`, `text-failed`, `text-cancelled`) and `label` from `STATUS_LABEL` plus `"Skipped"` and `"Cancelled"`; `StatusMark(props: { status: MarkStatus; size?: 14 | 16 }): JSX.Element` → `<span role="img" aria-label={label} data-status={status}>` with the icon, which turns (`animate-spin`) for `running`.
- Produces: `EmptyState(props: { icon: IconNode; text: string; testid?: string; children?: JSX.Element })`, a centred column: 24 px `muted` icon, the sentence in `muted` `text-sm` with `data-testid={testid}`, then `children` (an action).
- Produces: `Notice(props: { kind: "warning" | "error"; text: string; testid?: string })`, R20.1: `bg-cancelled-tint text-cancelled` + `TriangleAlert` with `role="status"`, or `bg-failed-tint text-failed` + `CircleAlert` with `role="alert"`; the text in `text-ink`.
- Produces in `buttons.ts` (same names, token values): `btn`, `btnSmall`, `primary`, `danger`, plus `btnIcon` (28 × 28, icon only) and `ghost` (no border, `hover:bg-hover`). Heights per R3.1 (28 px, small 24 px), `rounded-md`, `gap-1.5`, `inline-flex items-center`, disabled at `opacity-40`. `primary` = `bg-accent text-on-accent font-semibold`; `danger` = `bg-failed text-on-danger`.
- Produces in `ui.ts`: `input` (28 px, `border-control bg-surface rounded-md px-2`), `chip`, `chipIncluded`, `chipExcluded` (R7.3), `tab`, `tabOn`, `badge` (`inline-flex h-5 items-center gap-1 rounded-full px-2 text-2xs font-medium`), `TINT: Record<"passed" | "failed" | "skipped" | "running" | "cancelled", string>` (`"bg-passed-tint text-passed"` …), `box` (`rounded-lg bg-sunken p-2 font-mono text-xs`), `dialog` (R19.1, with the R5.2 enter: `transition-discrete` on opacity, scale and `display`/`overlay` over `--duration-base` with `starting:opacity-0 starting:scale-[0.98]`, and `backdrop:bg-(--backdrop)`), `countPill`.
- Modify `nodes.ts`: drop `STATUS_DOT` and `FLAKY_TONE` only after their last user is restyled (Tasks 7 and 8); `STATUS_LABEL` stays.

- [ ] **Step 1: Write the failing tests.** `Icon.test.tsx`: renders an `svg` with `aria-hidden="true"`, `width="14"` for `size={14}`, one child `path` per `Play` node. `StatusMark.test.tsx`: for every key of `MARK` (all of `Status` plus `skipped`, `cancelled`), the mark has `role="img"`, the expected `aria-label` (`"Passed"`, `"Not run"`, `"Interrupted"` …) and a distinct icon from its neighbours except `interrupted`/`cancelled`; `running` has `animate-spin`. `EmptyState.test.tsx` and `Notice.test.tsx`: text, `data-testid`, role per kind.
- [ ] **Step 2: Run** `npx vitest run src/renderer/Icon.test.tsx src/renderer/StatusMark.test.tsx src/renderer/EmptyState.test.tsx src/renderer/Notice.test.tsx`. Expected: FAIL.
- [ ] **Step 3: Implement** the components and class strings above.
- [ ] **Step 4: Run** the same tests, then `npm test`. Expected: PASS (the screens still use the old classes from `buttons.ts` names, now token-valued).
- [ ] **Step 5: Commit** `feat(ui): add the icon, status mark, empty state and notice components`.

### Task 4: Styled tooltips

**Files:**
- Create: `src/renderer/Tooltip.tsx`, `src/renderer/Tooltip.test.tsx`, `src/renderer/placeTip.ts`, `src/renderer/placeTip.test.ts`

**Interfaces:**
- Produces: `placeTip(target: { top: number; left: number; bottom: number; width: number }, tip: { width: number; height: number }, view: { width: number; height: number }): { top: number; left: number; above: boolean }`: 6 px below the target and centred on it; above when `bottom + 6 + height > view.height - 8`; `left` clamped to `[8, view.width - 8 - tip.width]`.
- Produces: `Tooltip(props: { text: string; children: JSX.Element; describe?: boolean }): JSX.Element`. It wraps `children` in a `<span class="inline-flex">` that listens for `pointerenter`/`pointerleave` (500 ms delay to show) and `focusin`/`focusout` (show at once when the focused element matches `:focus-visible`), and renders a `<div popover="manual" role="tooltip" id=…>` styled per R9.4. Escape (a `keydown` listener on `document`, capture phase, added while open) hides it and calls `stopPropagation()` and `preventDefault()`. Scroll (capture) hides it. A module-level slot keeps one tooltip open at a time. With `describe`, the focusable child gets `aria-describedby` pointing at the tooltip. Guards `showPopover`/`hidePopover` with optional calls (jsdom may lack them).

- [ ] **Step 1: Write the failing tests.** `placeTip.test.ts`: below and centred in open space; above when it would cross `view.height - 8`; `left` clamped to 8 at the left edge and to `view.width - 8 - tip.width` at the right. `Tooltip.test.tsx` with fake timers: hover shows after 500 ms and not at 499; `focusin` from keyboard shows at once; a second tooltip closes the first; moving the pointer from the target onto the tooltip keeps it open; Escape inside an open `<dialog>` hides the tooltip and the dialog's `cancel` does not fire; a disabled `<button>` inside the wrapper still shows on `pointerenter` of the wrapper; `describe` sets `aria-describedby`.
- [ ] **Step 2: Run** `npx vitest run src/renderer/placeTip.test.ts src/renderer/Tooltip.test.tsx`. Expected: FAIL.
- [ ] **Step 3: Implement** `placeTip` and `Tooltip`.
- [ ] **Step 4: Run** the same tests. Expected: PASS.
- [ ] **Step 5: Commit** `feat(ui): add styled tooltips`.

### Task 5: The frame, the startup screen and notices

**Files:**
- Modify: `src/renderer/App.tsx`, `index.html`, `e2e/startup.spec.ts` (if it reads the startup markup)

**Interfaces:**
- Consumes: `Notice` (Task 3), tokens (Task 2).

- [ ] **Step 1: Update tests**: `e2e/startup.spec.ts` "static startup screen" also expects the logo (`<svg` or `<img` inside `#startup`) and `Grace`. Add to an App unit test (create `src/renderer/App.test.tsx` only if no test renders `App` today) that the notice renders as `Notice` with `data-testid="notice"` and `role="status"`.
- [ ] **Step 2: Implement.** App root: `bg-ground text-ink`, no `dark:`. Under the toolbar, wrap `Progress`, `FilterBar`, the notice, `main` and `BottomPanel` in the sheet: `mx-(--sheet-inset) mb-(--sheet-inset) rounded-(--sheet-radius) border-t border-x-(length:--sheet-side) border-b-(length:--sheet-side) border-line bg-surface overflow-hidden flex flex-col min-h-0 flex-1` (in dark the inset, radius and side borders are 0, so the panels fill the window under one top line). The home screen has no sheet (R12.1). `#startup`: the logo from `assets/icon.svg` inlined as SVG at 48 px, `Grace` in 14 px, the spinner; colours from Task 2.
- [ ] **Step 3: Run** `npm test`, `npm run typecheck`, `npm run lint`. Expected: PASS.
- [ ] **Step 4: Commit** `feat(ui): put the project view on a sheet and restyle the startup screen`.

### Task 6: Toolbar and progress

**Files:**
- Modify: `src/renderer/Toolbar.tsx`, `src/renderer/Progress.tsx`, `src/renderer/EnvPicker.tsx`, `src/renderer/Threads.tsx`, `src/renderer/PropertiesDialog.tsx` (its toolbar button only), `src/renderer/Tree.test.tsx` (progress assertions live there), and the unit tests that cover the toolbar

**Interfaces:**
- Consumes: `buttons.ts`, `ui.ts`, `Icon`, `StatusMark`, `Tooltip`.

- [ ] **Step 1: Write the failing tests.** The toolbar order of R13.1 by `data-testid` (`home-button`, `project-path`, `project-layout`, `env-input`, `threads-input`, properties button, `refresh-button`, `open-button`, `report-button`, `rerun-failed-button`, `cancel-button`, `run-all-button`); `home-button` holds the logo and is named "Home"; `refresh-button` is named "Refresh"; the path element has an inner `<span dir="ltr">` with the full path (Review Focus 2) and a tooltip with it; Progress before any run shows `24 tests · Not run yet` (R14.3; `run.total` 24, no outcome, never ran) and after a run the `StatusMark`s named "Passed" and "Failed" with their counts; `progress-stripe` keeps `animate-slide` and `motion-reduce:animate-none`; `run-outcome` shows `Running…` with the `running` mark, `Finished` with `CircleCheck`, `Cancelled` with `CircleSlash`, `Run failed` with `CircleAlert` (R14.2).
- [ ] **Step 2: Run** the toolbar and progress tests. Expected: FAIL.
- [ ] **Step 3: Implement** R13 and R14: 44 px row, `bg-ground`, dividers `h-5 w-px bg-line`, `Run all` as `primary` with `Play` in `text-play`, the Re-run count in `countPill`, the env and threads boxes in `input`, the project path truncated from the start (`direction: rtl` on the outer span, `<span dir="ltr">` inside). Progress bar segments `bg-passed`, `bg-failed`, `bg-cancelled` with `gap-0.5` (R14.1); the line of R14.2 with `font-mono tabular-nums` figures.
- [ ] **Step 4: Run** `npm test`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(ui): restyle the toolbar and progress`.

### Task 7: Filter bar and tree

**Files:**
- Modify: `src/renderer/FilterBar.tsx`, `src/renderer/Tree.tsx`, `src/renderer/FilterBar.test.tsx`, `src/renderer/Tree.test.tsx`, `e2e/tags.spec.ts`

**Interfaces:**
- Consumes: `chip`, `chipIncluded`, `chipExcluded`, `StatusMark`, `EmptyState`, `Notice`, `Tooltip`, `Icon`.

- [ ] **Step 1: Write the failing tests.** FilterBar: the included chip has the `Check` icon and `aria-label "@smoke, included"`, the excluded chip the `Minus` icon, `line-through`, `aria-label "@wip, excluded"` (replace the `border-green-600`/`border-red-600` assertions). Tree: rows carry `StatusMark`s with the labels of R4.4; the chevron is a `ChevronRight` icon with `rotate-90` when open; a non-matching row has `data-match="false"`, `text-muted` and no `opacity-40` (replace the `opacity-40` assertions here and in `e2e/tags.spec.ts` with `data-match`); the run button is ≥ 24 × 24 (`size-6`), named `Run <name>`; the flaky mark keeps `FLAKY_TITLE` as its accessible name and shows it as a tooltip (replace `mark.title`); "No feature files found" is an `EmptyState` with `data-testid="tree-message"` and the R16.4 sentence; "Finding tests…" keeps `tree-message`; a dry-run error is a `Notice` of kind `error`.
- [ ] **Step 2: Run** `npx vitest run src/renderer/FilterBar.test.tsx src/renderer/Tree.test.tsx`. Expected: FAIL.
- [ ] **Step 3: Implement** R15 and R16 (rows `h-6 rounded-sm`, `hover:bg-hover`, selected `bg-selected shadow-[inset_2px_0_0_var(--color-accent)]`, focus ring inset `focus-visible:-outline-offset-2`, folder/feature icons, tags in `chip`, flaky chip `TINT.cancelled` + `Shuffle`). Remove `STATUS_DOT` and `FLAKY_TONE` from `nodes.ts` if Task 8 already dropped their other users; otherwise leave the removal to Task 8.
- [ ] **Step 4: Run** `npm test`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(ui): restyle the filter bar and the tree`.

### Task 8: Details and flaky proof

**Files:**
- Modify: `src/renderer/Details.tsx`, `src/renderer/FlakyProof.tsx`, `src/renderer/nodes.ts`, `src/renderer/FlakyProof.test.tsx`, any Details unit test

**Interfaces:**
- Consumes: `StatusMark`, `TINT`, `badge`, `box`, `EmptyState`, `Icon`, `ghost`.

- [ ] **Step 1: Write the failing tests.** Leaf: a badge with the status word and its mark (`details-status` keeps the word); the error box titled "Error" with `CircleAlert` (`details-error` keeps the text); each `step` row has a `StatusMark` named by its status (`passed`, `failed`, `skipped` → "Passed", "Failed", "Skipped"); the log toggle stays `step-toggle` with `aria-expanded`. Group: `details-counts` shows one `StatusMark` + figure + word per status present. Nothing selected: `EmptyState` with "Select a test to see its details". FlakyProof: each `flaky-run` row has its `StatusMark`.
- [ ] **Step 2: Run** those tests. Expected: FAIL.
- [ ] **Step 3: Implement** R17; drop `STATUS_DOT`, `STEP_DOT` and `FLAKY_TONE`.
- [ ] **Step 4: Run** `npm test`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(ui): restyle the details pane and the flaky proof`.

### Task 9: Bottom panel, splitter, console and banner

**Files:**
- Modify: `src/renderer/BottomPanel.tsx`, `src/renderer/Splitter.tsx`, `src/renderer/Console.tsx`, `src/renderer/Banner.tsx`, `src/renderer/Banner.test.tsx`, `src/renderer/Console.test.tsx`, `e2e/console.spec.ts`, `e2e/scrollbar.spec.ts`

**Interfaces:**
- Consumes: `tab`, `tabOn`, `countPill`, `Icon` (`SquareTerminal`, `History` (exported by `lucide` as an alias of `RotateCcwClock`), `Globe`).

- [ ] **Step 1: Write the failing tests.** `Banner.test.tsx`: "Grace" has `font-semibold` and `text-accent` (replace `text-sky-400`), "Ready." `text-passed` (replace `text-emerald-400`), the cursor keeps `animate-blink` and `motion-reduce:animate-none`. `Console.test.tsx`: ` ERROR ` lines `text-failed`, ` WARN ` lines `text-cancelled`. `e2e/console.spec.ts`: replace "darker than the window in dark mode only" with "the console is on the sunken colour in both themes" (computed background equals `#f8f6f1` light, `#0b0d11` dark). `e2e/scrollbar.spec.ts`: thumb colour from `control` per theme.
- [ ] **Step 2: Run** `npx vitest run src/renderer/Banner.test.tsx src/renderer/Console.test.tsx`. Expected: FAIL.
- [ ] **Step 3: Implement** R18.1–R18.3.
- [ ] **Step 4: Run** `npm test`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(ui): restyle the bottom panel and the console`.

### Task 10: History tab

**Files:**
- Modify: `src/renderer/History.tsx`, `src/renderer/History.test.tsx`

- [ ] **Step 1: Write the failing tests.** `history-result` holds a `StatusMark` (failed when `run.outcome === "failed" || run.failed > 0`, cancelled for a cancelled run, passed otherwise) and the words of `resultLabel`; `history-threads` and `history-duration` are right-aligned with `tabular-nums`; tag cells use `chipIncluded`/`chipExcluded` with `@tag` / `not @tag`; `history-what` shows its path in a tooltip (replace the `.title` assertions with the tooltip's text); `history-empty` is an `EmptyState` with "No runs yet. Runs appear here when they finish."; row actions are `btnSmall` ghost buttons with icons, keeping their test ids.
- [ ] **Step 2: Run** `npx vitest run src/renderer/History.test.tsx`. Expected: FAIL.
- [ ] **Step 3: Implement** R18.4 (sticky `bg-surface` headers in `text-xs text-muted font-medium`, `h-7` rows with `border-b border-line`, `scroll-mt-7` on rows for R10.2), and the Clear history dialog per R19.1 and R19.3.
- [ ] **Step 4: Run** `npm test`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(ui): restyle the history tab`.

### Task 11: HTTP tab and call details

**Files:**
- Modify: `src/renderer/HttpCalls.tsx`, `src/renderer/CallDetails.tsx`, `src/renderer/HttpCalls.test.tsx`, `src/renderer/CallDetails.test.tsx`

**Interfaces:**
- Produces: `GROUP_TINT: Record<HttpGroup, keyof typeof TINT>` = `1xx` skipped, `2xx` passed, `3xx` running, `4xx` cancelled, `5xx` failed, `none` failed; a status outside 100–599 uses `skipped`.

- [ ] **Step 1: Write the failing tests.** Badge per row: text `200`, `404`, `503`, `—` with `TINT` classes of `GROUP_TINT` (replace the `bg-green-700`… assertions) and tooltips `2xx Success`, `4xx Client error`, `5xx Server error`, `No response`; a status of `999` shows `999`, the `skipped` tint and the tooltip `Status 999` (Review Focus 5); a pressed group chip has `Check` and `bg-selected`; `http-call-url` shows the URL in a tooltip (replace `.title`); `http-empty` is an `EmptyState`; `copy-curl` shows `Copy` then `Check` + "Copied".
- [ ] **Step 2: Run** `npx vitest run src/renderer/HttpCalls.test.tsx src/renderer/CallDetails.test.tsx`. Expected: FAIL.
- [ ] **Step 3: Implement** R18.5, R18.6, R7.2.
- [ ] **Step 4: Run** `npm test`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(ui): restyle the http tab and call details`.

### Task 12: Home screen and the Properties dialog

**Files:**
- Modify: `src/renderer/Home.tsx`, `src/renderer/Home.test.tsx`, `src/renderer/PropertiesDialog.tsx`, `src/renderer/PropertiesDialog.test.tsx`, `src/renderer/EnvPicker.tsx` (inline messages), `src/renderer/EnvPicker.test.tsx`

- [ ] **Step 1: Write the failing tests.** Home first run: the sentence "Pick the folder that holds your .feature files. Grace brings its own Java and Karate." and `open-button` with the `FolderOpen` icon; a missing row shows `recent-missing` reading "Not found" with `TriangleAlert`, the row disabled, a tooltip "Folder not found"; `recent-remove` is ≥ 24 × 24 and named "Remove from list" with its tooltip (replace `getAttribute("title")`). Properties: inputs use `input`; errors show `CircleAlert` and stay linked by `aria-describedby`; the secrets note shows `Lock`; `Save`/`Cancel` order unchanged. EnvPicker hint and error show their icons.
- [ ] **Step 2: Run** the four test files. Expected: FAIL.
- [ ] **Step 3: Implement** R12, R19.1, R19.2, R20.2.
- [ ] **Step 4: Run** `npm test`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(ui): restyle the home screen and the properties dialog`.

### Task 13: Remove the default palette and guard against it

**Files:**
- Modify: `src/index.css`, any renderer file the scan finds
- Create: `src/renderer/palette.test.ts`

- [ ] **Step 1: Write the failing test.** `palette.test.ts` (node project) reads every `.ts`/`.tsx` under `src/renderer/` (tests included) and fails on `/\b(slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose)-\d{2,3}\b/`, on `/\bdark:/`, and on `/\btitle=/` in `.tsx` files (R9.3), naming the file and line.
- [ ] **Step 2: Run** `npx vitest run src/renderer/palette.test.ts`. Expected: FAIL only if a file was missed; fix each one it names.
- [ ] **Step 3: Add `--color-*: initial;`** at the top of `@theme`, keeping `--color-white`, `--color-black`, `--color-transparent`, `--color-current` defined.
- [ ] **Step 4: Write `e2e/look.spec.ts`** for the spec's Playwright scenarios no unit test covers: the window and toolbar colours per emulated scheme; fonts drawn offline (`document.fonts.check("13px 'IBM Plex Sans Variable'")` and `"12px 'IBM Plex Mono'"` true, no request whose URL is not `file:` or `devtools:`); after a run of `project("clean")`, marks named "Passed", "Failed" and "Not run"; a cancelled run marks "Interrupted" and shows `Cancelled`; the logo button named "Home" goes home; with `page.emulateMedia({ reducedMotion: "reduce" })` during a run, `document.getAnimations().filter((a) => a.playState === "running")` is empty; Tab through toolbar, filter bar, tree, details and bottom panel: each focused element's computed `outline-width` is `2px` and its `outline-color` the focus colour; the run buttons, tab buttons, filter chips and recent-list remove buttons measure at least 24 × 24; the first-run text; a missing recent folder's "Not found"; an empty folder's empty state; "Not run yet"; "Select a test to see its details"; the empty History and HTTP texts; Clear history's button order and Escape.
- [ ] **Step 5: Run** `npm test`, `npm run typecheck`, `npm run lint`, then `npm run e2e`. Expected: all pass; fix every e2e test the design changed (keep its intent).
- [ ] **Step 6: Commit** `refactor(ui)!: remove tailwind's default palette` with a `BREAKING CHANGE:` footer saying raw palette classes no longer compile.

### Task 14: Sync with develop and restyle the app-updates screens

Starts only once `feature/app-updates` is merged into `develop` (check `git log origin/develop` for `Settings.tsx`). Until then, skip to Task 16's checks on what exists, and tell the person.

**Files:**
- Modify: `src/renderer/Settings.tsx`, `src/renderer/StatusBar.tsx`, `src/renderer/SwitchDialog.tsx`, `src/renderer/UpdateModal.tsx`, `src/renderer/UpdateProgress.tsx`, `src/renderer/modal.ts`, their tests, `e2e/screens.shots.ts`

- [ ] **Step 1: Sync** with the `sync_with_base_branch` tool (this is an app-made worktree), resolve conflicts, run `npm test`.
- [ ] **Step 2: Capture their "before"**: add `settings`, `status-bar` (part of `project-idle`) and `update-dialog` shots to `e2e/screens.shots.ts`, and run it into `before/` against the synced, not yet restyled build. Commit `test(ui): add the settings screens to the before record`.
- [ ] **Step 3: Run** `npx vitest run src/renderer/palette.test.ts`. Expected: FAIL, naming the app-updates files.
- [ ] **Step 4: Restyle** them per R21 (status bar 24 px `bg-ground text-xs text-muted`, the update marker `TINT.running`; the Settings nav rows like R18.5 rows; dialogs per R19.1; `title=` to `Tooltip`), updating their tests' class and title assertions.
- [ ] **Step 5: Run** `npm test`. Expected: PASS.
- [ ] **Step 6: Commit** `feat(ui): restyle the status bar, settings and update dialogs`.

### Task 15: Settings › Appearance

**Files:**
- Modify: `src/main/settings.ts`, `src/main/settings.test.ts`, `src/shared/ipc.ts`, `src/preload.ts`, `src/main.ts`, `src/renderer/Settings.tsx`, `src/renderer/Settings.test.tsx`
- Create: `e2e/appearance.spec.ts`

**Interfaces:**
- Produces: `loadAppearance(dataDir: string): Theme` (via `parseTheme(settings.appearance?.theme)`, never throws) and `saveAppearance(dataDir: string, theme: Theme): void` (through `update()`, writes `appearance: { theme }`) in `src/main/settings.ts`.
- Produces: `CH.getAppearance = "appearance:get"`, `CH.setAppearance = "appearance:set"`; `Api.getAppearance(): Promise<Theme>`, `Api.setAppearance(theme: Theme): Promise<void>`.
- Main: `nativeTheme.themeSource = loadAppearance(dataDir)` before `createWindow`; the `setAppearance` handler refuses anything not in `THEMES` with the notice "Invalid request" (the `handleRunId` pattern), else saves and sets `themeSource`; `nativeTheme.on("updated", …)` calls `win.setBackgroundColor(GROUND[nativeTheme.shouldUseDarkColors ? "dark" : "light"])`.
- Renderer: a second Settings section, "Appearance", with a `Theme` radio group (`Follow Windows` value `system`, `Light`, `Dark`), loaded with `getAppearance()` on mount, saved on change.

- [ ] **Step 1: Write the failing tests.** `settings.test.ts`: a missing file and `{ "appearance": { "theme": "purple" } }` load `"system"`; `saveAppearance(dir, "dark")` keeps other keys and loads back `"dark"`. `Settings.test.tsx`: the Appearance section lists the three radios, `Follow Windows` checked by default, choosing `Dark` calls `api.setAppearance("dark")`. `e2e/appearance.spec.ts` (spec R8 scenarios): a fresh app shows `Follow Windows` checked; with `launchWithSettings({ appearance: { theme: "dark" } })` and no emulated scheme, `nativeTheme.themeSource` is `"dark"`, the window background is `#151821` and `app`'s computed background is `rgb(21, 24, 33)`; choosing `Light` in Settings with the Properties dialog open (Review Focus 1: open it first, go to Settings, change, go back) turns the background to `#f3efe6` without a reload and the dialog's state stays; one Escape closes a shown tooltip, the next goes back from Settings (Review Focus 4); `api.setAppearance("sepia")` from the page leaves `settings.json` unchanged.
- [ ] **Step 2: Run** the unit tests. Expected: FAIL.
- [ ] **Step 3: Implement** the Interfaces above.
- [ ] **Step 4: Run** `npm test`, `npm run typecheck`, `npm run lint`, then `npx playwright test e2e/appearance.spec.ts` after `npm run package`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(ui): add the appearance setting`.

### Task 16: After record, full checks and the PR

**Files:**
- Create: `docs/checks/2026-10-29-premium-ui/after/*.png`, `docs/checks/2026-10-29-premium-ui.md`

- [ ] **Step 1: Run** `npm run screens` (writes `after/`). Look at every after shot in both themes against the spec and the mockup; fix what is off, in the task's area, and re-run.
- [ ] **Step 2: Run, one at a time:** `npm test`, `npm run typecheck`, `npm run lint`, `npm run e2e`. Expected: all green. Keep the e2e summary line.
- [ ] **Step 3: By hand on the packaged app** (`npm run make`, unzip, run): the startup screen in Dark with Windows light (R6.3), the title bar from the first frame, Windows' "Show animations" off stops every motion, Tab through every screen shows the ring. Write the record `docs/checks/2026-10-29-premium-ui.md` in the house format (date, machine, what ran, result, before/after table linking each pair of PNGs, what still needs a person).
- [ ] **Step 4: Commit** `docs(ui): add the premium ui check record`.
- [ ] **Step 5: Push, open the PR** into `develop` (`Closes #107`, the e2e summary pasted, draft until checks pass), move the issue's card to `building` with the commands in `AGENTS.md`, bind the PR with the `ccd_pr` tools, then mark it ready for review. Do not add `merge-when-green`.
