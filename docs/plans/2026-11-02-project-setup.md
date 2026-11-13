---
status: built
issue: 64
summary: "Grace reads karate-pom.json itself and passes --no-pom, names unused pom keys in a notice, and puts Maven and Gradle output on the classpath"
---

# Project setup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Grace reads `karate-pom.json` itself (`paths`, `configDir`) and passes `--no-pom`, names the pom keys it does not use in a notice on open, and puts Maven and Gradle compiled output on the classpath.

**Architecture:** A new `readPom(root)` in `src/main/karate/pom.ts` parses the file. `detectLayout(root)` in `command.ts` uses it to give feature roots, the classpath (source then compiled folders), the `-g` value, the folders environments come from, and the open notice. `buildArgs` adds `--no-pom` and `-g`. The runner passes the new fields through and emits the notice on open.

**Tech Stack:** TypeScript, Node `fs`/`path`, Vitest, Playwright.

**Spec:** [docs/specs/2026-11-02-project-setup.md](../specs/2026-11-02-project-setup.md)

## Global Constraints

- Flags: `--no-pom` right after `--no-color`; `-g <configDir as written>` on dry runs and runs.
- Compiled folders, in order, after the source folders: Maven `target/test-classes`, `target/classes`; Gradle `build/classes/java/test`, `build/resources/test`, `build/classes/java/main`, `build/resources/main`.
- Notice copy, exact: `Grace does not use these karate-pom.json settings: <keys, comma-separated, file order>.` / `These karate-pom.json paths are not folders or files in the project: <entries as written, comma-separated>.` / `karate-pom.json could not be read: <reason>. Grace uses the default folders.` Sentences joined by one space.
- No timer, watcher or poll; the pom is read only before a dry run or run.

## Review Focus

- A `paths` entry of `.` (the project folder itself) is inside the folder and kept.
- A folder whose name starts with `..` (`..data`) is inside the project folder, not outside.
- `paths: []` behaves as absent (default feature roots, no paths sentence).
- A pom that is JSON `null`, an array or a number is "not an object".
- A pom with only `paths` and `configDir` gives no notice at all.

---

### Task 1: `readPom`

**Files:**
- Create: `src/main/karate/pom.ts`, `src/main/karate/pom.test.ts`

**Interfaces:**
- Produces:
  ```ts
  export interface Pom {
    paths: string[] | null;      // as written; null when absent or empty
    configDir: string | null;    // as written
    unused: string[];            // other keys, file order
    error: string | null;        // why it could not be read; then paths/configDir null, unused []
  }
  export const NO_POM: Pom;      // { paths: null, configDir: null, unused: [], error: null }
  export function readPom(root: string): Pom; // NO_POM when <root>/karate-pom.json is absent
  ```

- [ ] **Step 1: Tests** (`pom.test.ts`, temp folders like `command.test.ts`):
  - absent file → `NO_POM`
  - `{"paths":["tests/a"],"configDir":"conf","env":"qa","dryRun":true}` → `{ paths: ["tests/a"], configDir: "conf", unused: ["env","dryRun"], error: null }`
  - `{"paths":[]}` → `paths: null`
  - `{` → `error` is the JSON parser's message (non-empty); `paths` null, `unused` []
  - `null`, `[]`, `3` → `error: "not a JSON object"`
  - `{"paths":"tests"}` and `{"paths":[1]}` → `error: "paths is not a list of strings"`
  - `{"configDir":3}` → `error: "configDir is not a string"`
- [ ] **Step 2:** Run `npx vitest run src/main/karate/pom.test.ts`: fails (no module).
- [ ] **Step 3:** Implement with `readFileSync`; a read error other than ENOENT is an `error` too (its message). JSON parse error: its message.
- [ ] **Step 4:** Tests pass.
- [ ] **Step 5:** Commit `feat(runner): read karate-pom.json paths and configdir`.

### Task 2: `detectLayout` and `buildArgs`

**Files:**
- Modify: `src/main/karate/command.ts` (`ProjectLayout`, `detectLayout`, `buildArgs`)
- Test: `src/main/karate/command.test.ts`

**Interfaces:**
- Consumes: `readPom`, `Pom` (Task 1).
- Produces:
  ```ts
  export interface ProjectLayout {
    layout: Layout;
    classpath: string[];      // source folders, then the compiled folders that exist
    featureRoots: string[];   // pom paths kept, or the source folders
    configDir: string | null; // the -g value, as written
    configDirs: string[];     // where karate-config-<env>.js files are read from
    pomNotice: string | null; // the open notice's pom sentences; null when none apply
  }
  buildArgs(o: { ...today, configDir: string | null }): string[]
  ```
