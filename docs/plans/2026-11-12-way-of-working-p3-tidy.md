# Way of working, phase 3: the tidy report and the refusals. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Leftovers show at the start of the next session, and the commands that caused harm are refused.

**Architecture:** A command `cargo xtask tidy` in `crates/xtask/src/tidy.rs`: one pure function from facts (branches, worktrees, stashes, untracked files, links, scratch folders, GitHub's branches and PRs) to the list of leftovers, and a `run` that gathers the facts from `git`, the file system and `gh`, prints the list and always exits 0. A `SessionStart` hook in `.claude/settings.json` runs it. The refusals are `permissions.deny` rules in the same file, with no script behind them.

**Tech Stack:** Rust 1.99.1, standard library only; `git` and `gh` started with an argument list; Claude Code settings (`hooks`, `permissions.deny`).

**Spec:** [docs/specs/2026-11-12-way-of-working.md](../specs/2026-11-12-way-of-working.md), section 2 (Housekeeping) and section 6, mechanisms 2 and 3.

## Global Constraints

- Tier: Project. One PR, `Part of #356`, on `feature/way-of-working-tidy` from `develop`.
- `tidy` changes nothing, prints nothing when all is clean, and exits 0 whatever happens: a missing `gh`, no network, a repository it cannot read.
- The hook is silent and harmless where `cargo` is missing or the crate does not build: `cargo xtask tidy 2>/dev/null || true`, run by bash, with a timeout of 60 seconds.
- A leftover is something with no open PR behind it. Never reported: the default branch and `main`; the current branch unless it is merged and its remote branch is gone; a branch, a worktree or a scratch folder whose branch has an open PR; `feature/pr-screenshots` on GitHub; a bot's PR and a release or hotfix PR, however long they wait.
- Exact values: a PR is idle after 7 days; a branch without a PR is reported once its last commit is a day old; the scratch folder is `<temp>/agent-work/<repository folder name>/<task>/`; the link search goes 3 levels deep into untracked and ignored folders and skips `node_modules`, `vendor`, `out`, `target` and `.git`.
- Deny rules are a guardrail, not a wall: Claude Code matches the command as written, so the same program started another way passes (its documentation says so). The rules files keep the rule itself.
- `crates/xtask` keeps `#![forbid(unsafe_code)]` and no dependency. No new JavaScript, Python or shell file.

## Review Focus

1. **Noise.** A report that cries at ordinary work is ignored within a week: a branch just cut, a PR in review, Dependabot's branches, another session's worktree with an open PR. Task 1's table names each.
2. **A link that is missed.** The junction found on 2026-11-12 sat two levels deep in an untracked folder. Task 2 makes one and finds it, on Windows as a junction, elsewhere as a symbolic link.
3. **A hook that slows or breaks the start of a session.** Task 3 times the command and runs the hook's exact line with `cargo` off the path.
4. **A deny rule that blocks ordinary work**, such as reading a file that mentions `mklink`. Task 3 tries both a refused and an allowed command.
5. **Offline.** No network: the local leftovers still show, GitHub's are left out, nothing fails. Task 2.

---

### Task 1: `leftovers`, the pure rule

**Files:** Create `crates/xtask/src/tidy.rs`; modify `crates/xtask/src/main.rs`.

**Interfaces (produces):**
- `pub struct Branch { pub name: String, pub merged: bool, pub gone: bool, pub age_days: u64 }`
- `pub struct Pr { pub number: u64, pub branch: String, pub idle_days: u64, pub bot: bool }`
- `pub struct Remote { pub branches: Vec<String>, pub prs: Vec<Pr> }`
- `pub struct Facts { pub default: String, pub current: String, pub branches: Vec<Branch>, pub worktrees: Vec<(String, String)>, pub stashes: usize, pub untracked: Vec<String>, pub links: Vec<String>, pub scratch: Vec<String>, pub remote: Option<Remote> }` (`worktrees` are path and branch, without the main checkout; `scratch` are the names of the task folders that are not empty; `remote` is `None` when GitHub cannot be read)
- `pub fn leftovers(facts: &Facts) -> Vec<String>`

- [ ] **Step 1: Write the failing tests**, each from a clean `Facts` (on `develop`, nothing else) changed in one way:

| Test | Change | Expected |
|---|---|---|
| `a_clean_checkout_has_none` | none; and with a remote holding `main`, `develop`, `feature/pr-screenshots` | empty |
| `a_merged_branch` | branch `feature/x`, merged | one line containing `branch feature/x is merged into develop: git branch -d feature/x` |
| `a_branch_whose_remote_is_gone` | not merged, gone | `branch feature/x lost its remote branch` |
| `a_branch_just_cut_is_work_in_progress` | current branch `feature/x`, merged (no commit yet), not gone | empty |
| `a_stale_current_branch` | current branch merged and gone | the merged line |
| `a_branch_without_a_pr` | age 2 days, remote known with no PR for it | `branch feature/x has no open PR after 2 days` |
| `a_young_branch_or_one_with_a_pr` | age 0; or age 9 with an open PR | empty |
| `without_github_a_branch_is_not_judged` | age 9, `remote: None` | empty |
| `a_worktree` | `("/tmp/w", "feature/x")`, remote known, no PR | `worktree /tmp/w (feature/x) has no open PR`; with a PR, empty; with `remote: None`, empty |
| `a_stash` | 2 | `2 stashes: git stash list` |
| `untracked_files_on_the_default_branch` | `a.txt`, `b.txt` on `develop` | `untracked files on develop: a.txt, b.txt`; on `feature/x`, empty; with 7 files, the first 5 and `(+2 more)` |
| `a_link_inside_the_checkout` | `before/before/vendor` | `link inside the checkout: before/before/vendor` and `never a recursive delete` |
| `a_scratch_folder` | `old-task`, current branch `feature/new-task` | `scratch folder old-task is not empty`; `new-task` is the current task: empty |
| `a_remote_branch_without_a_pr` | remote branch `feature/parked`, no PR | `remote branch feature/parked has no open PR`; `dependabot/npm/x` with its PR: empty |
| `an_idle_pr` | PR 12 idle 7 days | `PR #12 (feature/x) has not moved for 7 days`; idle 6: empty; a bot's, or `release/v1.0.0-rc.10`, idle 30: empty |

- [ ] **Step 2:** `cargo test -p xtask tidy::` fails at `not yet implemented`. **Step 3: Implement.** **Step 4:** it passes.
- [ ] **Step 5: Commit** `build: add the rule that names a repository's leftovers`.

### Task 2: the facts, and `run`

**Files:** Modify `crates/xtask/src/tidy.rs`, `crates/xtask/src/main.rs`.

**Interfaces:**
- Consumes: `Facts`, `leftovers`.
- Produces: `pub fn local_facts(root: &Path, scratch: &Path, now: u64, git: &dyn Fn(&[&str]) -> Option<String>) -> Facts` (with `remote: None`); `pub fn remote(git: &dyn Fn(&[&str]) -> Option<String>, gh: &dyn Fn(&[&str]) -> Option<String>) -> Option<Remote>`; `pub fn run() -> u8`, which always returns 0.
- Commands: the default branch is `git symbolic-ref --short refs/remotes/origin/HEAD` without its `origin/`, `develop` when that fails; branches are `git for-each-ref refs/heads --format=%(refname:short)%09%(upstream:track)%09%(committerdate:unix)` and `git branch --merged origin/<default> --format=%(refname:short)`; worktrees `git worktree list --porcelain`; stashes `git stash list`; untracked and ignored paths `git status --porcelain --ignored`; GitHub's branches `git ls-remote --heads origin`; its PRs `gh pr list --state open --limit 200 --json number,headRefName,updatedAt,author --jq` with a filter that prints number, branch, whole days since the update and whether the author is a bot, tab-separated.

- [ ] **Step 1: Write the failing tests**, on a temporary repository made as in `pr.rs` (no global or system git configuration): `a_fresh_repository_is_clean`; `it_finds_what_was_left` (a merged branch, a second worktree, a stash, an untracked file on the default branch, a non-empty scratch folder, and a link two levels deep in an untracked folder: a junction made with `cmd /c mklink /J` on Windows, a symbolic link elsewhere; each named in `leftovers`, the worktree only once a `Remote` with no PR is put in); `a_gone_upstream_is_seen`; `remote_reads_branches_and_prs` (with `git` and `gh` closures returning canned text); `remote_is_none_when_gh_fails`.
- [ ] **Step 2:** fail. **Step 3: Implement**, and the `tidy` arm of `main` with its `USAGE` line. `run` prints, when there is anything, one heading line and one line per leftover. **Step 4:** `cargo test -p xtask`, clippy on the crate and `cargo fmt --all --check` pass.
- [ ] **Step 5: Commit** `build: report leftovers with cargo xtask tidy`.

### Task 3: the hook, the refusals, the rules

**Files:** Modify `.claude/settings.json`, `AGENTS.md`, `.claude/rules/ci.md`, `docs/specs/2026-11-12-way-of-working.md` (its status line).

- [ ] **Step 1: `.claude/settings.json`** gains:

```json
"hooks": {
  "SessionStart": [
    {
      "matcher": "startup|resume",
      "hooks": [
        { "type": "command", "command": "cd \"$CLAUDE_PROJECT_DIR\" && cargo xtask tidy 2>/dev/null || true", "shell": "bash", "timeout": 60 }
      ]
    }
  ]
},
"permissions": {
  "deny": [
    "Bash(mklink *)", "Bash(cmd /c mklink *)", "Bash(cmd //c mklink *)", "Bash(cmd.exe /c mklink *)", "Bash(ln -s*)",
    "PowerShell(cmd /c mklink *)", "PowerShell(New-Item *Junction*)", "PowerShell(New-Item *SymbolicLink*)",
    "Bash(git push --force*)", "Bash(git push -f *)", "Bash(git push * --force*)", "Bash(git push * -f *)",
    "PowerShell(git push --force*)", "PowerShell(git push -f *)", "PowerShell(git push * --force*)", "PowerShell(git push * -f *)"
  ]
}
```

- [ ] **Step 2: Try it.** Run the hook's line as written: no output on the clean checkout. Run it with `cargo` off the path (`PATH=/usr/bin:/bin`): no output, exit 0. Time `cargo xtask tidy`. Make a stray branch, run it, see the line, delete the branch. Ask Claude Code to run `cmd /c mklink /J <scratch>\a <scratch>\b`: refused. Run `git grep -c mklink -- WORKFLOW.md`: allowed. Each result goes in the PR body.
- [ ] **Step 3: The rules.** `AGENTS.md`'s `cargo xtask` row names `tidy` (and stays under 9,000 bytes); its Grace's own section says the session-start hook runs it. `.claude/rules/ci.md`'s script-crate paragraph names it. The spec's status line says the three phases are built.
- [ ] **Step 4: The gate:** `cargo test -p xtask`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo fmt --all --check`, `cargo xtask check-rules`, `npm run lint`.
- [ ] **Step 5: Commit** `build: run the tidy report at session start and refuse links and force pushes`.
- [ ] **Step 6: Open the PR** (`Tier: project`, `Part of #356`, `Spec:`, `Plan:`), then update #356's body: phase 3 in review.

---

## Not in this plan

Deleting anything: `tidy` only reports. A weekly workflow: the report at session start reads GitHub too (spec, "Kept simple on purpose").
