# Way of working, phase 2: the PR checks in Rust. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `checks` fails a PR that does not follow the way of working: no tier, a Small PR that is not small, a Feature or Project PR without its papers, behaviour without a test, a script in another language without a reason, an agent's session link.

**Architecture:** A second command in `crates/xtask`, `check-pr`, in its own module of pure functions. It reads the PR body from the environment and the diff from git. The checks of `scripts/check-pr.mjs` that need nothing from the bench (commit headers, branch target, a release record's `## Public docs`) are ported to it and leave the JavaScript file. The two that judge the bench's JSON stay in JavaScript until the bench itself is ported (see Scope).

**Tech Stack:** Rust 1.99.1, standard library only; `git` with an argument list; GitHub Actions (`checks.yml`).

**Spec:** [docs/specs/2026-11-12-way-of-working.md](../specs/2026-11-12-way-of-working.md), section 6, mechanism 1.

## Scope: one change to the spec

The spec says `scripts/check-pr.mjs` is ported whole and deleted. Two of its five checks, `limitRaiseError` and `performanceError`, call `judge` and `raisedLimits` of `bench/judge.mjs` on the bench's JSON. The bench stays JavaScript until the cut-over (spec, section 5). Porting those two now would mean a second copy of the judging rules in Rust, to be kept equal to the first by hand. So they stay where they are, in a `scripts/check-pr.mjs` cut down to them, and go to Rust with the bench. `checks` runs both commands. Acceptance check 8 of the spec is met for the three checks that move and for every new one.

## Global Constraints

- Tier: Project. One PR, `Part of #356`, on `feature/way-of-working-checks` from `develop`.
- `crates/xtask` keeps `#![forbid(unsafe_code)]` and no dependency. No regular-expression crate: the patterns are small enough to parse by hand, and each parser has its tests.
- Every message starts as the JavaScript one did where a check is ported, so nothing an agent learnt changes.
- A PR from a bot branch (`dependabot/*`, `feature/bump-*`) or a release branch (`release/*`, `hotfix/*`) is checked for commit headers, branch target, agent traces and script language only: it has no tier.
- A run that is not a `pull_request` event (`workflow_dispatch`) has no PR body: the checks that read the body are skipped.
- Exact values: the tier line is `Tier: small`, `Tier: feature` or `Tier: project`, alone on its line; the Small limit is 400 changed lines; the script extensions are `.py`, `.mjs`, `.js`, `.sh`, `.ps1`; the folders they may live in are `src/`, `e2e/`, `test/`, `bench/` and `docs/`.
- No new JavaScript, Python or shell file. Commits: Conventional Commits, `Part of #356` in each.

## Review Focus

1. **A check that fails a PR it should pass**, which blocks the owner's work: each check's tests include the ordinary PRs of this repository (a docs-only Small PR, a Dependabot PR, a bump PR, a release PR into `main`, a spec-only PR). Task 2.
2. **A Rust test that proves nothing**: a `fix` whose only test change is in the same `.rs` file passes the "tests come with behaviour" check through an added `#[test]` or `assert` line; a `fix` with none fails. Task 3.
3. **A body edited after a red run**: `checks.yml` runs on `edited`, so adding the `Tier:` line reruns the check. Task 5, tried on the PR itself.
4. **A dependency added in a way the scanner misses**: a renamed table (`[dependencies.foo]`), a target table, a workspace table. Task 3.
5. **The machine's git settings leaking into tests**: every temporary repository a test makes sets `commit.gpgsign=false`. Tasks 4 and 5.

---

## File structure

| File | Responsibility |
|---|---|
| `crates/xtask/src/pr.rs` (create) | `check-pr`: the pure checks, their tests, and `run` |
| `crates/xtask/src/main.rs` (modify) | Dispatches `check-pr`; `USAGE` names it |
| `scripts/check-pr.mjs`, `scripts/check-pr.test.ts` (modify) | Keep `limitRaiseError`, `performanceError` and their tests; lose the three ported checks and theirs |
| `.github/workflows/checks.yml` (modify) | Runs `cargo xtask check-pr` with the PR body; also on `edited` |
| `.github/pull_request_template.md` (rewrite) | Summary, `Tier:`, the issue line, `Spec:` and `Plan:` |
| `.claude/rules/ci.md`, `AGENTS.md` (modify) | The `checks` row and the `cargo xtask` row name the new command |

---

### Task 1: the three ported checks

**Files:** Create `crates/xtask/src/pr.rs`; modify `crates/xtask/src/main.rs`.

**Interfaces (produces):**
- `pub fn commit_error(header: &str) -> Option<String>`
- `pub fn branch_error(head: &str, base: &str) -> Option<String>`
- `pub fn versioned(head: &str) -> bool`: `release/vX.Y.Z-rc.N` or `hotfix/vX.Y.Z-rc.N`.
- `pub fn public_docs_error(head: &str, base: &str, files: &[String], read: &dyn Fn(&str) -> Option<String>) -> Option<String>`

- [ ] **Step 1: Port the tests** of `describe("commitError")`, `describe("branchError")` and `describe("publicDocsError")` of `scripts/check-pr.test.ts` into `pr.rs`, one Rust test per `it`, each `it.each` row an element of a loop, with the same inputs and the same expected text. The signatures above have `todo!()` bodies.
- [ ] **Step 2:** `cargo test -p xtask pr::` fails at `not yet implemented`.
- [ ] **Step 3: Implement** the three, with the JavaScript messages word for word.
- [ ] **Step 4:** `cargo test -p xtask` passes.
- [ ] **Step 5: Commit** `ci: port the commit, branch and public-docs checks to rust`.

### Task 2: tier, papers, traces, script language

**Files:** Modify `crates/xtask/src/pr.rs`.

**Interfaces (produces):**
- `pub enum Tier { Small, Feature, Project }`
- `pub fn tier(body: &str) -> Option<Tier>`
- `pub fn exempt(head: &str) -> bool`: a bot or release branch.
- `pub fn tier_error(head: &str, body: &str) -> Option<String>`
- `pub fn papers_errors(tier: Tier, body: &str, docs_only: bool, exists: &dyn Fn(&str) -> bool) -> Vec<String>`
- `pub fn trace_errors(texts: &[String]) -> Vec<String>`: the body and every commit message.
- `pub fn script_language_error(added: &[String], body: &str) -> Option<String>`

- [ ] **Step 1: Write the failing tests.**

| Test | Input | Expected |
|---|---|---|
| `tier_reads_its_line` | `"x\nTier: feature\ny"`; `"Tier: Small"`; `"Tier: big"`; `"The Tier: small one"` | `Feature`; none; none; none |
| `tier_error_names_the_line` | `feature/x`, body without it | error containing `the PR body has no "Tier: small", "Tier: feature" or "Tier: project" line` |
| `bot_and_release_branches_need_no_tier` | `dependabot/npm_and_yarn/x`, `feature/bump-cargo-2026-11-17`, `feature/bump-karate-2.1.5`, `release/v1.0.0-rc.10`, `hotfix/v1.0.0-rc.11`, empty body | none |
| `feature_bump_is_not_every_feature` | `feature/bumper`, empty body | an error |
| `a_small_pr_needs_no_papers` | `Small`, empty body | no error |
| `a_feature_needs_its_issue` | `Feature`, body with `Spec:` and `Plan:` but no issue | error containing `a feature PR links its issue: "Closes #N" or "Part of #N"` |
| `closes_and_part_of_both_count` | `Closes #12`; `Part of #356` | no error |
| `a_feature_needs_its_spec_and_plan` | `Feature`, issue line only | two errors: `no "Spec: <path>" line`, `no "Plan: <path>" line` |
| `the_named_files_must_exist` | `Spec: docs/specs/nope.md`, `exists` false | error containing `Spec: docs/specs/nope.md is not a file of this PR's head` |
| `a_spec_only_pr_needs_no_plan` | `Project`, issue, `Spec:` that exists, `docs_only` true | no error |
| `traces_are_refused` | `https://claude.ai/code/session_01`, `Claude-Session: abc`, `Requested by: x`, each alone | one error each, containing `says more about the agent than its attribution lines` |
| `the_attribution_lines_pass` | the two lines of `.claude/settings.json` | no error |
| `a_new_script_needs_its_reason` | added `scripts/x.mjs`, body without `Script-language:` | error containing `scripts/x.mjs: a new script is Rust (cargo xtask) unless the body says why on a "Script-language:" line` |
| `a_reason_lets_it_in` | the same with `Script-language: runs inside Playwright` | none |
| `app_code_and_docs_are_not_scripts` | added `src/main/a.js`, `e2e/b.mjs`, `bench/c.mjs`, `test/d.js`, `docs/research/x/e.py`, `crates/x/f.rs` | none |
| `every_script_extension_counts` | `a.py`, `tools/b.js`, `c.sh`, `d.ps1`, each alone | an error each |

- [ ] **Step 2:** the tests fail at `not yet implemented`.
- [ ] **Step 3: Implement.**
- [ ] **Step 4:** `cargo test -p xtask` passes.
- [ ] **Step 5: Commit** `ci: check a pr's tier, papers, traces and script language`.

### Task 3: Small stays small, and tests come with behaviour

**Files:** Modify `crates/xtask/src/pr.rs`.

**Interfaces (produces):**
- `pub fn dependencies(path: &str, text: &str) -> Vec<String>`: the dependency names of a `package.json` (the keys of `dependencies`, `devDependencies`, `optionalDependencies`, `peerDependencies`) or of a `Cargo.toml` (the keys of every table whose name is or ends with `dependencies`, and the `X` of every `[...dependencies.X]` table).
- `pub fn counted(path: &str) -> bool`: false for `docs/**`, `*.md`, `Cargo.lock`, `package-lock.json`, `test/fixtures/**`.
- `pub struct Change { pub path: String, pub lines: usize, pub added: bool }`
- `pub fn small_errors(changes: &[Change], new_dependencies: &[String]) -> Vec<String>`
- `pub fn is_test_path(path: &str) -> bool`: `*.test.ts`, `*.test.tsx`, `*.spec.ts`, `e2e/**`, `crates/*/tests/**`.
- `pub fn tests_error(headers: &[String], changes: &[Change], added_rust_lines: &[String]) -> Option<String>`

- [ ] **Step 1: Write the failing tests.**

| Test | Input | Expected |
|---|---|---|
| `package_json_dependencies` | a `package.json` with all four tables and a `scripts` table | the names of the four, none of `scripts` |
| `cargo_dependencies` | `[dependencies]`, `[dev-dependencies]`, `[workspace.dependencies]`, `[target.'cfg(windows)'.dependencies]`, `[dependencies.foo]`, and `[package]` with `name = "x"` | every dependency name and `foo`; not `name` |
| `a_small_pr_within_its_limits` | 400 counted lines, no new file under `docs/decisions/`, no new dependency | no error |
| `a_small_pr_over_400_lines` | 401 counted lines | error containing `a Small PR changes 401 lines outside docs, lock files and fixtures, over 400: step up a tier` |
| `docs_lock_files_and_fixtures_are_not_counted` | 5,000 lines in `docs/x.md`, `Cargo.lock`, `package-lock.json`, `test/fixtures/a.jsonl`, `README.md` | no error |
| `a_small_pr_adds_a_dependency` | `["serde_json"]` | error containing `a Small PR adds a dependency (serde_json): step up a tier` |
| `a_small_pr_adds_a_decision_record` | an added `docs/decisions/0034-x.md` | error containing `a Small PR adds a decision record: step up a tier` |
| `a_fix_without_a_test` | header `fix(runner): x`, change in `src/main/a.ts` | error containing `a feat or fix commit changes src/ or crates/ and no test` |
| `a_fix_with_its_test_file` | the same and `src/main/a.test.ts` | none |
| `a_rust_fix_with_a_test_in_the_same_file` | `fix: x`, `crates/grace/src/a.rs`, an added line `    #[test]`; and with `        assert_eq!(a, b);`; and with `    #[gpui::test]` | none, each |
| `a_rust_fix_with_neither` | the same, added lines without those | an error |
| `docs_and_chores_need_no_test` | `docs: x` or `refactor: x` with a change in `src/` | none |
| `a_feat_that_touches_no_code` | `feat: x`, a change in `docs/` only | none |

- [ ] **Step 2:** fail. **Step 3: Implement**, with one comment: `// shortcut: a Rust test is seen by an added #[test] or assert line, since unit tests share their file; a fix that only reorders a test passes unseen, upgrade if that is ever abused`. **Step 4:** pass.
- [ ] **Step 5: Commit** `ci: keep a small pr small and ask a fix for its test`.

### Task 4: `run`, on a real repository

**Files:** Modify `crates/xtask/src/pr.rs`, `crates/xtask/src/main.rs`.

**Interfaces:**
- Consumes: every function of Tasks 1 to 3.
- Produces: `pub fn errors(env: &Env, git: &dyn Fn(&[&str]) -> Option<String>) -> Vec<String>`, with `pub struct Env { pub head_ref: String, pub base_ref: String, pub base_sha: String, pub head_sha: String, pub body: Option<String> }` (`body` is `None` outside a `pull_request` event); `pub fn run() -> u8`, which reads `HEAD_REF`, `BASE_REF`, `BASE_SHA`, `HEAD_SHA`, `EVENT_NAME` and `PR_BODY`, prints each error as `::error::`, and returns 1 when there is one.
- Git calls, as the JavaScript makes them: the fork point is `merge-base BASE_SHA HEAD_SHA`; commit messages are `log --no-merges --format=%B%x00 HEAD_SHA --not BASE_SHA origin/develop`, split on the NUL, the first line of each its header; the changes are `diff --numstat` and `diff --name-only --diff-filter=A` between the fork point and `HEAD_SHA`; the added Rust lines are the `+` lines of `diff -U0 fork HEAD_SHA -- '*.rs'`; a file's text is `show <sha>:<path>`.

- [ ] **Step 1: Write the failing tests**, each in a temporary repository made by a helper that runs `git -c user.name=t -c user.email=t@t -c commit.gpgsign=false`: port the six tests of `describe("check-pr CLI")` that concern the three ported checks (a bad header is reported, a merge commit is not, a release into `main` without its record fails), and add: `a_small_docs_pr_passes`, `a_pr_without_a_tier_fails`, `a_dispatch_run_skips_the_body`, `a_dependabot_pr_passes`.
- [ ] **Step 2:** fail. **Step 3: Implement** `errors` and `run`, and the `check-pr` arm of `main`. **Step 4:** `cargo test -p xtask`, `cargo clippy -p xtask --all-targets -- -D warnings` and `cargo fmt --all --check` pass.
- [ ] **Step 5: Commit** `ci: run the pr checks from cargo xtask check-pr`.

### Task 5: the JavaScript that is left, CI, the template, the rules

**Files:** Modify `scripts/check-pr.mjs`, `scripts/check-pr.test.ts`, `.github/workflows/checks.yml`, `.github/pull_request_template.md`, `.claude/rules/ci.md`, `AGENTS.md`.

- [ ] **Step 1: Cut `scripts/check-pr.mjs`** to `limitRaiseError`, `performanceError` and a `main` that runs those two. Its header comment says what it is and that it goes to Rust with the bench. Remove from `scripts/check-pr.test.ts` the tests of the three ported checks and of the CLI's ported cases; in the helper that makes the temporary repository, add `-c commit.gpgsign=false`.
- [ ] **Step 2:** `npx vitest run scripts/check-pr.test.ts` passes on this machine, with no change to its git settings (it failed before on commit signing).
- [ ] **Step 3: `checks.yml`:** add `edited` to the `pull_request` types; after `cargo xtask check-rules`, a step `cargo xtask check-pr` with the `if` and the `env` of the `node scripts/check-pr.mjs` step plus `EVENT_NAME: ${{ github.event_name }}` and `PR_BODY: ${{ github.event.pull_request.body }}`. The `node scripts/check-pr.mjs` step stays.
- [ ] **Step 4: Rewrite `.github/pull_request_template.md`** as:

```markdown
<!-- Title: Conventional Commits. Rules: WORKFLOW.md and AGENTS.md. -->

## Summary

<!-- What changes and why, in 1-3 sentences. -->

Tier: small
<!-- small | feature | project (WORKFLOW.md). A feature or project PR also fills the three lines below; delete them for a small one. -->
Closes #
Spec: docs/specs/
Plan: docs/plans/

<!-- A new script that is not Rust says why: "Script-language: <reason>". -->
```

- [ ] **Step 5: The rules:** in `.claude/rules/ci.md`, the `checks` row names `cargo xtask check-rules`, `cargo xtask check-pr` (the tier and what follows from it, tests with behaviour, script language, agent traces, commit headers, branch target, a release record's `## Public docs`) and `scripts/check-pr.mjs` (a release record's `## Performance`, a raised limit). In `AGENTS.md`, the `cargo xtask` row names `check-pr`. `cargo xtask check-rules` still passes.
- [ ] **Step 6: The gate:** `npm run lint && npm run typecheck && npm test && cargo test --workspace && cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [ ] **Step 7: Commit** `ci: run cargo xtask check-pr in checks and shorten the pr template`.
- [ ] **Step 8: Open the PR** with the new template. Its own `checks` run is the proof: it passes with its `Tier: project`, `Part of #356`, `Spec:` and `Plan:` lines. Then, on the PR itself, remove the `Tier:` line from the body and see `checks` rerun and fail with the tier message; put it back and see it pass (Review Focus 3, acceptance check 8). Record both runs in the body.
- [ ] **Step 9: Update #356's body:** phase 1 merged, phase 2 in review, phase 3 next.

---

## Not in this phase

`cargo xtask tidy`, its hook, the refusals in the settings (phase 3). Porting `limitRaiseError`, `performanceError` and `bench/` (the cut-over).
