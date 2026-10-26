# Window Bounds Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore the window's size, position, screen and maximized state at start, pulled onto a connected screen, and make every `settings.json` save atomic.

**Architecture:** `src/main/settings.ts` gains an atomic `update()` (temp file, fsync, rename) and `loadWindowState` / `saveWindowState` under the `window` key. A pure `placeWindow` in `src/main/window.ts` maps the saved bounds onto the screens' work areas. `createWindow` in `src/main.ts` reads the state, creates the hidden window at the placed bounds, maximizes it in `reveal()` when needed, and saves on `close`, `moved`, `resized`, `maximize` and `unmaximize`.

**Tech Stack:** Electron 45, TypeScript, Vitest 5, Playwright 1.64.

**Spec:** [docs/specs/2026-10-26-window-bounds.md](../specs/2026-10-26-window-bounds.md) (R1–R3). Builds on the UI polish spec's F39 window start-up (`show: false`, `reveal()`, `minHeight: 480`).

## Global Constraints

- Branch `feature/window-bounds`; the spec commit `1745551` is on it. Conventional Commits, lower-case subjects, AGENTS.md rules.
- No new dependency. Nothing changes in the renderer, the preload or `webPreferences`.
- Values: settings key `window`, shape `{ x, y, width, height, maximized }`; temp file `settings.json.tmp`; first-start size `width: 1200, height: 800` with no `x`/`y`; `minHeight: 480` stays.
- `src/main/window.ts` has no runtime Electron import (`import type` is fine).
- e2e rectangles are placed relative to `screen.getPrimaryDisplay().workArea` and fit a 1024×768 screen less its taskbar. Compare bounds to within 1 px.
- Run one e2e file with `npm run package && npx playwright test e2e/<file>.spec.ts`. Never run `npm test` and `npm run e2e` at the same time.

## Review Focus

1. **`settings.json` held by another program during a save** (`EPERM`/`EBUSY` on rename, antivirus): the save throws, `settings.json` stays as it was, the window handler logs and goes on. Test: Task 1 `a failed rename leaves the file as it was`; Task 3 `a failed save does not stop the app from closing`.
2. **A second screen left of or above the main one** (negative `x`/`y`): bounds there are valid and kept. Test: Task 2 `keeps a window on a screen left of the main one`.
3. **Hand-edited numbers** (`1e999` parses as `Infinity`, `700.5`, `0`): `Infinity` and `0` read as nothing saved; fractions come out whole. Tests: Task 1 `loadWindowState` rows; Task 2 `rounds fractional bounds`.
4. **A window straddling two screens equally**: the first work area in the list wins, so the result is stable. Test: Task 2 `equal overlap picks the first work area`.
5. **A restart on CI's small screen**: the first-start window is taller than the work area, so the restart pulls it in and the window gets shorter. Specs that measure layout across a restart must not depend on the first start's height. Test: Task 3 Step 5 adjusts `panel.spec.ts`.

---

### Task 1: Atomic settings and the saved window state

**Files:**
- Modify: `src/main/settings.ts` (`update()`, new window-state helpers)
- Test: `src/main/settings.test.ts`

**Interfaces:**
- Produces:
  - `interface WindowState { x: number; y: number; width: number; height: number; maximized: boolean }`
  - `loadWindowState(dataDir: string): WindowState | null`: never throws; null for missing, unreadable or invalid (R2.5: `x`, `y`, `width`, `height` finite numbers, `width` and `height` above 0, `maximized` boolean).
  - `saveWindowState(dataDir: string, s: WindowState): void`: through `update()`, writes only `x, y, width, height, maximized`; throws on failure.
  - `update()` writes `settings.json.tmp` with `{ flush: true }`, then `renameSync` over `settings.json`.

- [ ] **Step 1: Write the failing tests** in `settings.test.ts`, extending the existing `node:fs` mock with a `renameFails` code like `readFails`:
  - `save then load returns the window state`: round trip of `{ x: -1900, y: 20, width: 700, height: 500, maximized: true }`.
  - `loadWindowState reads a broken state as nothing saved`: `null` for no file, `"abc"`, `{ ...ok, width: -5 }`, `{ ...ok, width: 0 }`, `{ ...ok, x: "a" }`, `{ ...ok, maximized: undefined }` (key missing), and the raw text `{"window":{"x":1e999,"y":0,"width":700,"height":500,"maximized":false}}`.
  - `saving the window keeps the other settings`: after `saveLastProject` and `saveProjectFilter`, `saveWindowState` leaves both loadable.
  - `a failed rename leaves the file as it was`: with `renameFails.code = "EPERM"`, `saveWindowState` throws and `settings.json`'s text is byte-equal to before. After clearing the flag, the next save succeeds and `loadLastProject` still returns the project.
  - `a leftover temp file is ignored`: write junk to `settings.json.tmp`; `loadLastProject` still works and the next save succeeds.
