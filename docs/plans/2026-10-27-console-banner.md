# Console banner Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the empty black console with a startup banner (app, Karate and Java versions, project, tests found) and a state line, on a console panel darker than the window in dark mode.

**Architecture:** `scripts/fetch-vendor.mjs` exports its pinned versions. The renderer's Vite config and the Vitest jsdom project inject them, with the app version, as the build-time constant `__KK_VERSIONS__`. Pure functions in `src/renderer/banner.ts` turn `AppState` into the tests text and a state key. `Banner.tsx` renders them, and `Console` renders the banner as its first child inside the scroller. No main-process, preload or IPC change.

**Tech Stack:** SolidJS 2 RC, Tailwind CSS v4, Vite (`define`), Vitest 5 (node and jsdom projects), Playwright `_electron`.

**Spec:** [docs/specs/2026-10-27-console-banner.md](../specs/2026-10-27-console-banner.md)

## Global Constraints

- No new dependency. Solid 2 rules from AGENTS.md (`@solidjs/web`, `Show`, `For`, no `classList`).
- Do not touch `src/main.ts`, `src/main/**`, `src/preload.ts`, `src/shared/ipc.ts`, `src/shared/events.ts` (F27 is changing them).
- Versions: Java `21.0.12.1+1`, Karate `2.1.4`, each written once in `scripts/fetch-vendor.mjs`. SHA-256 pins unchanged.
- Exact texts: `karate-kid <app> · Karate <karate> · Java <java>`; labels `project` and `tests` padded to 9 characters (`"project".padEnd(9)`, `"tests".padEnd(9)`); `none`; `<n> scenarios in <m> features` with `1 scenario` / `1 feature`; `No feature files found`; `finding tests…`; `could not be listed: see the message in the tree`; `Open a Karate project folder`; `Finding tests…`; `Starting Karate…`; `Ready. Press Run all, or ▶ on a test.` (`…` is U+2026, `▶` is U+25B6).
- Colours: app name `text-sky-400`, `Ready.` `text-emerald-400`, labels and state text `text-zinc-400`, values the console's `text-zinc-200`. Console `bg-zinc-900 dark:bg-zinc-950` plus `dark:shadow-[inset_0_2px_6px_rgb(0_0_0/0.6)]`. ERROR/WARN line colours unchanged.
- Never run `npm test` and `npm run e2e` at the same time.

## Review Focus

- Outline example rows count as scenarios and the outline itself does not (Karate counts rows): Task 2 `counts outline rows, not outlines`.
- A cancelled run, or one that ends with no console line, brings back `Ready.` rather than leaving `Starting Karate…`: Task 2 `ready again after a run that printed nothing`.
- A tag filter that hides tests does not change the counts: Task 2 `counts ignore the tag filter`.
- A long project path wraps inside the console and the tests line still starts on its own line: Task 3 `a long project path stays on its own line`.
- Light mode keeps today's console colour exactly: Task 4 `the console is darker than the window in dark mode only`.

Note: on a first open, main sends the `project` event only after discovery, so the window shows `project  none` with `Finding tests…` until the tree arrives. The `tests    finding tests…` text (R1.4) shows only when a project is shown without a tree while busy. Both follow the spec as written.

---

### Task 1: Pinned versions exported from fetch-vendor

**Files:**
- Modify: `scripts/fetch-vendor.mjs:13-28`
- Test: `scripts/fetch-vendor.test.ts`

**Interfaces:**
- Produces: `export const VERSIONS = { java: "21.0.12.1+1", karate: "2.1.4" }`, and `JRE`, `JAR`, `LICENSE` exported (same `{ url, sha256, file }` shape as today).

- [ ] **Step 1: Write the failing tests** in `scripts/fetch-vendor.test.ts`, a new `describe("pins")`:

