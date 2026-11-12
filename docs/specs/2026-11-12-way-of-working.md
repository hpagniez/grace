# Way of working: tiers, lean context, housekeeping

Date 2026-11-12. Status: approved by the owner on 2026-11-12. Built in three phases; plan of phase 1: [way-of-working-p1-files](../plans/2026-11-12-way-of-working-p1-files.md).

## Why

Grace is built mostly by AI agents. In 19 days the repository took 198 pull requests, 155 issues, 33 decision records, 45 specs with 45 plans and 31 check records, steered by a 229-line `AGENTS.md`. The owner's cost is **time and Claude Code licence usage**. He wants both lower without losing code quality or the documentation agents work from. He also wants one way of working he can reuse in any project, built on the superpowers workflow, with Grace's own rules as a thin layer, and stronger housekeeping: on 2026-11-12 the checkout held stale branches, 4 GB of old build output and two junctions into its own `vendor/`.

## Goals

1. Less licence usage per session and per subagent, before any work starts.
2. Less process for small changes; the full process only where it pays.
3. The same quality floor at every size of change.
4. One general file, with nothing about Grace in it, that another project can copy as it is.
5. A clean repository after every task, checked by a script, not by memory.

## Non-goals

- Changing the branch model (git flow, release candidates), the CI workflows' jobs or the updater's contract.
- Rewriting accepted decision records, old specs, plans or check records.
- Changing how the GPUI port is done: its guide, parity list and phase plans stay as they are.

## Anti-goals

- A second rule book beside the first. The total text an agent loads must go down.
- Saving tokens by dropping tests, verification or the records the port's cut-over rests on.
- A process that needs a plugin to work. Cloud sessions load no user-level plugin (see Findings).

## Findings

Measured or read in the documentation on 2026-11-12.

| Finding | Source |
|---|---|
| `AGENTS.md` is 34 KB and loads in full in every session. Pull requests and CI take 8.7 KB, GPUI rules 6.2 KB, commands 3.3 KB, Karate 3.1 KB, branches 3.1 KB, Solid and UI rules 2.2 KB. About half matters only when one part of the code is touched | byte count per section |
| A general-purpose subagent loads the whole of `CLAUDE.md` and `AGENTS.md` again, with a cold cache. The built-in Explore and Plan agents skip them | Claude Code docs, sub-agents |
| `.claude/rules/*.md` with a `paths:` list loads only when a matching file is read or edited, in the session and in subagents. Rules without `paths` load at start | Claude Code docs, memory |
| A project's `.claude/settings.json` can turn off a plugin enabled at user level (`"enabledPlugins": {"name@marketplace": false}`), and claude.ai connectors (`"disableClaudeAiConnectors": true`) | Claude Code docs, settings |
| Cloud sessions load the repository's `CLAUDE.md`, rules, skills, agents and hooks. They load no user-level `CLAUDE.md` and no plugin, even one the repository declares | Claude Code docs, cloud environments |
| `AGENTS.md` sets every subagent to extra-high effort, whatever the model. One documentation lookup run that way on 2026-11-12 used 223,000 tokens | `AGENTS.md`; that run's usage |
| Ten plugins are on at user level. Five have nothing to do with Grace (Vercel, Supabase, Firecrawl, taste-skill, impeccable). Their skills are listed in every session | `~/.claude/settings.json` |
| claude-mem's recall paid for itself on 2026-11-12: it told the session that the stopped VL-29 run had been cleaned up, and the three traps that run had hit | this session |
| Three to-do lists compete (the app's, claude-mem's work state, superpowers' checklists), and three memories (claude-mem, Claude Code's memory folder, hand-over notes in issues) | session start text |
| Rules given in conversation were not all written down: the resume note on #278 lists eleven, of which seven were missing from `AGENTS.md` | #353 |
| Superpowers 7 already sizes the work: its design step has "a small change" and "a project", and its plan step offers building in the session with one review at the end, or a fresh agent and a reviewer per task | superpowers `brainstorming`, `writing-plans` |
| `perf` takes about 20 minutes of a Windows runner on every code PR and measures Electron Grace only; on a PR that changes only `crates/` its comment says nothing | `perf.yml`, #351's body |

## Approaches

| | Approach | Trade-off |
|---|---|---|
| A | **Lean context only.** Keep every process step; split `AGENTS.md` by path, turn off unrelated plugins, set subagent effort by task | Lowest risk. Saves usage, little time |
| B | **Tiers plus lean context.** A, and the amount of process follows the size of the change | Saves time and usage. Needs a judgement of size at the start of each task |
| C | **Stock superpowers only.** Drop the issue per PR, decision records, check records and plans in the repository | Fastest. Loses the documentation the owner wants to keep |

