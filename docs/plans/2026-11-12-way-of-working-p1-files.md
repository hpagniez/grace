---
status: built
issue: 356
summary: "Split AGENTS.md into WORKFLOW.md, a Grace-only AGENTS.md and path-scoped rules files, and make CI fail when they outgrow their limits"
---

# Way of working, phase 1: the files and their split. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the one 37.7 KB `AGENTS.md` with a general `WORKFLOW.md`, a Grace-only `AGENTS.md` and path-scoped rules files, and make CI fail when they outgrow their limits.

**Architecture:** Text moves between Markdown files; nothing in the app changes. A new workspace crate, `crates/xtask`, holds the project's scripts in Rust; its first command, `check-rules`, checks the files' sizes, the general file's neutrality and the rules files' globs. The required `checks` workflow runs it on every PR, docs-only ones included.

**Tech Stack:** Markdown; Rust 1.99.1 (the workspace's pin), standard library only; `git` called with an argument list (`git show`, `git ls-files ':(glob)<pattern>'`).

**Spec:** [docs/specs/2026-11-12-way-of-working.md](../specs/2026-11-12-way-of-working.md), sections 1 to 5. Section 6 (the PR checks, the refusals, the tidy report) is phases 2 and 3, each with its own plan.

## Global Constraints

- Tier: Project. One PR for this phase, `Part of #356`, on a `feature/*` branch from `develop`.
- `WORKFLOW.md` at most 5,000 bytes; `AGENTS.md` at most 9,000 bytes. Bytes are those of the git blob at `HEAD`, not of the working tree (Windows line endings must not change the count).
- `WORKFLOW.md` contains none of `grace`, `karate`, `gpui`, `hpagniez`, whatever the case.
- A rule is never cut to fit. Text moves word for word unless the spec's "Keep, change, cut" table changes it. When `AGENTS.md` is over its limit, more text moves to a rules file or to `docs/process/`.
- `AGENTS.md` keeps the headings `## Git workflow` and `## No backward compatibility`: accepted decision records link them.
- Globs in a `paths:` list use only `*`, `**` and literal characters: no braces, no `!`. Git's pathspec and Claude Code's matcher agree on those.
- `crates/xtask`: `#![forbid(unsafe_code)]`, no dependency, the workspace's lints; programs are started with `std::process::Command` and an argument list, never through a shell; each command is a module of pure functions with unit tests, and `main` only reads arguments, calls them and sets the exit code.
- No new JavaScript, Python or shell file. `scripts/check-pr.mjs` changes only where it cites a section of `AGENTS.md` (it is ported in phase 2).
- No accepted decision record, old spec, plan or check record is edited.
- Commits: Conventional Commits, `Part of #356` in each.

## Review Focus

1. **A glob that matches nothing**, so its rules never load and nobody notices: `rules_files_errors` fails on it (Task 1, test `a_glob_that_matches_no_tracked_file`).
2. **A rule that applies when no matching file is touched**, moved into a path-scoped file: "never run two of `npm test`, `npm run e2e` and `npm run bench` at once" and the second-screen paragraph apply to any agent that runs tests, whatever it edited. They stay in `AGENTS.md` (Task 4, Step 1).
3. **A tool that reads only `AGENTS.md`** never sees `WORKFLOW.md`: `AGENTS.md`'s first paragraph says to follow it (Task 1, test `agents_md_points_to_workflow_md`).
4. **A rule lost in the move**: the one-off comparison of Task 5 lists every line of the old file that is in no new file; each is accounted for in the PR body.
5. **A rule that binds both apps** loading on one side only: the sample-file test of Task 6 checks that `src/index.css` and `crates/grace/src/theme.rs` both match `both-apps.md`.

---

## File structure

| File | Responsibility |
|---|---|
| `WORKFLOW.md` (create) | The general way: tiers, the floor, the repository is the memory, subagents, scripts, housekeeping, the person's machine, pull requests |
| `AGENTS.md` (rewrite) | Grace, always loaded: what the app is, the commands any agent may run, docs map, doc rules, no backward compatibility, performance priorities, git workflow in short, what Grace adds to `WORKFLOW.md` |
| `CLAUDE.md` (modify) | `@WORKFLOW.md` and `@AGENTS.md` |
| `.claude/rules/electron.md` (create) | Stack, Solid 2 rules, UI conventions, Electron-only commands |
| `.claude/rules/gpui.md` (create) | GPUI rules, GPUI-only commands |
| `.claude/rules/karate.md` (create) | Karate integration rules |
| `.claude/rules/ci.md` (create) | The workflow table, SignPath, Dependabot and bump workflows, the `gh` recipes, the `perf` comment and limits rules |
| `.claude/rules/both-apps.md` (create) | The rules that bind both apps until the cut-over |
| `docs/process/release.md` (create) | Release and hotfix steps, the release benchmark |
| `.claude/settings.json` (modify) | Five plugins and claude.ai connectors off |
| `crates/xtask/Cargo.toml`, `src/main.rs` (create) | The script crate; `main` dispatches on the first argument and exits 2 with the list of commands on an unknown one |
| `crates/xtask/src/rules.rs` (create) | `check-rules`: the pure checks and their tests |
| `Cargo.toml` (modify), `.cargo/config.toml` (create) | The workspace member; the alias `xtask = "run --quiet --package xtask --"` |
| `.github/workflows/checks.yml` (modify) | One step, `cargo xtask check-rules` |

---

### Task 1: `crates/xtask` and `check-rules`

**Files:**
- Create: `crates/xtask/Cargo.toml`, `crates/xtask/src/main.rs`, `crates/xtask/src/rules.rs`, `.cargo/config.toml`
- Modify: `Cargo.toml` (workspace `members`)

**Interfaces:**
- Produces, in `rules.rs`:
  - `pub const LIMITS: [(&str, usize); 2] = [("WORKFLOW.md", 5000), ("AGENTS.md", 9000)];`
  - `pub const PROJECT_WORDS: [&str; 4] = ["grace", "karate", "gpui", "hpagniez"];`
  - `pub fn rule_paths(text: &str) -> Vec<String>`: the globs of a rules file's `paths:` frontmatter (a YAML list of quoted or bare strings between the first two `---` lines; empty when there is none).
  - `pub fn rules_files_errors(read: &dyn Fn(&str) -> Option<String>, list: &dyn Fn(&str) -> Vec<String>, rules: &[String]) -> Vec<String>`: `read` gives a file's text at `HEAD` or `None`; `list` gives the tracked files a glob matches; `rules` are the paths of the `.claude/rules/*.md` files. One message per failure, empty when all is well.
  - `pub fn matching_rules(file: &str, read: &dyn Fn(&str) -> Option<String>, list: &dyn Fn(&str) -> Vec<String>, rules: &[String]) -> Vec<String>`: the rules files one of whose globs lists `file`.
  - `pub fn run() -> i32`: wires `read` to `git show HEAD:<path>`, `list` to `git ls-files ':(glob)<glob>'`, `rules` to `git ls-files .claude/rules/*.md`; prints each message as `::error::<message>`; returns 1 when there is one, else 0.

- [ ] **Step 1: Create the crate** with `main.rs` dispatching `check-rules` to `rules::run()` and `rules.rs` holding the signatures above with `todo!()` bodies, the workspace member and the alias. `cargo build -p xtask` passes.

- [ ] **Step 2: Write the failing tests** in `rules.rs` (`#[cfg(test)] mod tests`). `read` is a lookup in a `HashMap`, `list` returns `vec!["x"]` unless the test says otherwise, and `ok()` builds a set that passes: `WORKFLOW.md` of 100 bytes, `AGENTS.md` starting with `# AGENTS.md\n\nFollow [WORKFLOW.md](WORKFLOW.md).` and holding both kept headings, one rules file `r.md` with `---\npaths:\n  - "crates/**"\n---\n`.

| Test | Input | Expected |
|---|---|---|
| `accepts_files_within_their_limits` | `ok()` | no error |
| `accepts_files_exactly_at_their_limits` | 5,000 and 9,000 bytes | no error |
| `workflow_md_is_missing` | `ok()` without it | one error containing `WORKFLOW.md is missing` |
| `workflow_md_is_over_its_limit` | 5,001 `a` | one error containing `WORKFLOW.md is 5001 bytes, over 5000` |
| `agents_md_is_over_its_limit` | 9,001 bytes | one error containing `AGENTS.md is 9001 bytes, over 9000` |
| `counts_bytes_not_characters` | `WORKFLOW.md` of 2,501 `é` | error containing `5002 bytes` |
| `workflow_md_names_the_project` | text with `Grace`, `KARATE`, `gpui`, `hpagniez`, one case each | error containing `WORKFLOW.md must stay general: it names "<word as found>"` |
| `agents_md_points_to_workflow_md` | `AGENTS.md` whose first paragraph lacks `WORKFLOW.md` | error containing `AGENTS.md must start by pointing to WORKFLOW.md` |
| `agents_md_keeps_the_linked_headings` | without `## Git workflow`; without `## No backward compatibility` | error containing `AGENTS.md has no "## Git workflow" heading` (and the other) |
| `a_rules_file_without_paths` | `r.md` with no frontmatter | error containing `r.md has no paths: list` |
| `a_glob_that_matches_no_tracked_file` | `list` returns nothing for `nope/**` | error containing `r.md: "nope/**" matches no tracked file` |
| `a_glob_with_braces_or_a_bang` | `src/**/*.{ts,tsx}`; `!src/x` | error containing `uses braces or !` |
| `rule_paths_reads_quoted_and_bare_items` | `- "a/**"` and `- b/*` | `["a/**", "b/*"]` |
| `rule_paths_ignores_paths_after_the_frontmatter` | a `paths:` below the closing `---` | empty |
| `matching_rules_names_the_files_whose_globs_list_it` | two rules files, `list` answering per glob | the one that lists the file |

- [ ] **Step 3: Run them and see them fail**

Run: `cargo test -p xtask`
Expected: FAIL, panics at `not yet implemented`.

- [ ] **Step 4: Implement** `rule_paths`, `rules_files_errors`, `matching_rules` and `run`.

- [ ] **Step 5: Run them and see them pass**

Run: `cargo test -p xtask && cargo clippy -p xtask --all-targets -- -D warnings && cargo fmt --all --check`
Expected: all pass. `cargo xtask check-rules` itself still fails here (`WORKFLOW.md is missing`): the files come next.

- [ ] **Step 6: Commit** `build: add the xtask crate with a check of the agent rules files`, `Part of #356`.

### Task 2: `WORKFLOW.md`

**Files:**
- Create: `WORKFLOW.md`

- [ ] **Step 1: Write the file** from the spec's sections 2 and 5, with these headings in order and nothing else: `# Way of working`, `## Tiers` (the table as in the spec, with the superpowers skill names), `## The floor`, `## The repository is the memory`, `## Subagents` (the model and effort table; "the largest the project allows"), `## Scripts` (the four general lines of the spec's section 5), `## Housekeeping` (done means clean; nothing parked; the scratch folder `<temp>/agent-work/<repository>/<task>/`; no junction or symbolic link; what a leftover is and what an agent may delete on its own; "the project's tidy check, when it has one"), `## The person's machine`, `## Pull requests`. Last line, the order when sources disagree: the project's rules, this file, superpowers, ponytail.
- [ ] **Step 2: Check it**