```ts
it("pins each version once", () => {
  expect(VERSIONS).toEqual({ java: "21.0.12.1+1", karate: "2.1.4" });
});
it("builds the download URLs and file names from the versions", () => {
  expect(JRE.url).toBe(
    "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.12.1%2B1/OpenJDK21U-jre_x64_windows_hotspot_21.0.12.1_1.zip",
  );
  expect(JAR.url).toBe("https://github.com/karatelabs/karate/releases/download/v2.1.4/karate-2.1.4.jar");
  expect(JAR.file).toBe("karate-2.1.4.jar");
  expect(LICENSE.url).toBe("https://raw.githubusercontent.com/karatelabs/karate/v2.1.4/LICENSE");
  expect([JRE.sha256, JAR.sha256, LICENSE.sha256]).toEqual([
    "d35f31e712f0fcf6ac5a093edc90204fbff22f720ba3950bd09d331d5e621636",
    "1f743f4acc56526b39bd88c84bfe4491c46d5e94f4ab5a38c437c9400dbc2488",
    "28890ebdc037407a0e1168ce3b01880d5fbfbc6381f37629d25c070482ea0cea",
  ]);
});
```

- [ ] **Step 2: Run** `npx vitest run scripts/fetch-vendor.test.ts`. Expected: FAIL, `VERSIONS` is not exported.

- [ ] **Step 3: Implement.** Add `VERSIONS` above `JRE`, export the three pin objects, and build every URL and versioned file name from `VERSIONS` with template strings: the Java major is `VERSIONS.java.split(".")[0]`, the release tag is `jdk-${encodeURIComponent(VERSIONS.java)}`, the zip name uses `VERSIONS.java.replace("+", "_")`. Add a one-line comment that the renderer shows these versions (the banner).

- [ ] **Step 4: Run** `npx vitest run scripts/fetch-vendor.test.ts`. Expected: PASS. Then `node scripts/fetch-vendor.mjs` with a populated `vendor/`: prints `vendor ready` (the jar check still passes).

- [ ] **Step 5: Commit** `refactor(build): pin each bundled version once in fetch-vendor`.

### Task 2: Banner text from the app state

**Files:**
- Create: `src/renderer/banner.ts`
- Test: `src/renderer/banner.test.ts` (node project)

**Interfaces:**
- Consumes: `AppState`, `initialState()` (`src/renderer/reduce.ts`), `leaves(n)` (`src/renderer/nodes.ts`), `TestNode` (`src/shared/model.ts`).
- Produces:
  - `export type BannerState = "none" | "finding" | "starting" | "ready";`
  - `export function testsText(s: AppState): string | null` — the tests line's value, null for no line.
  - `export function bannerState(s: AppState): BannerState | null` — null once the console has lines.

Rules, in this order. `testsText`: no project → null; `s.error` → `could not be listed: see the message in the tree`; no tree → `finding tests…` if `s.busy`, else null; no leaves → `No feature files found`; else `<n> scenario(s) in <m> feature(s)`, n = `leaves(tree).length`, m = nodes of kind `"feature"` anywhere in the tree. `bannerState`: `s.console.length > 0` → null; `s.run.active` → `"starting"`; `s.busy` → `"finding"`; no project → `"none"`; else `"ready"`.

- [ ] **Step 1: Write the failing tests.** Build states from `initialState()` and a small tree helper (root folder → features → scenarios, outlines with `row` children). Tests and assertions:
  - `no project`: `testsText` null, `bannerState` `"none"`.
  - `counts scenarios and features`: 3 scenarios in 2 features → `"3 scenarios in 2 features"`; `bannerState` `"ready"`.
  - `singular`: 1 and 1 → `"1 scenario in 1 feature"`.
  - `counts outline rows, not outlines`: one feature with a scenario and an outline of 2 rows → `"3 scenarios in 1 feature"`.
  - `counts ignore the tag filter`: same tree with `s.filter.include = ["smoke"]` and `hideNonMatching: true` → same text.
  - `no tests`: tree with no leaves → `"No feature files found"`.
  - `finding tests`: project set, `tree: null`, `busy: true` → `"finding tests…"`; `bannerState` `"finding"`. With `busy: false` → `testsText` null.
  - `discovery error`: `error: "boom"` with a project → `"could not be listed: see the message in the tree"`.
  - `starting`: `run.active: true`, `busy: true`, empty console → `"starting"`.
  - `output hides the state line`: `console: ["x"]` → `bannerState` null, in every other state.
  - `ready again after a run that printed nothing`: `run.active: false`, `run.outcome: "cancelled"`, `busy: false`, empty console → `"ready"`.

