# AGENTS.md

Grace is a free, open-source desktop test-suite runner for [Karate](https://docs.karatelabs.io/) v2. It wraps the open-source Karate CLI in an Electron GUI. Karate's own GUI runners are paid. This repo is built mostly by AI agents. The app was called karate-kid until [ADR 0013](docs/decisions/0013-rename-the-app-to-grace.md); dated specs, plans, checks and research notes keep that name. The repository is `hpagniez/grace`.

## Commands

Windows 11, Node 24. Run `npm install`, then `npm run vendor` once (downloads the bundled JRE and Karate jar into `vendor/`, and compiles the debug listener in `listener/` into `vendor/listener.jar`; run it again after changing the listener).

| Command | What it does |
|---|---|
| `npm start` | Run the app in dev mode |
| `npm test` | Unit tests (Vitest) |
| `npm run e2e` | Package the app, then run the Playwright end-to-end tests |
| `npm run bench` | Package the app and measure it: memory, CPU, start, smoothness, size. `-- --a <exe>` compares it with another build; `-- --release` adds the release-only measures (the release benchmark itself is in [Git workflow](#git-workflow)). Writes `bench-out/bench.json` and `bench.md` |
| `npm run typecheck` | `tsc --noEmit` |
| `npm run lint` | oxlint and oxfmt check |
| `npm run make` | Build the Windows zip in `out/make/zip/win32/x64/` |
| `npm run screens:readme` | Package the app, then retake the README's screenshots in `docs/images/` |
| `cargo run -p grace` | Run GPUI Grace, the port in `crates/` ([GPUI rules](#gpui-rules)) |
| `cargo test --workspace` | GPUI Grace's tests: `grace-core` and the headless views |
| `npm run package:gpui` | Build GPUI Grace in release and lay it out as the release zip (ADR 0019) in `out/gpui/`; Windows, after `npm run vendor`. Not released until phase P5 |
| `npm run bench:gpui` | The bench on GPUI Grace: size, start, idle and `idle.wakeups.minimised` (no Playwright). `-- --a <exe>` compares it with another build, such as Electron Grace's |
| `npm run cargo:bump` | Move GPUI Grace to the newest `gpui-pre`, GPUI Kit and Rust, the Windows patches rebased, then `cargo update` (ADR 0032); prints what moved. `cargo-updates` runs it weekly |
| `sh scripts/e2e-gpui.sh` | GPUI Grace's end-to-end tests through the accessibility tree (`crates/grace-e2e`); on Linux it starts Xvfb and AT-SPI itself |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo fmt --all` | GPUI Grace's lint and format |

Never run any two of `npm test`, `npm run e2e` and `npm run bench` at the same time. The e2e Java leak checks scan every `java.exe` under `vendor/jre` and see the unit tests' processes, so never run two e2e runs at once either. Nor run Karate yourself during e2e (a measurement script, the big project): its thousands of local HTTP calls make Windows refuse Playwright's connection to a starting app, and the test fails with `Process failed to launch!` ([#191](https://github.com/hpagniez/grace/issues/191)). The bench needs a quiet machine, and it and e2e both package into `out/`. Unit test files run in parallel; their leak checks see only their own project's Java ([ADR 0020](docs/decisions/0020-run-unit-test-files-in-parallel.md)).

On a machine with more than one screen, every e2e window opens on a screen that is not the main one, out of the way of the person working: `launch()` in `e2e/helpers.ts` moves it there before it shows. Launch the app in a test only through `launch()`. Only `window.spec.ts` keeps its windows on the main screen (`primaryScreen`), since it tests where the app places them. The bench opens its windows on the first screen that is not the main one too, so run the release benchmark with the same screen layout as the `machine` in `bench/limits.json`.

## The owner's machine

The reference machine of `bench/limits.json` is the owner's own PC, and he works on it. It is the only place for the release benchmark, the Windows by-hand rows and the measures CI's runners cannot give.

- Ask the owner before each run that opens windows, needs a quiet machine or installs something: e2e, the bench, a measure, a by-hand check, a toolchain. Say how long it takes and what will show on screen. One yes covers one run. Builds, unit tests and git need no ask.
- Work in separate worktrees under one temp folder, never in his working copy, and with a fresh data folder (`GRACE_USER_DATA`), never his settings. Copy `vendor/` into a worktree, never link it: a recursive delete through a junction empties the checkout's own `vendor/`.
- Right after the work, before anything else, remove everything it made: worktrees, temp folders, build outputs, logs, local branches, toolchains it installed. Delete only what the work made. Then check `git worktree list`, `vendor/` and the process list, and tell him what was removed.
- A check record says what ran there and with whose go. Keep his user name, his paths and captures of his desktop out of it.

## Docs map

| Path | What it holds |
|---|---|
| [Issues](https://github.com/hpagniez/grace/issues) and the [Grace project](https://github.com/users/hpagniez/projects/2) | Backlog: one issue per feature, priority as label must (first release), should or could; status idea → building → done on the board |
| [docs/features.md](docs/features.md) | Frozen map of the old backlog IDs F01–F45 to their issues |
| [docs/inventory/karate-core.md](docs/inventory/karate-core.md) | Every open-source Karate v2 feature, tagged by GUI use; §19 = how the app drives Karate |
| [docs/inventory/commercial.md](docs/inventory/commercial.md) | What Karate Labs' paid tools offer, and the gaps |
| [docs/research/](docs/research/) | Dated research notes with sources |
| [docs/decisions/](docs/decisions/) | Decision records (MADR 4.0 minimal) |
| `docs/specs/` | Design specs, one per feature, written just before building it |
| `docs/checks/` | Dated check records of the packaged app: what was run, on which machine, the result, and what still needs a person |
| `docs/plans/` | Implementation plans, one per spec, task by task (overrides the superpowers default `docs/superpowers/plans/`) |

## Doc rules

- Design specs go in `docs/specs/YYYY-MM-DD-<topic>.md` (this overrides the superpowers default `docs/superpowers/specs/`). Write requirements, then acceptance criteria in Gherkin, and a `## Cost` section ([Performance](#performance)).
- Every feature has an issue with a priority label. Its body holds the feature, a `Karate surface:` line and a `Spec:` line. New features get only an issue number, never a new F-ID; cite them as `#N`. When a spec is written, point the issue's `Spec:` line at the file on `develop`. `gh issue edit <N> --body-file -` replaces the whole body, so start from the current one (`gh issue view <N> --json body --jq .body`), change only that line, and pipe the full body back. A PR that builds a feature says `Closes #N`; a spec-only PR, or one of several, says `Part of #N`. The project's workflows move the status. For a feature built over several PRs, move its card to `building` when its first building PR (not a spec-only PR) opens: `gh project item-add 2 --owner hpagniez --url <issue URL> --format json --jq .id` gives the item ID, then `gh project item-edit --id <item ID> --project-id PVT_kwHOAka2MM4BpJIV --field-id PVTSSF_lAHOAka2MM4BpJIVzhnmDmA --single-select-option-id 47fc9ee4`. Both calls need a token with the `project` scope (`gh auth refresh -h github.com -s project`) ([ADR 0014](docs/decisions/0014-backlog-in-github-issues-and-a-project.md)).
- A choice that is hard to reverse (dependency, architecture, file format) gets a new `docs/decisions/NNNN-title.md`. Never edit an accepted record; supersede it.
- Do not copy code, text or assets from Karate Labs' paid products. Learn the feature set only.

## No backward compatibility

The app is still in development. Never write code that keeps older versions working: no migration of settings, run history or data folders, no fallback to an old name, key, environment variable or file format, no deprecated alias. Change the name or format, and update every place that uses it. Data from an older version may be lost: say so in the commit's `BREAKING CHANGE:` footer.

This rule ends at the first stable release, `1.0.0`. From then on, people get new versions through the in-app updater, so every version must read the data of every earlier stable version. A rollback is not promised to read newer data. The updater's own contract (release tag, zip name and layout, `update-result.json`, backup layout, the `updates` settings key) never changes without a new decision record, pre-releases included ([ADR 0019](docs/decisions/0019-update-the-zip-install-from-github-releases.md)).

## Performance

Decided in [ADR 0023](docs/decisions/0023-run-lightly-as-a-release-requirement.md); spec: [docs/specs/2026-10-31-light-and-fast.md](docs/specs/2026-10-31-light-and-fast.md). Grace stays open all day beside IDEs, browsers and containers. Its priorities, in order; a lower one never wins over a higher one:

1. Run lightly: memory, CPU and a smooth screen, idle and during a run, Java included.
2. Be fast: start and response.
3. Be small: download and install size.

- Every design spec has a `## Cost` section: what the feature adds to memory, CPU, start time or size, idle and during a run, and how it keeps to the idle rule below. A PR that adds a dependency states its size.
- Idle: while no run is going, nothing in Grace wakes it while its window is minimised or hidden (no timer, poll, animation or redraw), and it stays within the `idle.cpu.minimised` limit then and the `idle.cpu.shown` limit while shown. Grace's own code adds no timer, poll or animation while idle; the budget is for the engine's housekeeping ([ADR 0030](docs/decisions/0030-no-wake-ups-while-hidden-and-an-idle-cpu-budget.md)).
- Every code PR gets a comment from the `perf` workflow: the PR against its base, measured by `npm run bench`. Read it. When it shows a measure worse than base beyond its noise margin, say why in the PR body. A worse number never blocks a merge, but the PR body explains it.
- The limits are in `bench/limits.json`. Lowering one is an ordinary commit. Raising one needs a new decision record: `scripts/check-pr.mjs` fails a PR that raises a limit without adding a file under `docs/decisions/`.
- Before a release, run the release benchmark ([Git workflow](#git-workflow)).
- Never write a release's `Decision:` line. An agent writes the `### Analysis`; the owner writes `Decision: go` or `Decision: no-go`.
- A proposal to change the stack (shell, UI framework, runtime) is judged first on running lightly, measured with the bench against the current build ([#152](https://github.com/hpagniez/grace/issues/152)).

## Git workflow

Decided in [ADR 0003](docs/decisions/0003-conventional-commits-and-git-flow.md); release and hotfix branch names in [ADR 0009](docs/decisions/0009-release-and-hotfix-branches-named-after-their-tag.md); release candidates only in [ADR 0015](docs/decisions/0015-release-candidates-until-a-stable-release-is-decided.md).

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
| `release/vX.Y.Z-rc.N` | `develop` | `main` and `develop` | Named after its tag, e.g. `release/v1.0.0-rc.3`. Version bump and fixes only. Tag on `main`. |
| `hotfix/vX.Y.Z-rc.N` | `main` | `main` and `develop` | Named after its tag, e.g. `hotfix/v1.0.0-rc.4`. Urgent fix to a released version. |

- No stable release while the app is in development: every version is a release candidate of the first stable release, `1.0.0-rc.N`, in the branch, `package.json` and tag. Each release or hotfix bumps `N`. Stable `v1.0.0` needs a decision record that supersedes ADR 0015.
- Before a `release/*` or `hotfix/*` PR into `main`, check the public docs against the version it ships: `README.md` (features, install and update steps, status, roadmap) and its screenshots in `docs/images/`. Fix them on the branch; `npm run screens:readme` retakes the screenshots. The packaged-app check record, `docs/checks/<date>-<release|hotfix>-<version>-packaged.md`, gets a `## Public docs` section: what was checked, and what changed or why nothing had to. `scripts/check-pr.mjs` fails the PR without it.
- Before a `release/*` or `hotfix/*` PR into `main`, also run the release benchmark on the reference machine, the one named in `bench/limits.json`. Run `npm run make`, download and unzip the previous release's zip, then run `node bench/bench.mjs --release --a <prev exe> --a-zip <prev zip> --b-zip out/make/zip/win32/x64/<zip> --label-a "<prev version> <sha7>" --label-b "<version> <sha7>"`, the labels naming the measured commits. Copy `bench-out/bench.json` to `docs/checks/<record>.bench.json` (the check record's file name with `.md` replaced) and paste `bench-out/bench.md`, a `### Bench:` table that ends with the `Result: pass` or `Result: miss` line, under `## Performance` in the record. On a miss (a measure over its limit, a measure with a limit that was not measured, or a measure worse than the previous release beyond its noise margin), write a `### Analysis` there (for each measure that missed: how much it moved, which change caused it, why, and whether it can be fixed) and stop for the owner's `Decision:`. `scripts/check-pr.mjs` fails the PR without the section, without the JSON or with one that has no previous release or fewer than 5 rounds, without a `Result:` line the JSON supports, on a miss without `Decision: go`, and on `Decision: no-go`.

- Merge with `--no-ff`.
- After a merge, always delete the merged `feature/*`, `release/*` or `hotfix/*` branch, local and remote (`git branch -d <branch>`, then `git push origin --delete <branch>`). Use `-d`, not `-D`: if git refuses, the branch is not merged. Delete the branch of a PR closed unmerged too. Never delete `main`, `develop` or `feature/pr-screenshots`.
- Never commit directly to `main` or `develop`, and never force-push them. Agents work on a `feature/*` branch.

### Pull requests and CI

Decided in [ADR 0007](docs/decisions/0007-github-actions-ci-cd-on-the-free-private-plan.md); merging on green in [ADR 0011](docs/decisions/0011-merge-labelled-prs-when-ci-is-green.md); e2e on CI only for PRs into `main` in [ADR 0017](docs/decisions/0017-e2e-on-ci-only-for-prs-into-main.md); the public repo in [ADR 0018](docs/decisions/0018-make-the-repository-public.md); `perf` in [ADR 0023](docs/decisions/0023-run-lightly-as-a-release-requirement.md). The repo is public on GitHub Free: Actions minutes on standard runners are free. A ruleset on `main` and `develop` blocks force-pushes and deletion, takes changes only through a PR merged with a merge commit, and requires `checks` to pass. Workflows may use only GitHub-owned actions pinned to a full commit SHA, plus SignPath's signing action in `release.yml` ([ADR 0024](docs/decisions/0024-sign-grace-exe-with-signpath-foundation.md)). Releases are immutable: once published, their tag and zip never change.

- `develop` is the default branch, so new PRs target it. Pick `main` for `release/*` and `hotfix/*` PRs.
- Open PRs as drafts while working. Mark ready for review when done.
- The repository's Actions settings list the one allowed SignPath SHA. A PR that changes the SHA of `SignPath/github-action-submit-signing-request` in `release.yml`, Dependabot's too, must be matched by the owner (the call needs admin) before the next tag, or that tag's release ends in a `startup_failure` that cannot be rerun ([#202](https://github.com/hpagniez/grace/issues/202)). The call replaces the list; check it with the same path and `gh api`: `gh api -X PUT repos/hpagniez/grace/actions/permissions/selected-actions -F github_owned_allowed=true -F verified_allowed=false -f 'patterns_allowed[]=SignPath/github-action-submit-signing-request@<full SHA>'`. Agents never change these settings: the PR body asks the owner.
- Every PR links an issue, whatever its type (fixes, tests, CI, docs and releases too): `Closes #N` when it finishes the issue, `Part of #N` when it does not. If no issue fits, open one first: a bug with the `bug` label and the sections of `.github/ISSUE_TEMPLATE/bug.yml`, other work like a feature issue. A fix found while working on another PR gets its own issue, and that PR names it too. For a bot PR (Dependabot, `vendor-updates`), whoever takes it opens the issue and adds the line to its body before it merges.
- A `Closes #N` in a commit message closes the issue when the commit lands on `develop`: write `Part of #N` in commits while the issue must stay open.
- Assign the PR to `hpagniez`. He cannot be asked for a review: the PRs are opened under his account.
- Commits and PR bodies end with the attribution lines `.claude/settings.json` gives, and say nothing else about the agent: no claude.ai session or project link, no `Claude-Session:` trailer, no "Requested by" line. Read a PR's body back after creating it and remove any.
- A PR that changes what the app shows carries before and after screenshots. They live on the never-merged branch `feature/pr-screenshots`, as `pr-<N>/<name>-before.png` and `-after.png`, linked by `https://raw.githubusercontent.com/hpagniez/grace/<sha>/...`. Take GPUI Grace's on Linux under Xvfb, or on [the owner's machine](#the-owners-machine) with his go.
- Never open a PR on another PR's branch: it gets no CI, and `merge-when-green` deletes its base branch, which closes it. Wait for the first to merge.
- A PR that changes `crates/` gets a Windows run by hand before it merges (`gh workflow run test.yml --ref <branch>`). `merge-when-green` does not wait for it, and a new dispatch cancels the one before: add the label only once the run is green.
- CI runs no e2e on PRs into `develop`. Before marking one ready, run `npm run e2e` locally if it changes more than `docs/**` and `*.md`, and paste the summary in the PR. PRs into `main` run e2e on CI once ready.
- Merge only when `checks` and `test` are green and `perf` has finished with success. The ruleset enforces only `checks`: `test` and `perf` never start on docs-only PRs, so it cannot require them.
- To have a PR into `develop` merge itself once CI is green, add the `merge-when-green` label, only when the user asks for the merge. It merges with a merge commit, deletes the branch, and closes the issues the PR names with `Closes #N` (GitHub ignores closing keywords in a merge made with the workflow token). PRs into `main` and PRs from forks are merged by hand.

| Workflow | When | What it runs |
|---|---|---|
| `checks` | Every PR to `develop` or `main`, except a `release/*` or `hotfix/*` merge back into `develop` (Linux); by hand, as a PR into `develop` | lint, typecheck, `npm audit --audit-level=high`, `scripts/check-pr.mjs` (commit headers, branch target, a release or hotfix check record's `## Public docs` and `## Performance` sections, a raised limit in `bench/limits.json`) |
| `test` | PRs that change more than `docs/**` and `*.md`, except a `release/*` or `hotfix/*` merge back into `develop` (Windows); by hand | Jobs on their own VMs ([ADR 0016](docs/decisions/0016-windows-tests-in-parallel-jobs.md)): `unit` runs `npm test` in two halves (half 2: the files listed in `test.yml`, half 1: every other file); `e2e` runs `npm run e2e` as two Playwright shards only on PRs into `main` that are not drafts, and on runs by hand (`gh workflow run test.yml --ref <branch>`); `rust` runs clippy, `cargo test`, `patches/check.sh` and GPUI Grace's e2e (on Windows only on PRs into `main` and by hand); `gpui-idle`, only by hand, runs `npm run bench:gpui` on Windows and fails when GPUI Grace wakes more than 5 times a second minimised |
| `perf` | PRs into `develop` or `main` that change more than `docs/**` and `*.md`, except a `release/*` or `hotfix/*` merge back into `develop` (Windows); by hand (the chosen ref against `develop`) | Builds the PR's base and its merge ref on one VM and runs `npm run bench` on both in PR mode, about 20 minutes. Posts the table as one PR comment, updated on every push, and in the job summary (summary only for forks and Dependabot). Fails only when it cannot build, measure or post, never on a worse number |
| `merge-when-green` | `checks`, `test` or `perf` finishes; a PR into `develop` is labelled or marked ready (Linux) | Merges a PR into `develop` from a branch of this repo (never a fork) that has the `merge-when-green` label and is not a draft, once `checks` and `test` passed and `perf` finished with success on its head commit (no `test` or `perf` run only for docs-only PRs). A back-merge from `release/*` or `hotfix/*` needs those runs from its PR into `main`, with every `e2e` step passed |
| `release` | Push of a `vX.Y.Z-rc.N` tag (a plain `vX.Y.Z` tag runs nothing); by hand | Tag must match `package.json`; `npm run make`; SignPath signs `Grace.exe` once a person approves, when the `SIGNPATH_ORGANIZATION_ID` variable is set ([ADR 0024](docs/decisions/0024-sign-grace-exe-with-signpath-foundation.md)); GitHub pre-release with the zip. By hand: build and test-sign, publish nothing |
| `vendor-updates` | Weekly, and by hand | Draft PR per newer Temurin 21 JRE or Karate 2.x (pins in `scripts/vendor.json`, [ADR 0012](docs/decisions/0012-weekly-draft-prs-for-the-bundled-jre-and-karate.md)); a Karate bump re-records the event fixtures. Mark it ready for review to run CI (unit tests, no e2e) |
| `cargo-updates` | Weekly (Tuesday), and by hand | PR `feature/bump-cargo-<date>` moving the GPUI set and Rust to the newest releases, the Windows patches rebased, and `Cargo.lock` (`scripts/bump-cargo.mjs`, [ADR 0032](docs/decisions/0032-keep-the-gpui-set-rust-and-cargo-lock-current.md)); then runs `checks`, `test` (Windows e2e and `gpui-idle` included) and `perf` on it by hand, and merges it once all three pass ([ADR 0033](docs/decisions/0033-merge-the-weekly-cargo-bump-prs-on-green.md)). A failed run leaves the PR open with a comment, and the run fails: fix the branch, run the failed workflow again by hand, then merge as any PR. A patch that no longer applies gets an issue, and the rest still moves |

Dependabot opens grouped update PRs into `develop` monthly, plus security PRs when an alert appears. It does not see the JRE and Karate pins in `scripts/vendor.json`: `vendor-updates` bumps them (`npm run vendor:bump -- <jre|karate>` by hand). The ECJ pin there, a build tool only, is bumped by hand ([ADR 0025](docs/decisions/0025-compile-the-debug-listener-with-ecj-on-the-bundled-jre.md)). Nor does it watch Cargo: `cargo-updates` moves the GPUI set, Rust and `Cargo.lock` ([ADR 0032](docs/decisions/0032-keep-the-gpui-set-rust-and-cargo-lock-current.md)).

## Subagents

When the main agent dispatches subagents (implementers, reviewers, explorers), it picks the model per task. Never leave the model unset: an unset model inherits the main agent's model.

| Model | Use for |
|---|---|
| Haiku 5.5 (`haiku`) | Mechanical, bounded work: locating code, read-only lookups, renames, re-reviewing a single small fix. |
| Sonnet 5.5 (`sonnet`) | Default for clear tasks: implementing a plan task with a precise brief, writing tests, spec and quality reviews of one task. |
| Opus 5.5 (`opus`) | Hard judgment: design and architecture, ambiguous specs, cross-cutting changes, debugging an unknown cause, whole-branch final reviews. |

- Always set reasoning effort to extra high (`effort: "xhigh"`), whatever the model.
- Pick the cheapest model that can do the task well. Escalate to the next model if a subagent fails or returns weak work; do not retry on the same model.

## Stack

Electron Grace, the app that ships until the GPUI cut-over ([GPUI rules](#gpui-rules)): Electron + SolidJS 2 RC + Tailwind CSS v4, packaged with Electron Forge. Pin exact versions (no `^`) and upgrade the Solid set together.

- Keep `contextIsolation` on, `nodeIntegration` off, `sandbox` on. The renderer reaches Node only through the typed preload API.
- Forge dev reloads the renderer once after start: main-process code that runs per page load must be idempotent.
- Java and Karate are bundled, not taken from the user's machine, decided in [ADR 0006](docs/decisions/0006-bundle-jre-and-karate-jar.md). Run Karate as `java -cp "karate.jar;<project classpath roots>" io.karatelabs.Main run ...`, not `java -jar`, so `classpath:` resolves in Maven and Gradle layouts.
- End-to-end tests: Playwright Test (`@playwright/test` 1.64.1) with `_electron`, decided in [ADR 0004](docs/decisions/0004-playwright-for-end-to-end-tests.md). Launch the built app (`electron-forge package`, then `_electron.launch({ args: ['.'] })`), not the packaged exe: its `EnableNodeCliInspectArguments: false` fuse blocks Playwright; keep that fuse off. Run `npx electron --version` first so the binary download does not hit the test timeout. Reference: [Playwright spike](docs/research/2026-10-24-playwright-spike/README.md).
- Unit tests: Vitest (`vitest` 5.0.5, `jsdom` 30.1.3, `@solidjs/testing-library` 1.0.0-rc.2), decided in [ADR 0005](docs/decisions/0005-vitest-for-unit-tests.md). Copy the [Vitest spike](docs/research/2026-10-24-vitest-spike/README.md) config: `*.test.ts` runs in Node, `*.test.tsx` in jsdom. The jsdom project needs `resolve.conditions: ['browser', 'development']` and `test.server.deps.inline: [/solid/]`, or Solid loads its server build. Testing-library 0.8 is Solid 1; do not use it.

### Solid 2 rules

Most training data is Solid 1. Do not write Solid 1 code:

- Import the renderer from `@solidjs/web`, not `solid-js/web`. Stores come from `solid-js`, not `solid-js/store`. `jsxImportSource` is `@solidjs/web`. Vite plugin is `@solidjs/vite-plugin`, not `vite-plugin-solid`.
- No `createResource`: use an async `createMemo`. `Suspense` → `Loading`, `ErrorBoundary` → `Errored`.
- `onMount` → `onSettled`. `createEffect` takes a tracked compute function and an untracked apply function. No `on`, no `batch` (use `flush()`).
- A write that feeds an async memo is held until the memo resolves (dev warning `SILENT_HOLD`). Show stale values with `isPending`, `latest` or `<Loading on>`.
- Store setters receive a draft; no path setters, no `produce`. `mergeProps` → `merge`, `splitProps` → `omit`, `unwrap` → `snapshot`.
- `Index` → `<For keyed={false}>`. Default (keyed) `For` children get `(item, index)` with `item` the raw value and `index` an accessor. With `keyed={false}` they get `item` as an accessor and `index` as a plain number. `Context.Provider` → the context itself.
- No `use:` directives, `classList`, `on:`, `attr:`, `bool:`. Use `onClick`, not `onclick`.

Source: https://v2.solidjs.com/migration/from-solid-1

### GPUI rules

Grace is moving to Rust and GPUI ([ADR 0031](docs/decisions/0031-move-grace-to-rust-and-gpui.md)); the [migration guide](docs/specs/2026-11-08-gpui-migration.md) holds the phases, and the [parity checklist](docs/specs/2026-11-08-gpui-migration-parity.md) what each must prove. Until the cut-over the Electron app above ships, and GPUI Grace lives beside it in a Cargo workspace at the root: `crates/grace-core` (no GPUI, plain `cargo test`) and `crates/grace` (the app, built as `Grace.exe`).

- A port is not a transcription (guide R3.6): where Rust or GPUI does better than the TypeScript, do it that way, keeping what people see, the data on disk and the Karate command line at parity. Record each such change in the phase plan's Redesigns table, in the PR that makes it. Tick a parity row in the PR that ports it. Only the owner accepts an `n/a` on a row someone using Grace would notice (R3.3).
- A phase's issue closes only when all its work is done: every PR merged, every row ticked or handed on by its Proof text, its exit check passed.
- Stock GPUI and GPUI Kit first: their public API and stock behaviour. A new patch or a replaced widget states its reason in the PR and in the Redesigns table. Drop a patch as soon as upstream has the fix.
- Never post on Zed's repository: no issue, PR or comment. Follow upstream from this repository's issues.
- Before pushing Rust: `cargo fmt --all --check` after the last edit, clippy and `cargo test --workspace` as in [Commands](#commands), and `sh scripts/e2e-gpui.sh`.
- Pins: `gpui-pre` and GPUI Kit (`gpui-component`, `gpui-base`, `gpui-kit-assets`) exactly in the workspace's `Cargo.toml`, the toolchain in `rust-toolchain.toml`. They follow the newest releases ([ADR 0032](docs/decisions/0032-keep-the-gpui-set-rust-and-cargo-lock-current.md)): `cargo-updates` opens a bump PR every week and merges it once its tests pass, and `npm run cargo:bump` makes the same change in the working tree. GPUI and GPUI Kit move together, the patches rebased ([patches/](patches/README.md)). Never edit `patches/gpui-pre-windows/` by hand: change the `.patch`, then the copy; CI's `patches/check.sh` fails when they differ.
- Linux needs `libxkbcommon-x11-dev`, `libxcb1-dev`, `libfontconfig-dev` and `libfreetype-dev` to build; run the app under Xvfb on a machine without a screen, with a Vulkan driver (`mesa-vulkan-drivers` gives the software one on a machine without a GPU, such as CI). The Windows build needs nothing beside the exe.
- Colours only through `grace::theme` (`Theme::global(cx).tokens`, `Theme::color`): no `rgb(`, `rgba(`, `hsla(` or `Hsla {` elsewhere; `theme::tests::colours_come_only_from_the_theme` enforces it. Until the cut-over the tokens must equal `src/index.css`'s, which the theme's tests check: change both together.
- GPUI Kit reads its own theme: set it through `gpui_component::Theme::update` (as `theme::apply` does), never `Theme::global_mut`, whose edits do not reach the buttons.
- Icons only through `grace::assets::Icon`, the same Lucide files as Electron Grace's `lucide` version; a new icon in `src/renderer` fails `has_the_icons_of_electron_grace` until it is added to `crates/grace/assets/icons/`. GPUI Kit's own `IconName` holds only the icons its controls use.
- Tooltips only through `grace::tooltip`: `tooltip(id, text)` around the trigger (`.focusable(..)` for a control that takes focus, `.side(..)` in a row), or `cut(id, text)` for a line that may be cut. A tooltip shows after 500 ms on hover or at once on keyboard focus, one at a time, and Escape or a click closes it before anything else. The window's root goes through `a11y::tab_order`, so Tab moves the focus.
- Statuses only through `elements::status_mark`, buttons through `elements::button` and `icon_button` (GPUI Kit's `Button`, which brings focus, the ring and the accessible name). Turn a button off with `elements::Off::off`, not GPUI Kit's `disabled`, make a checkbox with `elements::checkbox` (its `off` turns it off) and a radio button with `elements::radio`, and a text box with `elements::input`: they set the cursor a browser would (the hand, the not-allowed sign, the text cursor), and the checkbox and radio are drawn as a browser draws Electron Grace's.
- Accessibility is set by hand: an element reaches the tree only with both `.id(...)` and `.role(...)`, then `.aria_label(...)` and its states. A `Role::Label` reads its name from `.aria_value(...)`, not `.aria_label(...)`. `div` has no live region: wrap it in `a11y::live`. On Linux the pinned AccessKit adapter drops a button's disabled state and a row's level and expanded state.
- Headless `#[gpui::test]` windows build no accessibility tree: GPUI fills it only when an assistive technology connects. Test the tree through the e2e driver, `crates/grace-e2e`: find nodes by role and name, wait with `App::wait_for`, never sleep. Its tests are `#[ignore]`d in `cargo test` and run by `scripts/e2e-gpui.sh`, which builds Grace with the `e2e` feature. Test hooks live behind that feature only (`grace::e2e`); the release build has none. On Linux the driver moves the pointer and types through `xdotool`, in the window's coordinates. A tree read can be torn mid-update: wait for all the final values together. A cached view (`.cached(..)`) adds no accessibility nodes: cache only while `!window.is_a11y_active()`.
- Through AT-SPI, AccessKit's `Application` role reads as `Embedded`, and a node answers `GetRole` but not `GetRoleName`. Nor can it read the text in a GPUI Kit `Input` (a combo box or spin button): on Linux an e2e test checks what the box did, and `App::wait_for_text` reads the text only on Windows.
- The bench reads GPUI Grace through `grace::probe`: with `GRACE_PROBE` set it prints `PROBE <name> <unix ms>` on stderr (`bench/gpui.mjs`). It is in the release build, the one the bench measures, and costs one look at the environment when off. A new probe goes through `probe::mark`, never its own `eprintln!`.
- Idle ([ADR 0030](docs/decisions/0030-no-wake-ups-while-hidden-and-an-idle-cpu-budget.md)): no timer, `cx.spawn` loop or `with_animation` that runs while no run is going. An animation runs only while its state lasts (a running test's mark), and not with reduced motion.

### UI conventions

Spec: [docs/specs/2026-10-29-premium-ui.md](docs/specs/2026-10-29-premium-ui.md).

- Colours only through the semantic tokens in `src/index.css`. Tailwind's default palette is removed: no raw palette class (`zinc-500`), no `dark:`, no `title=`. `src/renderer/palette.test.ts` enforces it.
- The two themes differ only in colours: the dark block of `src/index.css` may override only colour tokens, shadows and the backdrop, never a size, spacing or radius. `src/renderer/tokens.test.ts` enforces it.
- Shared class strings live in `src/renderer/buttons.ts` and `src/renderer/ui.ts`. Reuse them.
- Icons only through `Icon` with `lucide` icon data. Statuses only through `StatusMark`.
- Tooltips only through `Tooltip`, or `Cut` for text that may be cut. In a row, set `side` so a one-line tooltip stays on its row; a click on any tooltip closes it.
- A dialog keeps its content through its 100 ms exit (`--duration-fast`), then drops it.

## Karate integration rules

Full detail: [docs/inventory/karate-core.md §19](docs/inventory/karate-core.md#19-integration-surface-for-a-gui). Where they disagree, the [CLI spike](docs/research/2026-10-24-karate-cli-spike/README.md) wins: it ran Karate 2.1.4 for real. Its `samples/` are real event-stream fixtures.

- Follow the Karate CLI first: for every feature and every kind of project, do what the Karate CLI does, and build Grace's own way only where it has none ([ADR 0026](docs/decisions/0026-follow-the-karate-cli-first.md)).
- Spawn from the Electron main process only, without a shell: `java -Dstdout.encoding=UTF-8 -Dstderr.encoding=UTF-8 -cp "<karate.jar>;<project classpath roots>" io.karatelabs.Main run --no-color -w <root> -o <absolute> -f html,karate:jsonl ...` with `cwd` = project root. Relative paths resolve from the process cwd, not `-w`.
- Remove `KARATE_OPTIONS`, `JAVA_TOOL_OPTIONS`, `JDK_JAVA_OPTIONS` and `_JAVA_OPTIONS` from the child environment; they override the app's values.
- Tail `<output>/karate-json/karate-events.jsonl` for live progress. There is no `PROGRESS` event: count depth-0 `SCENARIO_EXIT` against the dry-run total. Filter `callDepth == 0` for the test tree. Key nodes by feature + `refId` (slugs collide). Check `skipped` before `passed` (dry run sets both). Ignore `OUTLINE_ENTER.numExamples`.
- `-D` is dry run, `-p` is the pom file, `-n` is the scenario-name filter. Do not invent flags; check `RunCommand.java`.
- Tags: `@a`, `~@a`, `@a,@b` (OR), repeated `-t` (AND), or Karate JS `anyOf('@a','@b') && not('@c')`. `(@a or @b) and not @c` fails with exit 3.
- Exit codes: `0` pass, `1` test failures (also after `taskkill`), `2` usage error, `3` parse or runtime error (one broken feature aborts the whole run). A missing path, unknown `-e` or unmatched `-n` exit `0` silently: preflight with a dry run and warn on 0 scenarios.
- On Windows, cancel with `taskkill /PID <pid> /T /F` and record "cancelled" in the app. `child.kill()` leaves the JVM running when spawned through a shell or launcher.
- HTTP request/response detail exists only as text in step logs inside `FEATURE_EXIT`. OSS Karate v2 has no debug server. Live step and HTTP events need our own Java `RunListener` passed with `--listener`.
- The step debugger is that listener: `listener/grace/DebugListener.java`, on the classpath and passed with `--listener grace.DebugListener` only in a debug run, on one thread. It talks to the main process over a loopback socket (spec `docs/specs/2026-11-02-step-debugger.md`). It may use only Karate's public listener interface, never `karate.debug.port` or the paid `io.karatelabs.debug` classes.
- The HTTP inspector (F11) parses that step-log text in `src/main/karate/http.ts`; the text is Karate's, not a contract. After any Karate version change, run `npm run fixtures` to re-record `test/fixtures/events/` (including `run-http*.jsonl`), check the recordings still hold the http sample project's calls (spec `docs/specs/2026-10-25-http-inspector.md`, Acceptance criteria), and keep `src/main/karate/http.test.ts` green. The `vendor-updates` bump PR re-records them on its own.