Run: `git add WORKFLOW.md && git ls-files -s WORKFLOW.md && git cat-file -s :WORKFLOW.md && git grep -c -i -E "grace|karate|gpui|hpagniez" -- WORKFLOW.md; echo "grep exit $?"`
Expected: a size at most `5000`, and `grep exit 1` (no match).

- [ ] **Step 3: Commit** `docs: add the general way of working`, `Part of #356`.

### Task 3: the rules files and `docs/process/release.md`

**Files:**
- Create: `.claude/rules/electron.md`, `.claude/rules/gpui.md`, `.claude/rules/karate.md`, `.claude/rules/ci.md`, `.claude/rules/both-apps.md`, `docs/process/release.md`

Each rules file starts with its frontmatter, then a `#` heading, then the text moved word for word from `AGENTS.md` (relative links gain `../../`).

| File | `paths:` | Text moved from `AGENTS.md` |
|---|---|---|
| `electron.md` | `src/**`, `e2e/**`, `bench/**`, `test/**`, `*.config.*`, `package.json` | `## Stack` (intro and bullets), `### Solid 2 rules`, `### UI conventions`; from Commands, the rows `npm start`, `npm run screens:readme`, `npm run make` |
| `gpui.md` | `crates/grace/**`, `crates/grace-core/**`, `crates/grace-e2e/**`, `patches/**`, `Cargo.*`, `rust-toolchain.toml`, `scripts/e2e-gpui.sh`, `scripts/package-gpui.mjs`, `scripts/bump-cargo.mjs` | `### GPUI rules`, without the text that goes to `both-apps.md`; from Commands, the rows from `cargo run -p grace` to the clippy and fmt row |
| `karate.md` | `src/main/karate/**`, `src/main/runner*`, `crates/grace-core/**`, `listener/**`, `test/fixtures/**`, `scripts/capture-fixtures.mjs` | `## Karate integration rules` |
| `ci.md` | `.github/**`, `scripts/check-pr.mjs`, `scripts/bump-*.mjs`, `scripts/vendor.json`, `bench/limits.json`, `crates/xtask/**` | The workflow table; the SignPath bullet; the Dependabot paragraph; from Doc rules, the `gh issue edit` and `gh project` recipes; from Performance, the `perf` comment and `bench/limits.json` bullets; the Windows-dispatch and stacked-PR bullets |
| `both-apps.md` | `src/index.css`, `src/renderer/**`, `crates/grace/src/theme.rs`, `crates/grace/assets/**` | From GPUI rules: "Until the cut-over the tokens must equal `src/index.css`'s, which the theme's tests check: change both together", with the colour rule it belongs to, and the icons bullet. The colour rule's first sentence stays in `gpui.md` too |
| `docs/process/release.md` | none (not a rules file) | The two "Before a `release/*` or `hotfix/*` PR into `main`" bullets, the release-candidates bullet, the `release` row's detail, "Never write a release's `Decision:` line" |

