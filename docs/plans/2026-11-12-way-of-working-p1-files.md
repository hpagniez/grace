# Way of working, phase 1: the files and their split. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the one 37.7 KB `AGENTS.md` with a general `WORKFLOW.md`, a Grace-only `AGENTS.md` and path-scoped rules files, and make CI fail when they outgrow their limits.

**Architecture:** Text moves between Markdown files; nothing in the app changes. One new pure function in `scripts/check-pr.mjs`, `rulesFilesErrors`, checks the files' sizes, the general file's neutrality and the rules files' globs; `main()` calls it, so the required `checks` workflow enforces it on every PR, docs-only ones included.

**Tech Stack:** Markdown, Node 24 (`scripts/check-pr.mjs`, no new dependency), Vitest (`scripts/check-pr.test.ts`), git pathspec globs (`git ls-files ':(glob)<pattern>'`).

**Spec:** [docs/specs/2026-11-12-way-of-working.md](../specs/2026-11-12-way-of-working.md), sections 1 to 4. Section 5 (enforcement of the PR body, hooks, the weekly sweep) is phases 2 and 3, each with its own plan.

## Global Constraints

- Tier: Project. One PR for this phase, `Part of #356`, on a `feature/*` branch from `develop`.
- `WORKFLOW.md` at most 5,000 bytes; `AGENTS.md` at most 9,000 bytes; 14,000 bytes together. Bytes are those of the git blob at the PR's head, not of the working tree (Windows line endings must not change the count).
- `WORKFLOW.md` contains none of `grace`, `karate`, `gpui`, `hpagniez`, whatever the case.
- A rule is never cut to fit. Text moves word for word unless the spec's "Keep, change, cut" table changes it. When `AGENTS.md` is over its limit, more text moves to a rules file or to `docs/process/`.
- `AGENTS.md` keeps the headings `## Git workflow` and `## No backward compatibility`: accepted decision records link them.
- Globs in a `paths:` list use only `*`, `**` and literal characters: no braces, no `!`. Git's pathspec and Claude Code's matcher agree on those.
- No accepted decision record, old spec, plan or check record is edited.
- Commits: Conventional Commits, scope `docs` or `ci`, `Part of #356` in each.

## Review Focus

1. **A glob that matches nothing**, so its rules never load and nobody notices: `rulesFilesErrors` fails on it (Task 1, test "a glob that matches no tracked file").
2. **A rule that applies when no matching file is touched**, moved into a path-scoped file: "never run two of `npm test`, `npm run e2e` and `npm run bench` at once" and the e2e-windows-on-the-second-screen paragraph apply to any agent that runs tests, whatever it edited. They stay in `AGENTS.md` (Task 4, checklist line).
3. **A tool that reads only `AGENTS.md`** never sees `WORKFLOW.md`: `AGENTS.md`'s first paragraph says to follow it (Task 1, test "AGENTS.md points to WORKFLOW.md").
4. **A rule lost in the move**: the one-off comparison of Task 5 lists every line of the old file that is in no new file; each is accounted for in the PR body.
5. **A rule that binds both apps** loading on one side only: the sample-file test of Task 1 checks that `src/index.css` and `crates/grace/src/theme.rs` both match `both-apps.md`.

---

## File structure

| File | Responsibility |
|---|---|
| `WORKFLOW.md` (create) | The general way: tiers, the floor, the repository is the memory, subagents, housekeeping, the person's machine, pull requests |
| `AGENTS.md` (rewrite) | Grace, always loaded: what the app is, commands that any agent may run, docs map, doc rules, no backward compatibility, performance priorities, git workflow in short, what Grace adds to `WORKFLOW.md` |
| `CLAUDE.md` (modify) | `@WORKFLOW.md` and `@AGENTS.md` |
| `.claude/rules/electron.md` (create) | Stack, Solid 2 rules, UI conventions, Electron-only commands |
| `.claude/rules/gpui.md` (create) | GPUI rules, GPUI-only commands |
| `.claude/rules/karate.md` (create) | Karate integration rules |
| `.claude/rules/ci.md` (create) | The workflow table, SignPath, Dependabot and bump workflows, the `gh project` recipe, the `perf` comment and limits rules |
| `.claude/rules/both-apps.md` (create) | The rules that bind both apps until the cut-over |
| `docs/process/release.md` (create) | Release and hotfix steps, the release benchmark |
| `.claude/settings.json` (modify) | Five plugins and claude.ai connectors off |
| `scripts/check-pr.mjs`, `scripts/check-pr.test.ts` (modify) | `rulesFilesErrors`, its tests, its call in `main()` |