- Rules: `configDirs` is `[...resolved configDir folders, root]` with a pom `configDir` (a `classpath:<f>` value gives `<f>` under each source classpath folder, any other value `path.resolve(root, value)`), else `[root, ...source folders]` (today's overlay folders). Compiled folders are never in `featureRoots` or `configDirs`. A `paths` entry is kept when it does not start with `classpath:`, exists, and `path.relative(root, abs)` is neither `..`, nor starts with `..` + `path.sep`, nor is absolute.

- [ ] **Step 1: Tests:**
  - existing four tests updated to the new shape (`configDir: null`, `configDirs: [root, ...source]`, `pomNotice: null`).
  - maven with `target/test-classes`, `target/classes` → `classpath` = `[src/test/java, src/test/resources, target/test-classes, target/classes]`; `featureRoots` and `configDirs` hold no `target` folder.
  - gradle with `build/classes/java/test`, `build/resources/test` → classpath in the Global Constraints order.
  - standalone with a `target/test-classes` folder → `classpath: [root]`.
  - pom `paths: ["tests/a", "missing", "..", "classpath:x", "."]` with `tests/a` present → `featureRoots: [root/tests/a, root]`; `pomNotice` = `These karate-pom.json paths are not folders or files in the project: missing, .., classpath:x.`
  - pom `paths: ["..data"]` with folder `..data` → kept.
  - pom `{"configDir":"conf","env":"qa"}` → `configDir: "conf"`, `configDirs: [root/conf, root]`, `pomNotice: "Grace does not use these karate-pom.json settings: env."`
  - maven pom `{"configDir":"classpath:conf"}` → `configDirs: [src/test/java/conf, src/test/resources/conf, root]`.
  - unused keys and a left-out path → both sentences, settings first, one space between.
  - unreadable pom → `featureRoots`/`configDir`/`configDirs` as with no pom, `pomNotice` = `karate-pom.json could not be read: <error>. Grace uses the default folders.`
  - `buildArgs`: `base` gains `"--no-pom"` after `"--no-color"`; `configDir: "conf"` puts `"-g", "conf"` right after `-f html,karate:jsonl` and any `-e`; `null` adds nothing.
- [ ] **Step 2:** `npx vitest run src/main/karate/command.test.ts` fails.
- [ ] **Step 3:** Implement. Keep `-T 1` on dry runs; its comment drops the pom reason.
- [ ] **Step 4:** Tests pass; `npm run typecheck` shows only the runner's `buildArgs` call missing `configDir` (Task 3 fixes it).
- [ ] **Step 5:** Commit with Task 3 (the build must typecheck).

### Task 3: Runner

**Files:**
- Modify: `src/main/runner.ts` (`open`, `rediscover`'s overlay line, `spawn`)
- Test: `src/main/runner.setup.test.ts` (new; real Karate, like `runner.env.test.ts`)

**Interfaces:**
- Consumes: `ProjectLayout` (Task 2).

- [ ] **Step 1: Tests** (each in a temp copy written by the test; `setup()`, `T` from `test/runner.ts`):
  - **pom project:** `tests/a/a.feature` (`* match cfg == 'base'`), `tests/b/b.feature`, `conf/karate-config.js` (`cfg: 'base'`), `conf/karate-config-qa.js` (`cfg: 'qa'`), pom `{"paths":["tests/a","missing"],"configDir":"conf","env":"qa","dryRun":true}`. After `open`: the tree has `f:tests/a/a.feature`, not `f:tests/b/b.feature`; the last notice is exactly `Grace does not use these karate-pom.json settings: env, dryRun. These karate-pom.json paths are not folders or files in the project: missing.`; the last `environment` event offers `qa`; after running all, a.feature's scenario passed.
  - **only on open:** after `refresh`, no new `notice` event.
  - **unreadable pom** (`{`): open lists the clean fixture's tests and the notice starts with `karate-pom.json could not be read:` and ends with `Grace uses the default folders.`
  - **with skipped properties** is left to the existing properties tests: assert there only if the join is easy to reach; otherwise rely on Task 3's code review.
- [ ] **Step 2:** Run `npx vitest run src/main/runner.setup.test.ts` (ask the parent session for the go first): fails.
- [ ] **Step 3:** Implement: `spawn` passes `configDir`; overlays come from `configDirs`; `open` emits one notice joining the skipped-properties text and `pomNotice` (in that order, one space), when either applies, only when opened.
- [ ] **Step 4:** Tests pass; `npm run typecheck`, `npm run lint` clean.
- [ ] **Step 5:** Commit `feat(runner): run with --no-pom and the build's compiled classes`.

### Task 4: End-to-end

**Files:**
- Create: `e2e/project-setup.spec.ts`

- [ ] **Step 1:** Two tests, through `launch()` and `openProject()`:
  - **pom:** the Task 3 pom project. The `notice` shows the settings sentence; the tree shows `f:tests/a/a.feature` and not b; `env-input` offers `qa`; Run all ends with a.feature passed.
  - **maven compiled output:** `project("maven")`, plus `target/test-classes/data/built.json` (`{"built":true}`) and `src/test/java/examples/built.feature` (`* def b = read('classpath:data/built.json')` / `* match b.built == true`). Running it passes.
- [ ] **Step 2:** `npm run e2e -- project-setup` (after the parent's go): passes.
- [ ] **Step 3:** Commit `test(e2e): cover karate-pom.json and compiled classes`.

### Task 5: Wrap up

- [ ] Spec status `approved`. Full `npm test` and `npm run e2e` (after the parent's go), summary in the PR.
- [ ] #64: `Spec:` line → the spec on `develop`; `Karate surface:` drops `karate doctor --json` with the reason. Card to `building` only if a second PR follows.