Changed while moving, per the spec's verdict table: in `electron.md` and `gpui.md`, the e2e rule becomes "run the end-to-end tests of the app the PR changes. Small: the spec files of the touched area. Feature and Project: the whole suite once, before ready".

- [ ] **Step 1: Create the six files.**
- [ ] **Step 2: Commit** `docs: split the agent rules by the files they govern`, `Part of #356`. (Their globs are checked in Task 6, once `AGENTS.md` passes too.)

### Task 4: `AGENTS.md`, `CLAUDE.md` and the files that cite them

**Files:**
- Modify: `AGENTS.md`, `CLAUDE.md`, `scripts/bump-pr.mjs:64`, `.github/dependabot.yml:13`, `scripts/check-pr.mjs` (messages only), `scripts/check-pr.test.ts` and `scripts/bump-pr.test.ts` (only the strings they pin)

- [ ] **Step 1: Rewrite `AGENTS.md`** with these headings, in order:
  - `# AGENTS.md`: first paragraph "Follow [WORKFLOW.md](WORKFLOW.md), the general way of working; this file holds what is Grace's own", then what Grace is (today's intro).
  - `## Rules by area`: a five-row table of the rules files and what each covers, and `docs/process/release.md`. One line: Claude Code loads each with the files it governs; any other tool reads the one for the files it touches.
  - `## Commands`: `npm install` and `npm run vendor`; the rows `npm test`, `npm run e2e`, `npm run bench`, `npm run typecheck`, `npm run lint`, `cargo test --workspace`, `sh scripts/e2e-gpui.sh`, and a new row `cargo xtask <command>` ("Grace's scripts, in Rust: `check-rules`"); the "never run any two of" paragraph and the second-screen paragraph, whole (Review Focus 2).
  - `## Docs map`: as today, plus a `docs/process/` row.
  - `## Doc rules`: as today, with the verdict table's changes: an issue by tier (a Small docs, CI or chore PR needs none); Gherkin only for behaviour someone using the app sees; `## Cost` as one line "Cost: none" unless the feature adds a dependency, a process, a timer or work on the run path; decision records for a dependency, an architecture or a file format only; the state of long work in the tracking issue's body, edited in place. The `gh` recipes are in `ci.md`.
  - `## No backward compatibility`: as today.
  - `## Performance`: the priorities, the idle rule, the stack-change rule; one line pointing to `ci.md` and `docs/process/release.md` for the rest.
  - `## Git workflow`: the commit types and scopes; the branch table; merge with `--no-ff`; never commit to `main` or `develop`; what Grace adds to `WORKFLOW.md`'s pull requests: `develop` is the default target, `Closes #N` or `Part of #N`, assign to `hpagniez`, attribution, screenshots on `feature/pr-screenshots` (Feature and Project; Small only when the look changes), `merge-when-green` only on the owner's word, never delete `feature/pr-screenshots`.
  - `## Grace's own`: scripts are Rust, in `crates/xtask`, run as `cargo xtask <command>`; the toolchain's version is `rust-toolchain.toml`, its place `RUSTUP_HOME` and `CARGO_HOME`; existing JavaScript and shell scripts are ported when touched, and `bench/` stays JavaScript until the cut-over. The reference machine (the second screen, `GRACE_USER_DATA`, copy `vendor/`, what a check record must leave out). Subagent models `haiku`, `sonnet`, `opus`, never Fable. Superpowers paths `docs/specs/` and `docs/plans/`. The three plugins' jobs, one line each.
- [ ] **Step 2: Set `CLAUDE.md`** to exactly two lines: `@WORKFLOW.md`, `@AGENTS.md`.
- [ ] **Step 3: Update the citations**: `scripts/bump-pr.mjs:64` says `.claude/rules/karate.md` in place of `AGENTS.md (Karate integration rules)`; `.github/dependabot.yml:13` says `.claude/rules/electron.md`. The messages of `check-pr.mjs` that end in `(AGENTS.md)` for release records say `(docs/process/release.md)`; the one for a raised limit says `(.claude/rules/ci.md)`. Update the strings the two test files expect, if they pin them.
- [ ] **Step 4: Check the sizes and the tests**

Run: `git add -A && git cat-file -s :WORKFLOW.md && git cat-file -s :AGENTS.md && npx vitest run scripts/`
Expected: at most `5000`, at most `9000`; tests PASS. If `AGENTS.md` is over, move text to a rules file or `docs/process/` (Global Constraints) and run again.

- [ ] **Step 5: Commit** `docs: cut AGENTS.md down to what every task needs`, `Part of #356`.

### Task 5: nothing lost, and the settings

**Files:**
- Modify: `.claude/settings.json`

- [ ] **Step 1: List the lines of the old file that are in no new file.** For each line of `git show origin/develop:AGENTS.md`, stripped of its list or table markers and with runs of spaces and `../` link prefixes folded, longer than 40 characters: print it when the same folding of `WORKFLOW.md`, `AGENTS.md`, `docs/process/release.md` and `.claude/rules/*.md` joined does not contain it. A one-off command typed at the prompt (`git show`, `git grep -F -c`), not a file in the repository.
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
- Modify: `crates/xtask/src/rules.rs` (tests), `.github/workflows/checks.yml`

**Interfaces:**
- Consumes: `rules_files_errors`, `matching_rules` and `run` from Task 1.

- [ ] **Step 1: Write the repository tests** in `rules.rs`, reading the real repository through the same `git` calls as `run` (working directory: `CARGO_MANIFEST_DIR/../..`):
  - `the_repository_passes`: `rules_files_errors` returns no error.
  - `rules_load_by_area`: for each sample file, `matching_rules` returns exactly:

| Sample file | Rules files |
|---|---|
| `crates/grace/src/app.rs` | `gpui.md` |
| `crates/grace-core/src/karate/events.rs` | `gpui.md`, `karate.md` |
| `crates/grace/src/theme.rs` | `both-apps.md`, `gpui.md` |
| `crates/xtask/src/rules.rs` | `ci.md` |
| `src/renderer/App.tsx` | `both-apps.md`, `electron.md` |
| `src/index.css` | `both-apps.md`, `electron.md` |
| `src/main/karate/http.ts` | `electron.md`, `karate.md` |
| `.github/workflows/checks.yml` | `ci.md` |
| `docs/features.md` | none |

- [ ] **Step 2: Run them**: `git add -A && cargo test -p xtask`. Expected: PASS when Tasks 2 to 4 are right; fix the globs otherwise (a failure here is a defect in a rules file, not in the test).
- [ ] **Step 3: Add the CI step** to `checks.yml`, after the `cargo fmt --all --check` step and with the same `if`: `- run: cargo xtask check-rules`.
- [ ] **Step 4: Run it as CI does, from a clean build**

Run: `cargo clean -p xtask && cargo xtask check-rules; echo "exit $?"`
Expected: no `::error::` line, `exit 0`. Note the build time printed by `cargo build -p xtask --timings` or the wall time, for the PR body.

- [ ] **Step 5: Run the whole gate**

Run: `npm run lint && npm run typecheck && npm test && cargo test --workspace && cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: all pass.

- [ ] **Step 6: Commit** `ci: fail a pr whose agent rules files break their limits`, `Part of #356`.
- [ ] **Step 7: Open the PR** (draft, into `develop`, assigned to `hpagniez`, `Part of #356`, `Tier: project`). Its body holds: the byte counts of the files loaded at start, before (`AGENTS.md` 37,722 and `CLAUDE.md` 11) and after; `## Cut or changed` from Task 5; the build time of `xtask`; the test summaries; a request to the owner to open a fresh session and run `/context` and `/memory` once (plugins off, connectors off, rules loading by path), as a second reading and not a gate.
- [ ] **Step 8: Update #356's body**: its `Spec:` line points at the spec on `develop`; a "State" section says phase 1 is in review and phases 2 and 3 are next.

---

## Not in this phase

The `Tier:` line, the port of `scripts/check-pr.mjs` to `cargo xtask check-pr` with the new PR checks, the shorter PR template (phase 2); `cargo xtask tidy`, its `SessionStart` hook and the refusals in the settings (phase 3); skipping `perf` on `crates/`-only PRs (a follow-up of its own, on the owner's word of 2026-11-12).