---

### Task 1: `rulesFilesErrors` in `check-pr.mjs`

**Files:**
- Modify: `scripts/check-pr.mjs`
- Test: `scripts/check-pr.test.ts`

**Interfaces:**
- Produces: `export function rulePaths(text: string): string[]`, the globs of a rules file's `paths:` frontmatter (a YAML list of quoted or bare strings between the first two `---` lines; `[]` when there is none).
- Produces: `export function rulesFilesErrors(read: (path: string) => string | null, list: (glob: string) => string[], rules: string[]): string[]`. `read` gives a file's text at the PR's head or `null`; `list` gives the tracked files a glob matches; `rules` are the paths of the `.claude/rules/*.md` files. Returns one message per failure, `[]` when all is well.
- Produces: `export const LIMITS = { "WORKFLOW.md": 5000, "AGENTS.md": 9000 }` (the spec's 14,000 together is their sum, so it needs no check of its own).

- [ ] **Step 1: Write the failing tests** in a `describe("rulesFilesErrors", ...)` block. `read` is a lookup in an object literal, `list` returns `["x"]` unless the test says otherwise, and `ok` is a set of files that passes: `WORKFLOW.md` of 100 bytes, `AGENTS.md` starting with `# AGENTS.md\n\nFollow [WORKFLOW.md](WORKFLOW.md).` and holding both kept headings, one rules file with `---\npaths:\n  - "crates/**"\n---\n`.

| Test name | Input | Expected |
|---|---|---|
| accepts files within their limits | `ok` | `[]` |
| WORKFLOW.md is missing | `ok` without it | one error containing `WORKFLOW.md is missing` |
| WORKFLOW.md is over 5,000 bytes | 5,001 `a` | one error containing `WORKFLOW.md is 5001 bytes, over 5000` |
| AGENTS.md is over 9,000 bytes | 9,001 bytes | one error containing `AGENTS.md is 9001 bytes, over 9000` |
| files exactly at their limits | 5,000 and 9,000 bytes | `[]` |
| counts bytes, not characters | `WORKFLOW.md` of 2,501 `é` | error containing `5002 bytes` |
| WORKFLOW.md names the project | text with `Grace` / `KARATE` / `gpui` / `hpagniez` (`it.each`) | error containing `WORKFLOW.md must stay general: it names "<word as found>"` |
| AGENTS.md points to WORKFLOW.md | `AGENTS.md` whose first paragraph lacks `WORKFLOW.md` | error containing `AGENTS.md must start by pointing to WORKFLOW.md` |
| AGENTS.md keeps the headings decision records link | without `## Git workflow` (and, `it.each`, without `## No backward compatibility`) | error containing `AGENTS.md has no "## Git workflow" heading` |
| a rules file without paths | rules file with no frontmatter | error containing `<file> has no paths: list` |
| a glob that matches no tracked file | `list` returns `[]` for `nope/**` | error containing `<file>: "nope/**" matches no tracked file` |
| a glob with braces or ! | `src/**/*.{ts,tsx}`, `!src/x` | error containing `uses braces or !` |

And `describe("rulePaths", ...)`: quoted items, bare items, no frontmatter gives `[]`, a `paths:` after the closing `---` is ignored.

- [ ] **Step 2: Run them and see them fail**

Run: `npx vitest run scripts/check-pr.test.ts`
Expected: FAIL, `rulesFilesErrors is not a function`.

- [ ] **Step 3: Implement `rulePaths`, `LIMITS` and `rulesFilesErrors`** in `scripts/check-pr.mjs`, beside the other `*Error` functions. Sizes with `Buffer.byteLength(text, "utf8")`.

- [ ] **Step 4: Run them and see them pass**

Run: `npx vitest run scripts/check-pr.test.ts`
Expected: PASS, the existing tests included.

- [ ] **Step 5: Commit**

`git commit -m "ci: check the size and globs of the agent rules files" -m "Part of #356"`. Not called from `main()` yet: the files do not exist (Task 6).

### Task 2: `WORKFLOW.md`

**Files:**
- Create: `WORKFLOW.md`

- [ ] **Step 1: Write the file** from the spec's section 2, in this order of headings, and nothing else: `# Way of working`, `## Tiers` (the table as in the spec, with the superpowers skill names), `## The floor`, `## The repository is the memory`, `## Subagents` (the model and effort table; "the largest the project allows"), `## Housekeeping` (done means clean; nothing parked; the scratch folder `<temp>/agent-work/<repository>/<task>/`; no junction or symbolic link; what a leftover is and what an agent may delete on its own; `scripts/tidy.mjs` is named as "the project's tidy check, when it has one", since it arrives in phase 3), `## The person's machine`, `## Pull requests`. Order when sources disagree, as the last line: the project's rules, this file, superpowers, ponytail.
- [ ] **Step 2: Check it**

Run: `node -e "const s=require('fs').readFileSync('WORKFLOW.md');console.log(s.length, /grace|karate|gpui|hpagniez/i.test(s))"`
Expected: a number at most `5000`, then `false`.

- [ ] **Step 3: Commit** `docs: add the general way of working`, `Part of #356`.

### Task 3: the rules files and `docs/process/release.md`

**Files:**
- Create: `.claude/rules/electron.md`, `.claude/rules/gpui.md`, `.claude/rules/karate.md`, `.claude/rules/ci.md`, `.claude/rules/both-apps.md`, `docs/process/release.md`

Each rules file starts with its frontmatter, then a `#` heading, then the text moved word for word from `AGENTS.md` (relative links gain `../../`).

| File | `paths:` | Text moved from `AGENTS.md` |
|---|---|---|
| `electron.md` | `src/**`, `e2e/**`, `bench/**`, `test/**`, `*.config.*`, `package.json` | `## Stack` (intro and bullets), `### Solid 2 rules`, `### UI conventions`; from Commands, the rows `npm start`, `npm run screens:readme`, `npm run make` |
| `gpui.md` | `crates/**`, `patches/**`, `Cargo.*`, `rust-toolchain.toml`, `scripts/e2e-gpui.sh`, `scripts/package-gpui.mjs`, `scripts/bump-cargo.mjs` | `### GPUI rules`, without the two bullets that go to `both-apps.md`; from Commands, the rows from `cargo run -p grace` to the clippy and fmt row |
| `karate.md` | `src/main/karate/**`, `src/main/runner*`, `crates/grace-core/**`, `listener/**`, `test/fixtures/**`, `scripts/capture-fixtures.mjs` | `## Karate integration rules` |
| `ci.md` | `.github/**`, `scripts/check-pr.mjs`, `scripts/bump-*.mjs`, `scripts/vendor.json`, `bench/limits.json` | The workflow table; the SignPath bullet; the Dependabot paragraph; from Doc rules, the `gh issue edit` and `gh project` recipe; from Performance, the `perf` comment and `bench/limits.json` bullets; the Windows-dispatch and stacked-PR bullets |
| `both-apps.md` | `src/index.css`, `src/renderer/**`, `crates/grace/src/theme.rs`, `crates/grace/assets/**` | From GPUI rules: the sentence "Until the cut-over the tokens must equal `src/index.css`'s, which the theme's tests check: change both together" with the colour rule it belongs to, and the icons bullet. The colour rule's first sentence stays in `gpui.md` too |
| `docs/process/release.md` | none (not a rules file) | The two "Before a `release/*` or `hotfix/*` PR into `main`" bullets, the release-candidates bullet, the `release` row's detail, "Never write a release's `Decision:` line" |

Changed while moving, per the spec's verdict table: in `electron.md` and `gpui.md`, the e2e rule becomes "run the end-to-end tests of the app the PR changes. Small: the spec files of the touched area. Feature and Project: the whole suite once, before ready".

- [ ] **Step 1: Create the six files.**
- [ ] **Step 2: Check every glob matches**

Run: `for f in .claude/rules/*.md; do node -e "import('./scripts/check-pr.mjs').then(m=>{for(const g of m.rulePaths(require('fs').readFileSync(process.argv[1],'utf8')))console.log(process.argv[1],g,require('child_process').execFileSync('git',['ls-files',':(glob)'+g],{encoding:'utf8'}).split('\n').filter(Boolean).length)})" $f; done`
Expected: a count above `0` on every line.

- [ ] **Step 3: Commit** `docs: split the agent rules by the files they govern`, `Part of #356`.

### Task 4: `AGENTS.md`, `CLAUDE.md` and the files that cite them

**Files:**
- Modify: `AGENTS.md`, `CLAUDE.md`, `scripts/bump-pr.mjs:64`, `.github/dependabot.yml:13`

- [ ] **Step 1: Rewrite `AGENTS.md`** with these headings, in order:
  - `# AGENTS.md`: first paragraph "Follow [WORKFLOW.md](WORKFLOW.md), the general way of working; this file holds what is Grace's own", then what Grace is (today's intro).
  - `## Rules by area`: a five-row table of the rules files and what each covers, and `docs/process/release.md`. One line: Claude Code loads each with the files it governs; any other tool reads the one for the files it touches.
  - `## Commands`: `npm install` and `npm run vendor`; the rows `npm test`, `npm run e2e`, `npm run bench`, `npm run typecheck`, `npm run lint`, `cargo test --workspace`, `sh scripts/e2e-gpui.sh`; the "never run any two of" paragraph and the second-screen paragraph, whole (Review Focus 2).
  - `## Docs map`: as today, plus a `docs/process/` row.
  - `## Doc rules`: as today, with the verdict table's changes: an issue by tier (a Small docs, CI or chore PR needs none); Gherkin only for behaviour someone using the app sees; `## Cost` as one line "Cost: none" unless the feature adds a dependency, a process, a timer or work on the run path; decision records for a dependency, an architecture or a file format only; the state of long work in the tracking issue's body, edited in place. The `gh` recipes are in `ci.md`.
  - `## No backward compatibility`: as today.
  - `## Performance`: the priorities, the idle rule, the stack-change rule; one line pointing to `ci.md` and `docs/process/release.md` for the rest.
  - `## Git workflow`: the commit types and scopes; the branch table; merge with `--no-ff`; never commit to `main` or `develop`; what Grace adds to `WORKFLOW.md`'s pull requests: `develop` is the default target, `Closes #N` or `Part of #N`, assign to `hpagniez`, attribution, screenshots on `feature/pr-screenshots` (Feature and Project; Small only when the look changes), `merge-when-green` only on the owner's word, never delete `feature/pr-screenshots`.
  - `## Grace's own`: the reference machine (the second screen, `GRACE_USER_DATA`, copy `vendor/`, what a check record must leave out); subagent models `haiku`, `sonnet`, `opus`, never Fable; superpowers paths `docs/specs/` and `docs/plans/`; the three plugins' jobs in one line each.
- [ ] **Step 2: Set `CLAUDE.md`** to exactly two lines: `@WORKFLOW.md`, `@AGENTS.md`.
- [ ] **Step 3: Update the citations**: `scripts/bump-pr.mjs:64` says `.claude/rules/karate.md` in place of `AGENTS.md (Karate integration rules)`; `.github/dependabot.yml:13` says `.claude/rules/electron.md`. The messages of `check-pr.mjs` that end in `(AGENTS.md)` for release records say `(docs/process/release.md)`; the one for a raised limit says `(.claude/rules/ci.md)`. Update the strings `scripts/check-pr.test.ts` and `scripts/bump-pr.test.ts` expect, if they pin them.
- [ ] **Step 4: Check the sizes and the tests**

Run: `wc -c WORKFLOW.md AGENTS.md && npx vitest run scripts/`
Expected: `WORKFLOW.md` at most 5000, `AGENTS.md` at most 9000, total at most 14000; tests PASS. If `AGENTS.md` is over, move text to a rules file or `docs/process/` (Global Constraints) and run again.

- [ ] **Step 5: Commit** `docs: cut AGENTS.md down to what every task needs`, `Part of #356`.

### Task 5: nothing lost, and the settings

**Files:**
- Modify: `.claude/settings.json`
- Create (scratch folder, not committed): the comparison's output

- [ ] **Step 1: List the lines of the old file that are in no new file**

Run, from the repository root (the scratch folder is `<temp>/agent-work/grace/way-of-working/`):
`node -e "const cp=require('child_process'),fs=require('fs');const old=cp.execFileSync('git',['show','origin/develop:AGENTS.md'],{encoding:'utf8'});const now=['WORKFLOW.md','AGENTS.md','docs/process/release.md',...fs.readdirSync('.claude/rules').map(f=>'.claude/rules/'+f)].map(f=>fs.readFileSync(f,'utf8')).join('\n');const norm=s=>s.replace(/\]\((\.\.\/)+/g,'](').replace(/\s+/g,' ').trim();const n=norm(now);for(const l of old.split('\n')){const t=norm(l).replace(/^[-|#\d. ]+/,'');if(t.length>40&&!n.includes(t))console.log(t.slice(0,160))}"`
Expected: only lines the spec's verdict table cuts or changes, and table rows split between files. Fix any other line by moving its text into the right file, and run again.

- [ ] **Step 2: Write the list that remains** into the PR body under `## Cut or changed`, each with the spec's reason (acceptance check 3).
- [ ] **Step 3: `.claude/settings.json`**: add, keeping `attribution`:

```json
"enabledPlugins": {
  "vercel-plugin@vercel": false,
  "supabase@anthropic-plugin-directory": false,
  "firecrawl@anthropic-plugin-directory": false,
  "taste-skill@taste-skill": false,
  "impeccable@impeccable": false
},
"disableClaudeAiConnectors": true
```

- [ ] **Step 4: Commit** `build: turn off the plugins and connectors grace does not use`, `Part of #356`.

### Task 6: enforce it, and the PR

**Files:**
- Modify: `scripts/check-pr.mjs` (`main()`), `scripts/check-pr.test.ts`

**Interfaces:**
- Consumes: `rulesFilesErrors(read, list, rules)` and `rulePaths(text)` from Task 1.

- [ ] **Step 1: Write the failing repository tests** in `scripts/check-pr.test.ts`, `describe("the repository's rules files", ...)`, reading the working tree and `git ls-files`:
  - "pass rulesFilesErrors": `rulesFilesErrors` over the real files returns `[]`.
  - "load by area" (`it.each`), the rules files whose globs match each sample file, by `git ls-files ':(glob)<glob>'` containing it:

| Sample file | Rules files that match, exactly |
|---|---|
| `crates/grace/src/app.rs` | `gpui.md` |
| `crates/grace-core/src/karate/events.rs` | `gpui.md`, `karate.md` |
| `crates/grace/src/theme.rs` | `gpui.md`, `both-apps.md` |
| `src/renderer/App.tsx` | `electron.md`, `both-apps.md` |
| `src/index.css` | `electron.md`, `both-apps.md` |
| `src/main/karate/http.ts` | `electron.md`, `karate.md` |
| `.github/workflows/checks.yml` | `ci.md` |
| `docs/features.md` | none |

- [ ] **Step 2: Run them**: `npx vitest run scripts/check-pr.test.ts`. Expected: PASS for both if Tasks 2 to 4 are right; fix the globs otherwise (a failure here is a real defect in a rules file, not in the test).
- [ ] **Step 3: Call it from `main()`**: `read` is the existing `show(HEAD_SHA, f)`; `list` is `git ls-files ':(glob)<glob>'` on the tree CI checked out for the PR; `rules` are the `.claude/rules/*.md` paths of `git ls-tree --name-only HEAD_SHA .claude/rules/`. Push each message into `errors`.
- [ ] **Step 4: Run the checker as CI does**

Run: `HEAD_REF=feature/way-of-working-files BASE_REF=develop BASE_SHA=origin/develop HEAD_SHA=HEAD node scripts/check-pr.mjs; echo "exit $?"`
Expected: no `::error::` line, `exit 0`.

- [ ] **Step 5: Run the whole gate**

Run: `npm run lint && npm run typecheck && npm test`
Expected: all pass.

- [ ] **Step 6: Commit** `ci: fail a pr whose agent rules files break their limits`, `Part of #356`.
- [ ] **Step 7: Open the PR** (draft, into `develop`, assigned to `hpagniez`, `Part of #356`, `Tier: project`). Its body holds: the byte counts of the files loaded at start, before (`AGENTS.md` 37,722 and `CLAUDE.md` 11) and after; `## Cut or changed` from Task 5; a request to the owner to open a fresh session and run `/context` and `/memory` once (plugins off, connectors off, rules loading by path), as a second reading and not a gate; the test summary.
- [ ] **Step 8: Update #356's body**: its `Spec:` line points at the spec on `develop`; a "State" section says phase 1 is in review and phases 2 and 3 are next.

---

## Not in this phase

The `Tier:` line and the PR-body checks, the shorter PR template (phase 2); `scripts/tidy.mjs`, the hooks and the `housekeeping` workflow (phase 3); skipping `perf` on `crates/`-only PRs (a follow-up of its own, on the owner's word of 2026-11-12).
