# Way of working: tiers, lean context, housekeeping

Date 2026-11-12. Status: proposed, for the owner's approval. Nothing here is applied yet.

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
- A leftover is something with no open PR behind it. `scripts/tidy.mjs` reports: a branch that is merged, whose remote is gone, or that has no open PR; a worktree whose branch has no open PR; a stash; untracked files while on the default branch; a link inside the checkout; a scratch folder whose task has no open PR; a PR idle for 7 days. Work in progress is not a leftover: the current branch with its open PR, its worktree and its untracked files are left alone, and so are release and hotfix branches with an open PR and PRs opened by a bot. It changes nothing and prints nothing when all is clean. A `SessionStart` hook in `.claude/settings.json` runs it, so leftovers show at the start of the next session, at no cost when there are none.
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
| GPUI rules | `.claude/rules/gpui.md` | `crates/**`, `patches/**`, `Cargo.*`, `rust-toolchain.toml`, `scripts/e2e-gpui.sh` |
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

**Plugins and connectors, for this repository** (`.claude/settings.json`): off for Vercel, Supabase, Firecrawl, taste-skill and impeccable; claude.ai connectors off. Superpowers and ponytail stay on. claude-mem, humanizer and i-have-adhd are the owner's to choose (Open questions).

**Superpowers paths.** `AGENTS.md` keeps its two overrides: designs in `docs/specs/`, plans in `docs/plans/`.

### 4. What this replaces

- PR #353 merges first, once the owner has confirmed its two changed rules (Open questions). This work then starts from `develop`; no PR is opened on another PR's branch. #353's text is split like the rest.
- Files that cite a section of `AGENTS.md` are updated with it: `.github/pull_request_template.md`, and the messages of `scripts/check-pr.mjs` and `scripts/bump-pr.mjs`. Accepted decision records are not edited: the headings they link (`Git workflow`, `No backward compatibility`) stay in `AGENTS.md`.
- The Subagents section's "always extra high" rule.
- The "every PR links an issue" rule, for Small PRs only.

### 5. Enforcement

The owner's requirement (2026-11-12): whether the workflow was followed is checked by machines, not by trust. Every check below is a script: none costs licence usage. A rule that no script can check is listed at the end, so nobody takes it for enforced.

**The PR says its tier.** The PR template shrinks from 107 lines to a summary, one `Tier: small|feature|project` line, the issue line, and `## Verification`. `scripts/check-pr.mjs` reads the PR body (passed by `checks.yml`, which also runs when the body is edited) and the diff:

| Check in `check-pr.mjs`, on every PR | Fails when |
|---|---|
| Tier | The body has no `Tier:` line. A bot branch (`dependabot/*`, `feature/bump-*`) counts as Small without one |
| Small stays small | A Small PR adds a dependency (a new name in `package.json` or a `Cargo.toml`), adds a file under `docs/decisions/`, or changes more than 400 lines outside docs, lock files and fixtures: the message says to step up a tier |
| Feature and Project have their papers | The body has no `Closes #N` or `Part of #N`, or no `Spec:` and `Plan:` lines naming files that exist at the PR's head |
| Tests come with behaviour | A PR with a `feat` or `fix` commit changes code under `src/` or `crates/` and no test file |
| Verification | A PR that changes code has an empty `## Verification` section (it holds the commands run and their summary lines) |
| Agent traces | A commit message or the body holds a claude.ai session or project link, a `Claude-Session:` trailer or a "Requested by" line |
| Rules files | `WORKFLOW.md` or `AGENTS.md` is over its size limit, `WORKFLOW.md` names the project, or a glob in a `.claude/rules/*.md` `paths:` list matches no tracked file |

These run in `checks`, the one workflow the ruleset requires, so a PR that fails them cannot merge. The existing checks (commit headers, branch target, release records, raised limits) stay.

**Guardrails on the machine**, as hooks in `.claude/settings.json`, which load in local and cloud sessions alike:

| Hook | Does |
|---|---|
| `SessionStart` | Runs `scripts/tidy.mjs`; its report, when there is one, is the first thing the session reads |
| `PreToolUse` on shell commands | Refuses a command that makes a junction or symbolic link (`mklink`, `New-Item -ItemType Junction` or `SymbolicLink`, `ln -s`), and a `git commit` while on `main` or `develop` |

**A weekly sweep on GitHub.** A new workflow, `housekeeping` (scheduled, Linux, GitHub-owned actions only), runs `scripts/tidy.mjs --remote`: branches with no open PR other than `main`, `develop` and `feature/pr-screenshots`, PRs idle for 7 days, issues still open whose closing PR merged. It keeps one issue, "Housekeeping", open with the list while there is one, and closes it when there is none.

**Not checked by a script, and why.**

| Rule | Why not | What stands in |
|---|---|---|
| The test was written before the code | Order inside a session leaves no trace in the diff | The "tests come with behaviour" check, and the whole-branch reviewer on Feature and Project |
| The owner approved the design | A file cannot prove who wrote a line | The spec's `Status:` line, and the owner merging the PR |
| The tier was judged honestly | Size is a judgement | The "Small stays small" limits |
| The scratch folder was emptied on a cloud session's machine | The machine is gone with the session | Nothing needed |