- [ ] **Step 2: Run** `npx vitest run src/renderer/banner.test.ts`. Expected: FAIL, module not found.

- [ ] **Step 3: Implement** both functions in `src/renderer/banner.ts`, keeping it free of Solid and DOM imports.

- [ ] **Step 4: Run** `npx vitest run src/renderer/banner.test.ts`. Expected: PASS.

- [ ] **Step 5: Commit** `feat(ui): derive the console banner text from the app state`.

### Task 3: The banner in the console

**Files:**
- Create: `src/renderer/Banner.tsx`
- Modify: `src/renderer/Console.tsx`, `src/renderer/BottomPanel.tsx:104-106`, `src/index.css`, `src/declarations.d.ts`, `vite.renderer.config.mts`, `vitest.config.mts`
- Test: `src/renderer/Banner.test.tsx`, `src/renderer/Console.test.tsx`

**Interfaces:**
- Consumes: `VERSIONS` (Task 1), `testsText`, `bannerState`, `BannerState` (Task 2).
- Produces:
  - Global `declare const __KK_VERSIONS__: { app: string; karate: string; java: string };` in `src/declarations.d.ts`.
  - `vite.renderer.config.mts` exports `define = { __KK_VERSIONS__: JSON.stringify({ app: pkg.version, ...VERSIONS }) }` (`pkg` from `./package.json` with `{ type: "json" }`, `VERSIONS` from `./scripts/fetch-vendor.mjs`) and passes it to `defineConfig`. The `dom` project in `vitest.config.mts` imports that `define` and sets it next to its `plugins`.
  - `Banner(props: { state: AppState })`.
  - `Console(props: { lines: string[]; children?: JSX.Element })`: children render first inside the scroller, before the lines.
  - Test ids: `console-banner`, `banner-versions`, `banner-project`, `banner-tests`, `console-state`, `console-cursor`.

- [ ] **Step 1: Write the failing tests** in `src/renderer/Banner.test.tsx`, rendering `<Banner state={s} />` with states built from `initialState()`:
  - `names the pinned versions`: `banner-versions` text is `` `karate-kid ${pkg.version} · Karate ${VERSIONS.karate} · Java ${VERSIONS.java}` `` (import both in the test); `karate-kid` sits in a `text-sky-400` span.
  - `no project`: `banner-project` text `"project  none"`; no `banner-tests`; `console-state` text `"Open a Karate project folder"`.
  - `ready`: project root `D:\work\orders-api`, 3 scenarios in 2 features → `"project  D:\\work\\orders-api"`, `"tests    3 scenarios in 2 features"`, `console-state` text `"Ready. Press Run all, or ▶ on a test."` with `Ready.` in a `text-emerald-400` span.
  - `finding` and `starting`: `console-state` text `"Finding tests…"` and `"Starting Karate…"`.
  - `the cursor`: `console-cursor` has `aria-hidden="true"`, and its class has `animate-blink` and `motion-reduce:animate-none`.
  - `output hides the state line`: `console: ["x"]` → no `console-state`, banner lines still there.
  - `a long project path stays on its own line`: a 300-character root → `banner-project` holds the whole root, and `banner-tests` is a separate element after it.

  In `src/renderer/Console.test.tsx`, add `the banner survives the line cap`: render `<Console lines={store.state.console}><p data-testid="head">h</p></Console>` with the store setup of the existing cap test, append `CONSOLE_LIMIT + 10` lines; `screen.getByTestId("console").firstElementChild` is `head`, and the follow assertion of that test still holds.