- [ ] **Step 2: Run** `npx vitest run src/main/settings.test.ts`. Expected: FAIL (`loadWindowState` is not exported).
- [ ] **Step 3: Implement** the Interfaces above in `settings.ts`. Parse with the existing `isObject`.
- [ ] **Step 4: Run** `npx vitest run src/main/settings.test.ts`, then `npm run typecheck`. Expected: PASS, the existing settings tests included.
- [ ] **Step 5: Commit** `fix(settings): save settings.json atomically and keep the window state` (one commit, or two if the atomic write lands first: `fix(settings): save settings.json atomically`, then `feat(settings): load and save the window state`).

### Task 2: Placement on the screens

**Files:**
- Create: `src/main/window.ts`, `src/main/window.test.ts`

**Interfaces:**
- Consumes: nothing.
- Produces: `placeWindow(saved: Rectangle, workAreas: Rectangle[]): Rectangle | null`, with `Rectangle` from `import type { Rectangle } from "electron"`. It returns `null` for first-start bounds (no work area overlaps the saved bounds by a positive area). Otherwise it picks the work area with the largest overlap (the first one on a tie), shrinks width and height to that work area's, moves the rectangle inside it, and rounds every field.

- [ ] **Step 1: Write the failing tests** with `main = { x: 0, y: 0, width: 1920, height: 1040 }` and `second = { x: 1920, y: 0, width: 2560, height: 1400 }`, `areas = [main, second]`:
  - One `it.each` for the spec's unit table (saved → placed):
    - `2000,100 1200×800` → same
    - `3900,100 1200×800` → `3280,100 1200×800`
    - `1800,100 1200×800` → `1920,100 1200×800`
    - `2000,900 1200×800` → `2000,600 1200×800`
    - `2000,100 3000×1600` → `1920,0 2560×1400`
    - `5000,100 1200×800` → `null`
    - `100,-900 1200×800` → `null`
  - `keeps a window on a screen left of the main one`: areas `[main, { x: -1920, y: 0, width: 1920, height: 1040 }]`, saved `-1800,50 700×500` → same.
  - `equal overlap picks the first work area`: saved `1620,100 600×400` (300 px on each screen) → `1320,100 600×400` (pulled into `main`).
  - `rounds fractional bounds`: saved `100.4,50.6 700.5×500.2` on `[main]` → whole numbers (`100,51 701×500` or the same rounding throughout; assert `Number.isInteger` on each field).
  - `no work areas gives first-start bounds`: `placeWindow(r, [])` → `null`.