## Cost

This work is its own measure. The gate is bytes, which a script counts: the files loaded at start, before and after, in the PR body. Expected: from 34 KB (about 8,500 tokens) to at most 14 KB (about 3,500), the rest loading only with the files it governs; the skills list shorter by five plugins. An agent cannot run `/context`; the PR body asks the owner to look at it once in a fresh session, as a second reading and not a gate. `tidy.mjs` adds one process at session start, about a second, and no text when the repository is clean. Nothing changes in the app.

## Acceptance checks

1. `WORKFLOW.md` names no Grace file, command, issue or person. A test fails if it contains `grace`, `karate`, `gpui` or `hpagniez`, whatever the case.
2. `WORKFLOW.md` is at most 5 KB, `AGENTS.md` at most 9 KB, and the two together at most 14 KB. A test fails otherwise.
3. Every rule sentence of today's `AGENTS.md` (as on #353) is in one of the new files, or is listed in the PR body as cut or changed with this document's reason.
4. Each `.claude/rules/*.md` has a `paths:` list, and every glob in it matches at least one tracked file. A test fails otherwise.
5. A test reads each rules file's `paths:` and checks, for one sample file per area (`crates/grace/src/app.rs`, `src/renderer/App.tsx`, `.github/workflows/checks.yml`, `src/index.css`), which rules files match it: GPUI and Karate rules for the first and not Electron's or CI's, and so on. That Claude Code then loads them is its documented behaviour; the owner's one look with `/memory` confirms it.
6. `scripts/tidy.mjs` prints nothing and exits 0 on a clean checkout, and names each leftover on a checkout that has a second worktree, a merged branch, a branch whose remote is gone, a stash, an untracked file or a junction. Its test builds each case in a temp repository.
7. The byte counts of the files loaded at start, before and after, are in the PR body.
8. Each row of the `check-pr.mjs` table in Enforcement has a unit test with a PR that passes and one that fails, in `scripts/check-pr.test.ts` beside the existing ones.
9. With the hooks in place, a shell command that makes a junction is refused, and so is `git commit` on `develop` (each tried once by hand, recorded in the PR body).
10. `housekeeping`, run by hand on a repository state with a branch that has no PR, opens the "Housekeeping" issue naming it; run again after the branch is deleted, it closes the issue.
11. `npm run lint`, `npm run typecheck` and `npm test` pass.

## The owner decided

- Approach B: tiers plus lean context (2026-11-12).
- Superpowers is the basis of the general way.
- Quality, documentation for agents and code quality are not traded for speed.
- Housekeeping must be stronger.
- Following the workflow is verified by machines: guardrails and CI checks.

## Calls made in this document (the owner can overrule each)

- Three tiers, not four: a trivial change is a Small one.
- The general layer is one file in the repository, copied by other projects, not a plugin or a user-level file: it is the only form that reaches local sessions, cloud sessions and other tools.
- No template repository yet: `WORKFLOW.md` and `tidy.mjs` are copied from Grace until a second project needs them.
- Rules by path live in `.claude/rules/`, not in nested `AGENTS.md` files: the Karate rules span two folders, and the documentation does not say a nested `AGENTS.md` loads beside a root `CLAUDE.md`.
- Git flow stays.
- A PR idle for 7 days, and a branch without a PR for a day, count as leftovers.
- Enforcement is scripts only. A weekly agent that audits merged PRs against `WORKFLOW.md` would catch more, but it spends the licence usage this work is meant to save; it is left out.
- The Small limit is 400 changed lines outside docs, lock files and fixtures.
- The work lands as three PRs, each a Project-tier phase with its own plan: the files and their split (sections 1 to 3), the checks in `check-pr.mjs` with the PR template (section 5), then `tidy.mjs` with the hooks and the `housekeeping` workflow.

## Open questions for the owner

0. **PR #353's two changed rules**, which this work builds on: attribution as `.claude/settings.json` gives it, with no session link (the resume note said no attribution at all); and a go needed on your machine only for what opens windows, needs a quiet machine or installs something.
1. **claude-mem** in this repository: keep it on, or off? It adds a second memory and a second to-do list, and its session-start text is paid in every session. Off fits "the repository is the memory"; on keeps its search over past sessions.
2. **humanizer and i-have-adhd**: personal tools, left on unless you say otherwise.
3. **`perf` on `crates/`-only PRs**: skip it until #334, yes or no (a follow-up of its own).
4. **Sessions**: this design assumes most work now runs on your machine, with cloud sessions as the exception. In a cloud session superpowers is not there and `WORKFLOW.md` alone carries the process. If cloud is to be the main place again, the superpowers skills would have to be copied into the repository.

## Left to the builder

The exact wording of `WORKFLOW.md` within the limits above; the split of each sentence of `AGENTS.md` between files; the test file names; how `tidy.mjs` finds idle PRs (it may skip that check when `gh` is missing or offline).