The owner picked **B** on 2026-11-12. C was set aside because it removes documentation; A alone because it saves no time.

## Design

### 1. Two layers

| File | Holds | Loaded |
|---|---|---|
| `WORKFLOW.md` (new, repository root) | The general way of working. Nothing about Grace. Another project copies the file as it is | Always: `CLAUDE.md` imports it |
| `AGENTS.md` | Grace only: what the app is, commands, docs map, the rules that apply whatever is touched | Always |
| `.claude/rules/<area>.md` (new) | Grace rules for one part of the code, each with a `paths:` list | Only when a matching file is read or edited |
| `docs/process/release.md` (new) | The release and hotfix steps, moved out of `AGENTS.md` word for word | When an agent works on a release: `AGENTS.md` links it |

`CLAUDE.md` becomes two lines: `@WORKFLOW.md` and `@AGENTS.md`. `AGENTS.md` starts with "Follow `WORKFLOW.md`", for tools that read only `AGENTS.md`.

Size limits, in bytes (lines mean little when one bullet runs to 1 KB): `WORKFLOW.md` at most 5 KB, `AGENTS.md` at most 9 KB, 14 KB together (from 34 KB). When the text does not fit, more of it moves to a rules file or to `docs/process/`: the commands of one app go with that app's rules, the `gh project` recipe with the CI rules. A rule is never cut to fit; cuts are only those of the verdict table below.

### 2. The general way (`WORKFLOW.md`)

It states each step in its own words and names the superpowers skill that carries it out where that plugin is installed. Without the plugin the steps still stand.

**Tiers.** The agent names the tier in its first message of a task; the owner can overrule it. When a task outgrows its tier, stop, say so, and step up.

| | Small | Feature | Project |
|---|---|---|---|
| When | A fix, a doc or config change, a dependency bump, a refactor that changes no behaviour. The behaviour wanted is clear, and nothing hard to reverse is decided | New behaviour someone using the app sees, or a change across several parts | Several PRs, or a hard-to-reverse choice (dependency, architecture, file format), or work no one can check from one diff |
| Design | Two or three lines in the PR body: what and why | A short design file, approved by the owner (`brainstorming`, small change) | The full design, read by a reviewer agent first, approved by the owner (`brainstorming`, project); a decision record for each hard-to-reverse choice |
| Plan | None | A plan file (`writing-plans`) | A plan file per phase |
| Build | In the session | In the session | A fresh agent per task where tasks are independent (`subagent-driven-development`), else in the session |
| Review | The session's own check | One fresh reviewer on the whole branch (`requesting-code-review`) | A reviewer per task, then one on the whole branch |
| Issue | Only for a bug someone reported, or when the PR does not finish the work | One | One to track, one per phase |

**The floor, at every tier.**

- Test first for any change in behaviour: the failing test, then the code (`test-driven-development`). A doc, config or mechanical change needs none.
- A bug gets its cause found before its fix (`systematic-debugging`).
- Nothing is called done, fixed or passing without the command's output in the same message (`verification-before-completion`).
- One branch, one PR, one logical change per commit.

**The repository is the memory.** A decision goes in a decision record, a rule in the rules files, the state of long work in its tracking issue. A rule the owner gives in conversation is written into the rules files in the same session. Nothing an agent needs later may live only in a conversation or in a plugin's memory. One to-do list per session: the one the app shows.

**Subagents.** Use one only for work that is independent or would flood the session with file contents. Look-ups go to the Explore agent, which loads no rules. Model and effort follow the task:

| Task | Model | Effort |
|---|---|---|
| Finding code, read-only look-ups, renames | the smallest | low |
| A plan task with a precise brief, tests, a review of one task | the middle one | medium |
| Design, an unknown cause, a whole-branch review | the largest the project allows | high |

Move up one step when a subagent returns weak work; do not retry on the same one.

**Housekeeping.** Done means clean:

- A task ends with its PR merged or closed, its branch deleted on the machine and on the remote, its worktree removed, and the default branch up to date on the machine.
- Nothing is parked. A branch has a PR within a day, and a PR nobody has touched for 7 days is finished or closed with a note (its commits stay reachable from the closed PR).
- Everything an agent makes outside the checkout goes under one scratch folder, `<temp>/agent-work/<repository>/<task>/`, and is removed when the task ends. No junction or symbolic link into the checkout, ever.
- A leftover is something with no open PR behind it. The project's tidy check reports: a branch that is merged, whose remote is gone, or that has no open PR; a worktree whose branch has no open PR; a stash; untracked files while on the default branch; a link inside the checkout; a scratch folder whose task has no open PR; a PR idle for 7 days. Work in progress is not a leftover: the current branch with its open PR, its worktree and its untracked files are left alone, and so are release and hotfix branches with an open PR and PRs opened by a bot. It changes nothing and prints nothing when all is clean. It runs at the start of every session, so leftovers show the next day, at no cost when there are none.
- What an agent may do with the report: delete on its own only what cannot lose work, that is a merged branch (`git branch -d`), a worktree with no changes whose branch is merged, an empty folder. Anything else (an unmerged branch, a stash, untracked files, a folder with content, an idle PR) it shows to the owner and asks.

**The person's machine.** Ask before a run that opens windows, needs a quiet machine or installs something; say how long and what will show; one yes covers one run. Builds, unit tests and git need no ask.

**Pull requests.** Draft while working; ready when the checks named by the project pass; merged only on the owner's word or the project's own merge rule; the branch deleted with the merge.

### 3. Grace fine-tuning

**Where today's `AGENTS.md` goes.** Text moves as it is unless the table below changes it.

| Today's section | Goes to | `paths:` |
|---|---|---|
| Intro, Commands, Docs map, No backward compatibility, Performance | `AGENTS.md` | always |
| The owner's machine, housekeeping, subagent rules, commit and PR basics | `WORKFLOW.md` (general); Grace keeps only what differs: the reference machine, the second screen, `GRACE_USER_DATA`, the model names (`haiku`, `sonnet`, `opus`; never Fable) | always |
| Doc rules, branches, PR rules that are Grace's (issue links, labels, `merge-when-green`, screenshots branch) | `AGENTS.md`, cut to what an agent needs for every PR | always |
| Workflow table, SignPath, Dependabot and bump workflows | `.claude/rules/ci.md` | `.github/**`, `scripts/check-pr.mjs`, `scripts/bump-*.mjs`, `scripts/vendor.json` |
| Release and hotfix steps, the release benchmark | `docs/process/release.md` | linked from `AGENTS.md` |
| Stack, Solid 2 rules, UI conventions | `.claude/rules/electron.md` | `src/**`, `e2e/**`, `bench/**`, `*.config.*`, `package.json` |
| GPUI rules | `.claude/rules/gpui.md` | `crates/grace/**`, `crates/grace-core/**`, `crates/grace-e2e/**`, `patches/**`, `Cargo.*`, `rust-toolchain.toml`, `scripts/e2e-gpui.sh` |
| Rules that bind both apps until the cut-over (the theme tokens equal `src/index.css`, a new icon is added on both sides) | `.claude/rules/both-apps.md`, so they load whichever side is edited | `src/index.css`, `src/renderer/**`, `crates/grace/src/theme*`, `crates/grace/assets/**` |
| Karate integration rules | `.claude/rules/karate.md` | `src/main/karate/**`, `src/main/runner*`, `crates/grace-core/**`, `listener/**`, `test/fixtures/**` |

**Keep, change, cut.**

