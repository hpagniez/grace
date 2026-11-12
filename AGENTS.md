# AGENTS.md

Follow [WORKFLOW.md](WORKFLOW.md), the general way of working; this file holds what is Grace's own.

Grace is a free, open-source desktop test-suite runner for [Karate](https://docs.karatelabs.io/) v2. It wraps the open-source Karate CLI in an Electron GUI. Karate's own GUI runners are paid. This repo is built mostly by AI agents. The app was called karate-kid until [ADR 0013](docs/decisions/0013-rename-the-app-to-grace.md); dated specs, plans, checks and research notes keep that name. The repository is `hpagniez/grace`.

Grace is moving to Rust and GPUI ([ADR 0031](docs/decisions/0031-move-grace-to-rust-and-gpui.md)): until the cut-over the Electron app ships, and GPUI Grace lives beside it in `crates/`. Do not copy code, text or assets from Karate Labs' paid products. Learn the feature set only.

## Rules by area

Claude Code loads each rules file with the files it governs; any other tool reads the one for the files it touches.

| File | Holds |
|---|---|
| `.claude/rules/electron.md` | Stack, Solid 2 rules, UI conventions, the bench, Electron's commands and e2e |
| `.claude/rules/gpui.md` | GPUI rules, the port's rules, Rust commands and e2e |
| `.claude/rules/karate.md` | How Grace drives Karate, for both apps |
| `.claude/rules/both-apps.md` | The theme and icons, which change on both sides until the cut-over |
| `.claude/rules/ci.md` | Workflows, limits, issue and board recipes. Read it before marking a code PR ready: the checks to wait for, the `perf` comment, the project card |
| `.claude/rules/docs.md` | Docs map; rules for specs, plans, issues and decision records |
| [docs/process/release.md](docs/process/release.md) | Release and hotfix steps, the release benchmark: read it before any release work |
| [docs/process/owner-machine.md](docs/process/owner-machine.md) | Read it before any run on the owner's machine: a measure, the bench, a by-hand check |

Designs go in `docs/specs/YYYY-MM-DD-<topic>.md` and plans in `docs/plans/`: this overrides the superpowers defaults.

## Commands

Windows 11, Node 24. Run `npm install`, then `npm run vendor` once (downloads the bundled JRE and Karate jar into `vendor/`, and compiles the debug listener in `listener/` into `vendor/listener.jar`; run it again after changing the listener).

| Command | What it does |
|---|---|
| `npm test` | Unit tests (Vitest) |
| `npm run e2e` | Package the app, then run the Playwright end-to-end tests |
| `npm run typecheck` | `tsc --noEmit` |
| `npm run lint` | oxlint and oxfmt check |
| `cargo xtask <command>` | Grace's Rust scripts: `check-rules`, `check-pr`, `scope`, `tidy` (run at session start) |

Never run any two of `npm test`, `npm run e2e` and `npm run bench` at the same time. The e2e Java leak checks scan every `java.exe` under `vendor/jre` and see the unit tests' processes, so never run two e2e runs at once either. Nor run Karate yourself during e2e (a measurement script, the big project): its thousands of local HTTP calls make Windows refuse Playwright's connection to a starting app, and the test fails with `Process failed to launch!` ([#191](https://github.com/hpagniez/grace/issues/191)). The bench needs a quiet machine, and it and e2e both package into `out/`. Unit test files run in parallel; their leak checks see only their own project's Java ([ADR 0020](docs/decisions/0020-run-unit-test-files-in-parallel.md)).

## No backward compatibility

The app is still in development. Never write code that keeps older versions working: no migration of settings, run history or data folders, no fallback to an old name, key, environment variable or file format, no deprecated alias. Change the name or format, and update every place that uses it. Data from an older version may be lost: say so in the commit's `BREAKING CHANGE:` footer.

This rule ends at the first stable release, `1.0.0`. From then on, people get new versions through the in-app updater, so every version must read the data of every earlier stable version. A rollback is not promised to read newer data. The updater's own contract (release tag, zip name and layout, `update-result.json`, backup layout, the `updates` settings key) never changes without a new decision record, pre-releases included ([ADR 0019](docs/decisions/0019-update-the-zip-install-from-github-releases.md)).

## Performance

Decided in [ADR 0023](docs/decisions/0023-run-lightly-as-a-release-requirement.md); spec: [docs/specs/2026-10-31-light-and-fast.md](docs/specs/2026-10-31-light-and-fast.md). Grace stays open all day beside IDEs, browsers and containers. Its priorities, in order; a lower one never wins over a higher one:

1. Run lightly: memory, CPU and a smooth screen, idle and during a run, Java included.
2. Be fast: start and response.
3. Be small: download and install size.

- Idle: while no run is going, nothing in Grace wakes it while its window is minimised or hidden (no timer, poll, animation or redraw), and it stays within the `idle.cpu.minimised` limit then and the `idle.cpu.shown` limit while shown. Grace's own code adds no timer, poll or animation while idle; the budget is for the engine's housekeeping ([ADR 0030](docs/decisions/0030-no-wake-ups-while-hidden-and-an-idle-cpu-budget.md)).
- A proposal to change the stack (shell, UI framework, runtime) is judged first on running lightly, measured with the bench against the current build ([#152](https://github.com/hpagniez/grace/issues/152)).

## Git workflow

Decided in [ADR 0003](docs/decisions/0003-conventional-commits-and-git-flow.md).

### Commits

[Conventional Commits 1.0.0](https://www.conventionalcommits.org/en/v1.0.0/): `<type>(<scope>)<!>: <subject>`.

- Types: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`.
- Scope is optional: the area touched (`docs`, `runner`, `ui`, `ipc`, `deps`). Reuse an existing scope before inventing one.
- Subject: imperative, lower case, no trailing period, 72 characters max.
- Breaking change: `!` after the scope, plus a `BREAKING CHANGE:` footer.
- One logical change per commit.

### Branches (git flow)

| Branch | Cut from | Merges into | Notes |
|---|---|---|---|
| `main` | | | Released code only. Tagged `vX.Y.Z-rc.N` (SemVer pre-release). |
| `develop` | `main` | | Integration branch. |
| `feature/<topic>` | `develop` | `develop` | Via PR. Kebab-case topic. |

Release and hotfix branches: [docs/process/release.md](docs/process/release.md).

- Merge with `--no-ff`.
- After a merge, always delete the merged `feature/*`, `release/*` or `hotfix/*` branch, local and remote (`git branch -d <branch>`, then `git push origin --delete <branch>`). Use `-d`, not `-D`: if git refuses, the branch is not merged. Delete the branch of a PR closed unmerged too. Never delete `main`, `develop` or `feature/pr-screenshots`.
- Never commit directly to `main` or `develop`, and never force-push them. Agents work on a `feature/*` branch.

### Pull requests

- `develop` is the default branch, so new PRs target it. Pick `main` for `release/*` and `hotfix/*` PRs.
- A Feature or Project PR links its issue: `Closes #N` when it finishes the issue, `Part of #N` when it does not. A Small PR needs an issue only for a reported bug (the `bug` label and the sections of `.github/ISSUE_TEMPLATE/bug.yml`) or when it leaves work unfinished.
- A `Closes #N` in a commit message closes the issue when the commit lands on `develop`: write `Part of #N` in commits while the issue must stay open.
- A fix found while working on another PR gets its own PR.
- Assign the PR to `hpagniez`. He cannot be asked for a review: the PRs are opened under his account.
- Commits and PR bodies end with the attribution lines `.claude/settings.json` gives, and say nothing else about the agent: no claude.ai session or project link, no `Claude-Session:` trailer, no "Requested by" line. Read a PR's body back after creating it and remove any.
- Never open a PR on another PR's branch: it gets no CI, and `merge-when-green` deletes its base branch, which closes it. Wait for the first to merge.
- To have a PR into `develop` merge itself once CI is green, add the `merge-when-green` label, only when the user asks for the merge. It merges with a merge commit, deletes the branch, and closes the issues the PR names with `Closes #N` (GitHub ignores closing keywords in a merge made with the workflow token). PRs into `main` and PRs from forks are merged by hand.

## Grace's own

- Scripts are Rust, in `crates/xtask`. The toolchain's version is `rust-toolchain.toml`; its place is `RUSTUP_HOME` and `CARGO_HOME`. The existing JavaScript and shell scripts are ported when touched; `bench/` stays JavaScript until the cut-over.
- Subagent models: `haiku`, `sonnet` and `opus` are the smallest, the middle one and the largest. Never Fable. Never leave the model unset: an unset model inherits the main agent's.
- Plugins: superpowers carries the process; ponytail keeps the change small; claude-mem is for recall only, never the to-do list or a rule's home.