- [ ] **Step 2: Run** `npx vitest run src/renderer/Banner.test.tsx src/renderer/Console.test.tsx`. Expected: FAIL, `Banner` not found and `head` not first.

- [ ] **Step 3: Implement.**
  - The define wiring and declaration above.
  - `src/index.css`: an `@theme` block with `--animate-blink: blink 1.1s steps(1) infinite;` and `@keyframes blink { 50% { opacity: 0; } }`.
  - `Banner.tsx`: one `div` per line, label spans `text-zinc-400` padded with `padEnd(9)`, the state line `text-zinc-400` followed by a block cursor `<span data-testid="console-cursor" aria-hidden="true" class="ml-1 inline-block h-3 w-[0.6em] translate-y-0.5 bg-zinc-200 animate-blink motion-reduce:animate-none" />`.
  - `Console.tsx`: render `{props.children}` before the `For`; change the background classes per Global Constraints.
  - `BottomPanel.tsx`: `<Console lines={props.state.console}><Banner state={props.state} /></Console>`.

- [ ] **Step 4: Run** `npm test`, `npm run typecheck`, `npm run lint`. Expected: all pass.

- [ ] **Step 5: Commit** `feat(ui): greet with a startup banner in the console (f24)`.

### Task 4: End-to-end checks

**Files:**
- Create: `e2e/console.spec.ts`

**Interfaces:**
- Consumes: `launch`, `openProject`, `project`, `nodeByKey`, `test`, `expect` (`e2e/helpers.ts`); `VERSIONS` from `../scripts/fetch-vendor.mjs`; test ids from Task 3; `run-all-button`, `run-outcome` (existing).

- [ ] **Step 1: Write the tests.**
  - `open, run and run again`: open `project("clean")`; wait for the tree; `console-banner` contains `Karate ${VERSIONS.karate}` and `Java ${VERSIONS.java}`; `banner-project` contains the project folder; `banner-tests` matches `/^tests {4}\d+ scenarios? in \d+ features?$/`; `console-state` contains `Ready.`. Press Run all, wait for `run-outcome` `Finished` (60 s); `console-banner` is the console's first child and `console-state` is gone. Pick a console line from that run that names its own run folder (each run has its own `-o` folder, ADR 0010); if Karate prints none, use the run's line count instead. Press Run all again and wait: `console-banner` count is 1, and the picked line is gone (or the line count is under 1.5 times the first run's).
  - `the console is darker than the window in dark mode only`: launch with `colorScheme: "dark"`, read `getComputedStyle(...).backgroundColor` of `console` and `app`; then the same with `"light"`. Dark: console differs from app. Light: console equals the dark app's colour (both `zinc-900`).

- [ ] **Step 2: Build and run** `npm run e2e -- e2e/console.spec.ts` (not while `npm test` runs). Expected: PASS. Then the whole `npm run e2e`: PASS.

- [ ] **Step 3: Commit** `test(e2e): check the console banner and its dark panel`.

### Task 5: Check by hand and ship the docs

**Files:**
- Create: `docs/checks/2026-10-27-console-banner.md` (same shape as `docs/checks/2026-10-26-ui-polish.md`)
- Modify: `docs/features.md` (F24 row: `done`)

- [ ] **Step 1:** Start the app (`npm start`) in dark mode, then light mode. Record in the check file: the banner before any run, `Finding tests…` on Refresh, `Starting Karate…` after Run all, output under the banner, the second run's reset, the console darker than the window in dark mode only, the cursor blinking and still with Windows "Animation effects" off. Steps a person still has to confirm are marked as such.
- [ ] **Step 2:** Set the F24 row's status to `done`.
- [ ] **Step 3: Commit** `docs: record the console banner checks and ship f24`.
