# Rename to Grace Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the app as Grace, with LICENSE and the non-affiliation and bundled-software notices.

**Architecture:** String changes in config, markup, tests and docs. Nothing moves the old `%APPDATA%\karate-kid` folder: AGENTS.md forbids backward compatibility while the app is in development.

**Tech Stack:** Electron 45, TypeScript, Vitest 5, Playwright 1.64.

**Spec:** [ADR 0013](../decisions/0013-rename-the-app-to-grace.md). The icon and mascot are out of scope (own branch).

## Global Constraints

- Branch `feature/rename-to-grace`. Conventional Commits, lower-case subjects, AGENTS.md rules. No new dependency.
- Names: `package.json` `name` `grace`, `productName` `Grace`; env var `GRACE_USER_DATA`; localStorage key `grace.panelHeight`.
- No migration, fallback or alias for the old names (AGENTS.md, No backward compatibility).
- Dated specs, plans, checks and research notes keep the old name. `scripts/bump-pr.test.ts` keeps its `github.com/Tusquito/karate-kid` URLs (the repo is not renamed yet). Internal identifiers (`__KK_VERSIONS__`, `kk-` temp prefixes) stay.
- Run one e2e file with `npm run package && npx playwright test e2e/<file>.spec.ts`. Never run `npm test` and `npm run e2e` at the same time.

## Review Focus

1. **A leftover old name** in user-facing text, config or tests: Task 1 Step 3's grep lists none outside the dated docs.
2. **The e2e launcher still passing the old env var**: every e2e app would share the real `%APPDATA%\Grace`. Test: Task 1 Step 4 runs specs that read their own userData.
3. **The startup screen markup** is asserted verbatim by `e2e/startup.spec.ts`. Test: Task 1 Step 4.

---

### Task 1: Rename the app

**Files:**
- Modify: `package.json` (`name`, `productName`), `package-lock.json` (both top `name` fields), `index.html` (`<title>`, startup `aria-label` `Loading Grace` and text `Grace`), `src/main.ts` (`GRACE_USER_DATA`), `src/renderer/Banner.tsx` (sky-400 span text `Grace`), `src/renderer/Banner.test.tsx`, `src/renderer/panelHeight.ts` (`PANEL_KEY`), `e2e/helpers.ts` (`launch()` env), `e2e/startup.spec.ts:34`, `e2e/panel.spec.ts:164,173`.

- [ ] **Step 1:** Change `Banner.test.tsx` to expect `Grace ${pkg.version} · …` and `getByText("Grace")`. Run `npx vitest run src/renderer/Banner.test.tsx`. Expected: FAIL.
- [ ] **Step 2:** Make the other changes above. Run `npx vitest run src/renderer`. Expected: PASS.
- [ ] **Step 3:** `git grep -n -i "karate-kid\|KARATE_KID" -- ':!docs/specs' ':!docs/plans' ':!docs/checks' ':!docs/research' ':!docs/decisions' ':!scripts/bump-pr.test.ts'` lists only `README.md`, `AGENTS.md` and `docs/features.md` (Task 2).
- [ ] **Step 4:** `npm run package && npx playwright test e2e/startup.spec.ts e2e/panel.spec.ts e2e/env.spec.ts`. Expected: all pass.
- [ ] **Step 5: Commit** `feat(ui)!: rename the app to grace` with a `BREAKING CHANGE:` footer: the exe, zip and data folder are named Grace; data in `%APPDATA%\karate-kid` is not carried over.

### Task 2: Licence and notices

**Files:**
- Create: `LICENSE` (MIT, `Copyright (c) 2026 Tusquito`)
- Modify: `README.md`, `AGENTS.md:3`, `docs/features.md:3`

- [ ] **Step 1:** README: title `# Grace`, the easter egg in one line, a `## Licence` section (MIT; not affiliated with or endorsed by Karate Labs Inc.; Karate is their trademark), a `## Bundled software` section (Temurin JRE, GPLv2 with the Classpath Exception, source at `SOURCE_REPO` and `SOURCE` in `resources/jre/release`; Karate, MIT; licence texts in `resources/licenses`).
- [ ] **Step 2:** AGENTS.md and features.md say Grace; AGENTS.md notes the repo and folder are still named karate-kid until the maintainer renames them.
- [ ] **Step 3:** `npm run lint && npm run typecheck && npm test`. Expected: all pass.
- [ ] **Step 4: Commit** `docs: add the mit licence and the grace notices`.