- [ ] **Step 2: Run** `npx vitest run src/main/window.test.ts`. Expected: FAIL (module missing).
- [ ] **Step 3: Implement** `placeWindow` in `src/main/window.ts`.
- [ ] **Step 4: Run** `npx vitest run src/main/window.test.ts`, then `npm run typecheck` and `npm run lint`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(ui): place the saved window on a connected screen`.

### Task 3: Restore and save the window

**Files:**
- Modify: `src/main.ts` (`createWindow`, `second-instance`)
- Create: `e2e/window.spec.ts`
- Modify: `e2e/panel.spec.ts` (restart tests, see Step 5), and any other restart spec the full run shows to depend on the window's size.

**Interfaces:**
- Consumes: Task 1 `loadWindowState`, `saveWindowState`, `WindowState`; Task 2 `placeWindow`.
- Produces, in `main.ts`:
  - `createWindow(dataDir: string)`. Both callers (`whenReady` and `activate`) pass `dataDir`. It reads the state, places it with `placeWindow(saved, screen.getAllDisplays().map((d) => d.workArea))`, and passes the placed rectangle to `new BrowserWindow`, or `width: 1200, height: 800` when there is no state or no placement.
  - A module-level `reveal()`, used by `ready-to-show`, `did-fail-load`, the 5 s timer and `second-instance`. If the window is hidden, it calls `maximize()` then `focus()` when the state said maximized, else `show()`. `second-instance` keeps `restore()` for a minimized window and `focus()`.
  - A save handler on `close`, `moved`, `resized`, `maximize` and `unmaximize`, per R1.1: `maximized = isMaximized() && !isMinimized()`; bounds from `getNormalBounds()` when maximized or minimized, else `getBounds()`. It wraps `saveWindowState` in try/catch with `console.error`.

- [ ] **Step 1: Write the failing e2e tests** in `e2e/window.spec.ts`, using `launch` from `e2e/helpers.ts`. Local helpers: read `{ bounds, normal, maximized, minimized, visible }` of the first window and the primary `workArea` through `app.evaluate`; write `settings.json` into a userData folder before `launch(userData)`; `firstStart()` launches a fresh app, reads its bounds and closes it (cache per file). Tests, one per spec scenario:
  - `the window comes back where it was`: `setBounds({ x: wa.x + 40, y: wa.y + 30, width: 700, height: 500 })`, `app.close()`, relaunch with the same userData → those bounds, not maximized.
  - `a maximized window comes back maximized`: at 700×500, `maximize()`, close, relaunch → maximized; `unmaximize()` → the 700×500 bounds.
  - `a move by the person is saved before the app closes`: `setBounds` to 700×500, then `emit("moved")` → poll `settings.json`'s `window` to equal the bounds with `maximized: false`.
  - `a window closed while minimized opens normal`: 700×500, `maximize()`, `minimize()`, close, relaunch → visible, not minimized, not maximized, 700×500.
  - `a failed save does not stop the app from closing`: `mkdirSync(<userData>/settings.json)`, launch → `firstStart()` bounds; `setBounds` and `emit("moved")`, then `await app.close()` completes (an uncaught throw would leave Electron's error dialog open and the close would time out).
  - `a short saved window is held at the minimum height`: saved `{ x: wa.x + 40, y: wa.y + 30, width: 700, height: 300, maximized: false }` → height 480.
  - `a broken saved state opens the window as on a first start`: for each spec row, write the state, launch, compare to `firstStart()`, close.
  - `a window partly off the screen is pulled onto it`: saved `x: wa.x + wa.width - 350`, `y: wa.y + 30`, 700×500 → `x + width` is `wa.x + wa.width`, `y` unchanged, 700×500.
  - `a window bigger than the screen is shrunk to fit`: saved at `wa.x, wa.y`, `width: wa.width + 500`, `height: 500` → width `wa.width`, inside the work area.
  - `a window on a screen that is gone opens on the main screen`: saved at `-30000, -30000` 700×500 → `firstStart()` bounds.
  - `a maximized window on a screen that is gone opens maximized on the main screen`: same with `maximized: true` → maximized, and `screen.getDisplayMatching(getBounds())` is the primary display.
- [ ] **Step 2: Run** `npm run package && npx playwright test e2e/window.spec.ts`. Expected: FAIL (the window always opens at first-start bounds).
- [ ] **Step 3: Implement** the Interfaces above in `src/main.ts`.
- [ ] **Step 4: Run** `npm run package && npx playwright test e2e/window.spec.ts e2e/startup.spec.ts`, then `npm run typecheck` and `npm run lint`. Expected: PASS.
- [ ] **Step 5: Adjust restart specs** (Review Focus 5). In `e2e/panel.spec.ts`, `the height survives a restart` and `releasing a drag above the window keeps the maximum and ends the drag` first set the window to 600 px tall (`setWindowHeight(app, 600)`, which fits CI's work area), so the restart restores the same height. Run `npm run package && npx playwright test e2e/panel.spec.ts`. Expected: PASS.
- [ ] **Step 6: Commit** `feat(ui): restore the window's bounds and maximized state at start` (test changes to `panel.spec.ts` may go in a separate `test(e2e): ...` commit).

### Task 4: Full check and ship

**Files:**
- Create: `docs/checks/2026-10-26-window-bounds.md`
- Modify: `docs/features.md` (F41 set to `done`)

- [ ] **Step 1: Run** `npm run e2e`. Expected: every test passes, the existing ones included.
- [ ] **Step 2: Run** `npm test`, `npm run typecheck` and `npm run lint`. Expected: PASS.
- [ ] **Step 3: Write** `docs/checks/2026-10-26-window-bounds.md` in the form of `docs/checks/2026-10-26-ui-polish.md`: the automated results with the machine and the date, and the spec's by-hand scenarios as open items for a person (two screens, second screen gone, partly off the screen, snapped), plus a second launch during start-up with a maximized saved state (R2.4).
- [ ] **Step 4: Update** `docs/features.md`: F41 `done`.
- [ ] **Step 5: Commit** `docs: add the window bounds app check and mark f41 done`.