| Practice | Verdict | Why |
|---|---|---|
| Conventional commits, `checks`, `scripts/check-pr.mjs` | Keep | Automatic; costs nothing per PR |
| Feature branch, PR, `merge-when-green` | Keep | The owner merges by one label |
| Git flow with `main`, `develop`, release branches | Keep | Heavier than a one-person app needs, but the workflows, `check-pr` and the updater rest on it; changing it costs more than it saves now |
| Decision records | Keep, for a dependency, an architecture or a file format only | 33 in 19 days; a process tweak is a rule change, not a record |
| The parity list, phase plans with Redesigns, check records of the port | Keep | They are the proof the cut-over needs |
| Performance priorities, `bench/limits.json`, the release benchmark and the owner's `Decision:` | Keep | The app's first requirement |
| An issue for every PR | **Change**: by tier (above). A Small docs, CI or chore PR needs none; its body is the record | An issue and a PR that say the same thing twice |
| A spec and a plan for every feature | **Change**: by tier | 45 of each; small work does not need them |
| Gherkin acceptance criteria in every spec | **Change**: only for behaviour someone using the app sees | They became e2e tests and parity rows there; elsewhere they are form |
| A `## Cost` section in every spec | **Change**: one line, "Cost: none", unless the feature adds a dependency, a process, a timer or work on the run path | Most specs repeat the same four zeros |
| Full `npm run e2e` on the machine before every code PR into `develop` | **Change**: run the end-to-end tests of the app the PR changes (`src/` or `crates/`). Small: the spec files of the touched area. Feature and Project: the whole suite once, before ready | A `crates/` PR gains nothing from Electron's suite; the suite still runs in full on every PR into `main` |
| Before and after screenshots in every UI PR | Keep for Feature and Project; Small only when the look changes | The owner reads PRs by them |
| Subagents always at extra-high effort | **Cut**: effort by task (above) | The largest single cost found |
| Hand-over notes as long issue comments | **Change**: the tracking issue's body holds the state (where each phase stands, what is next, questions for the owner), edited in place | One place, always current, no 4,000-word note to re-read |
| Dead stack rules (scaffold command, Kobalte, TanStack Router) | Cut | The code is gone (#353) |
| `perf` on PRs that change only `crates/` | **For the owner** (not part of this work): skip it until the bench measures GPUI Grace (#334) | 20 Windows minutes for a comment that says nothing; ADR 0023 asks for the comment on every code PR, so skipping needs his word |

**Plugins and connectors, for this repository** (`.claude/settings.json`): off for Vercel, Supabase, Firecrawl, taste-skill and impeccable. Connectors and the plugins of the claude.ai account cannot be switched off from the repository in the desktop app (seen on 2026-11-12: `disableClaudeAiConnectors` changed nothing there): the owner switches them in the app. Superpowers, ponytail and claude-mem stay on together (the owner, 2026-11-12), each with one job, so they do not compete:

| Plugin | Its job | Not its job |
|---|---|---|
| superpowers | The process: design, plan, test first, review, finishing a branch | |
| ponytail | How much gets built: the smallest complete change | It never drops a step of the tier or of the floor |
| claude-mem | Recall: searching what earlier sessions did and found | The to-do list (the app's is the only one) and rules or decisions (they go in the repository) |

Where they disagree, the order is: the project's rules, then `WORKFLOW.md`, then superpowers, then ponytail. None of the three loads in a cloud session. humanizer and i-have-adhd are the owner's to choose (Open questions).

**Superpowers paths.** `AGENTS.md` keeps its two overrides: designs in `docs/specs/`, plans in `docs/plans/`.

### 4. What this replaces

- PR #353 merges first, once the owner has confirmed its two changed rules (Open questions). This work then starts from `develop`; no PR is opened on another PR's branch. #353's text is split like the rest.
- Files that cite a section of `AGENTS.md` are updated with it: `.github/pull_request_template.md`, and the messages of `scripts/check-pr.mjs` and `scripts/bump-pr.mjs`. Accepted decision records are not edited: the headings they link (`Git workflow`, `No backward compatibility`) stay in `AGENTS.md`.
- The Subagents section's "always extra high" rule.
- The "every PR links an issue" rule, for Small PRs only.

### 5. Scripts

The owner's requirements (2026-11-12): one convention for all scripting, in a safe technology; Rust first, then Python, then JavaScript, moving down only when the job cannot be done in the language above; where each toolchain is installed and which one a project uses are configurable; and the whole stays simple.

**General (`WORKFLOW.md`, a few lines).**

- A project has one script command and one home for its scripts. Its `AGENTS.md` names them in one line.
- The order is Rust, then Python, then JavaScript. A project that sets no language uses the first of them it already builds with. A script goes one language down only when the one above cannot do the job (it must run inside another runtime, such as a browser driver), and the PR says why in one line. No new shell script: a CI step is a one-liner that calls the script command.
- Three rules in any language: a program is started with an argument list, never through a shell; the standard library before a dependency; every script has tests.
- Nothing hard-codes where a toolchain lives or which version runs. The version is the language's own pin file (`rust-toolchain.toml`, `.python-version`); the place is the toolchain's own variables (`RUSTUP_HOME` and `CARGO_HOME`; `UV_PYTHON_INSTALL_DIR`).

**Grace.** Scripts are Rust: one crate, `crates/xtask`, run as `cargo xtask <command>` (an alias in `.cargo/config.toml`), with `#![forbid(unsafe_code)]`, the workspace's lints and no dependency. Each command is a module of pure functions with unit tests; `main` reads the arguments, calls them and sets the exit code. This work adds three commands and no more: `check-rules`, `check-pr`, `tidy`.

**The existing scripts.** All 19 are in use: each is called by `package.json`, by a workflow, or imported by one that is (17 JavaScript files, 3,837 lines; 2 shell files, 49 lines). None is removed. They are ported when touched (the owner, 2026-11-12): a script is rewritten in Rust when it is next changed in substance. Phase 2 ported the three checks of `scripts/check-pr.mjs` that need nothing from the bench; the two that judge the bench's JSON (a release's performance record, a raised limit) call `bench/judge.mjs` and stay JavaScript until the bench is ported, so the judging rules exist once. `bench/` (2,485 lines) stays JavaScript: it drives Electron through Playwright and goes at the cut-over, with whatever JavaScript is left then. The one script outside every automatic path is `scripts/make-icon.mjs` (48 lines, `npm run icon`), kept because it is the only way to rebuild the icon.

### 6. Enforcement

The owner's requirement (2026-11-12): whether the workflow was followed is checked by machines, not by trust. Three mechanisms, all scripts or settings: none costs licence usage. What no script can check is listed at the end, so nobody takes it for enforced.

**1. The required CI check.** The PR template shrinks from 107 lines to a summary, a `Tier: small|feature|project` line and the issue line. `checks`, the one workflow the ruleset requires, runs `cargo xtask check-rules` and `cargo xtask check-pr`, which reads the PR body and the diff and holds the checks ported from `scripts/check-pr.mjs`; that script keeps its two bench checks and still runs. A PR that fails cannot merge.

| Check | Fails when |
|---|---|
| Tier | The body has no `Tier:` line. A bot branch (`dependabot/*`, `feature/bump-*`) counts as Small without one |
| Small stays small | A Small PR adds a dependency (a new name in `package.json` or a `Cargo.toml`), adds a file under `docs/decisions/`, or changes more than 400 lines outside docs, lock files and fixtures: the message says to step up a tier |
| Feature and Project have their papers | The body has no `Closes #N` or `Part of #N`, or no `Spec:` and `Plan:` lines naming files that exist at the PR's head |
| Tests come with behaviour | A PR with a `feat` or `fix` commit changes code under `src/` or `crates/` and no test file |
| Script language | The PR adds a `.py`, `.mjs`, `.js`, `.sh` or `.ps1` file outside `src/`, `e2e/`, `test/` and `bench/`, and its body has no `Script-language:` line giving the reason |
| Agent traces | A commit message or the body holds a claude.ai session or project link, a `Claude-Session:` trailer or a "Requested by" line |
| Rules files (`check-rules`) | `WORKFLOW.md` or `AGENTS.md` is over its size limit, `WORKFLOW.md` names the project, or a glob in a `.claude/rules/*.md` `paths:` list matches no tracked file |

**2. Refusals, as settings.** `.claude/settings.json` denies, with no script behind it, the commands that caused harm: making a junction or a symbolic link (`mklink`, `New-Item -ItemType Junction` or `SymbolicLink`, `ln -s`), and `git push --force`. The repository's ruleset already refuses a push to `main` or `develop` outside a PR.

**3. The tidy report.** A `SessionStart` hook runs `cargo xtask tidy`. It reports leftovers on the machine and, when `gh` is there, on GitHub: branches with no open PR other than `main`, `develop` and `feature/pr-screenshots`, and PRs idle for 7 days. It prints nothing when all is clean and never fails the session, a missing `cargo` included.

**Not checked by a script, and why.**

| Rule | Why not | What stands in |
|---|---|---|
| The test was written before the code | Order inside a session leaves no trace in the diff | The "tests come with behaviour" check, and the whole-branch reviewer on Feature and Project |
| The owner approved the design | A file cannot prove who wrote a line | The spec's `Status:` line, and the owner merging the PR |
| The tier was judged honestly | Size is a judgement | The "Small stays small" limits |
| The commands were run and passed | A pasted summary proves nothing | CI runs them itself |

## Cost

This work is its own measure. The gate is bytes, which a script counts: the files loaded at start, before and after, in the PR body. Expected: from 34 KB (about 8,500 tokens) to at most 14 KB (about 3,500), the rest loading only with the files it governs; the skills list shorter by five plugins. An agent cannot run `/context`; the PR body asks the owner to look at it once in a fresh session, as a second reading and not a gate. The tidy command adds one process at session start and no text when the repository is clean; `checks` gains the build of a crate with no dependency. Nothing changes in the app.

## Acceptance checks

1. `WORKFLOW.md` names no Grace file, command, issue or person. A test fails if it contains `grace`, `karate`, `gpui` or `hpagniez`, whatever the case.
2. `WORKFLOW.md` is at most 5 KB, `AGENTS.md` at most 9 KB, and the two together at most 14 KB. A test fails otherwise.
3. Every rule sentence of today's `AGENTS.md` (as on #353) is in one of the new files, or is listed in the PR body as cut or changed with this document's reason.
4. Each `.claude/rules/*.md` has a `paths:` list, and every glob in it matches at least one tracked file. A test fails otherwise.
5. A test reads each rules file's `paths:` and checks, for one sample file per area (`crates/grace/src/app.rs`, `src/renderer/App.tsx`, `.github/workflows/checks.yml`, `src/index.css`), which rules files match it: GPUI and Karate rules for the first and not Electron's or CI's, and so on. That Claude Code then loads them is its documented behaviour; the owner's one look with `/memory` confirms it.
6. `cargo xtask tidy` prints nothing and exits 0 on a clean checkout, and names each leftover on a checkout that has a second worktree, a merged branch, a branch whose remote is gone, a stash, an untracked file or a junction. Its test builds each case in a temp repository.
7. The byte counts of the files loaded at start, before and after, are in the PR body.
8. Each row of the check table in Enforcement has a unit test with a PR that passes and one that fails, in `crates/xtask`, and every test of the three ported checks has its port there; `scripts/check-pr.mjs` and its test keep only the two bench checks.
9. With the settings in place, a shell command that makes a junction is refused (tried once by hand, recorded in the PR body).
10. `cargo xtask tidy` names a remote branch that has no open PR (its test feeds it a branch list and a PR list), and prints nothing and exits 0 when `cargo` or `gh` is missing from the hook's path.
11. `crates/xtask` has `#![forbid(unsafe_code)]` and no dependency, and passes `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
12. `npm run lint`, `npm run typecheck`, `npm test` and `cargo test --workspace` pass.

## The owner decided

- Approach B: tiers plus lean context (2026-11-12).
- Superpowers is the basis of the general way.
- Quality, documentation for agents and code quality are not traded for speed.
- Housekeeping must be stronger.
- Following the workflow is verified by machines: guardrails and CI checks.
- Scripts: one convention, Rust first, then Python, then JavaScript; toolchain place and version configurable; existing scripts ported when touched.
- The whole must stay simple: control and documentation, not complexity.

## Calls made in this document (the owner can overrule each)

- Three tiers, not four: a trivial change is a Small one.
- The general layer is one file in the repository, copied by other projects, not a plugin or a user-level file: it is the only form that reaches local sessions, cloud sessions and other tools.
- No template repository yet: `WORKFLOW.md` is copied from Grace until a second project needs it.
- Rules by path live in `.claude/rules/`, not in nested `AGENTS.md` files: the Karate rules span two folders, and the documentation does not say a nested `AGENTS.md` loads beside a root `CLAUDE.md`.
- Git flow stays.
- A PR idle for 7 days, and a branch without a PR for a day, count as leftovers.
- Enforcement is scripts and settings only. A weekly agent that audits merged PRs against `WORKFLOW.md` would catch more, but it spends the licence usage this work is meant to save; it is left out.
- Kept simple on purpose (the owner's steer): one script tool with three commands; refusals as settings, not as a hook script; no weekly workflow, since the tidy report at session start covers GitHub too; no "Verification" section to fill, since CI runs the commands itself; no second general file and no table of rules per language.
- Toolchain configuration uses each toolchain's own pin file and variables. No new configuration file.
- The Small limit is 400 changed lines outside docs, lock files and fixtures.
- The work lands as three PRs, each a Project-tier phase with its own plan: the files and their split (sections 1 to 3), the PR checks ported to Rust with the PR template (section 6), then the tidy command with its hook and the refusals. Phase 1 also starts `crates/xtask`.

## Questions the owner answered (2026-11-12)

- **PR #353's two changed rules** stand as written, and #353 is merged: attribution as `.claude/settings.json` gives it, with no session link; a go on his machine only for what opens windows, needs a quiet machine or installs something.
- **humanizer and i-have-adhd** stay on.
- **`perf` on `crates/`-only PRs** is skipped until #334, as a follow-up of its own.
- **Sessions** run mostly on his machine. In a cloud session superpowers is not there and `WORKFLOW.md` alone carries the process.

## Left to the builder

The exact wording of `WORKFLOW.md` within the limits above; the split of each sentence of `AGENTS.md` between files; the test file names; how the tidy command finds idle PRs (it skips that check when `gh` is missing or offline).
