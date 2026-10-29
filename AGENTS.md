# AGENTS.md

Grace is a free, open-source desktop test-suite runner for [Karate](https://docs.karatelabs.io/) v2. It wraps the open-source Karate CLI in an Electron GUI. Karate's own GUI runners are paid. This repo is built mostly by AI agents. The app was called karate-kid until [ADR 0013](docs/decisions/0013-rename-the-app-to-grace.md); dated specs, plans, checks and research notes keep that name. The repository is `hpagniez/grace`.

## Commands

Windows 11, Node 24. Run `npm install`, then `npm run vendor` once (downloads the bundled JRE and Karate jar into `vendor/`).

| Command | What it does |
|---|---|
| `npm start` | Run the app in dev mode |
| `npm test` | Unit tests (Vitest) |
| `npm run e2e` | Package the app, then run the Playwright end-to-end tests |
| `npm run typecheck` | `tsc --noEmit` |
| `npm run lint` | oxlint and oxfmt check |
| `npm run make` | Build the Windows zip in `out/make/zip/win32/x64/` |

Never run `npm test` and `npm run e2e` at the same time: the e2e Java leak checks scan every `java.exe` under `vendor/jre` and see the unit tests' processes. Unit test files run in parallel; their leak checks see only their own project's Java ([ADR 0020](docs/decisions/0020-run-unit-test-files-in-parallel.md)).

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

- Design specs go in `docs/specs/YYYY-MM-DD-<topic>.md` (this overrides the superpowers default `docs/superpowers/specs/`). Write requirements, then acceptance criteria in Gherkin.
- Every feature has an issue with a priority label. Its body holds the feature, a `Karate surface:` line and a `Spec:` line. New features get only an issue number, never a new F-ID; cite them as `#N`. When a spec is written, point the issue's `Spec:` line at the file on `develop`. `gh issue edit <N> --body-file -` replaces the whole body, so start from the current one (`gh issue view <N> --json body --jq .body`), change only that line, and pipe the full body back. A PR that builds a feature says `Closes #N`; a spec-only PR, or one of several, says `Part of #N`. The project's workflows move the status. For a feature built over several PRs, move its card to `building` when its first building PR (not a spec-only PR) opens: `gh project item-add 2 --owner hpagniez --url <issue URL> --format json --jq .id` gives the item ID, then `gh project item-edit --id <item ID> --project-id PVT_kwHOAka2MM4BpJIV --field-id PVTSSF_lAHOAka2MM4BpJIVzhnmDmA --single-select-option-id 47fc9ee4`. Both calls need a token with the `project` scope (`gh auth refresh -h github.com -s project`) ([ADR 0014](docs/decisions/0014-backlog-in-github-issues-and-a-project.md)).
- A choice that is hard to reverse (dependency, architecture, file format) gets a new `docs/decisions/NNNN-title.md`. Never edit an accepted record; supersede it.
- Do not copy code, text or assets from Karate Labs' paid products. Learn the feature set only.

## No backward compatibility

The app is still in development. Never write code that keeps older versions working: no migration of settings, run history or data folders, no fallback to an old name, key, environment variable or file format, no deprecated alias. Change the name or format, and update every place that uses it. Data from an older version may be lost: say so in the commit's `BREAKING CHANGE:` footer.

This rule ends at the first stable release, `1.0.0`. From then on, people get new versions through the in-app updater, so every version must read the data of every earlier stable version. A rollback is not promised to read newer data. The updater's own contract (release tag, zip name and layout, `update-result.json`, backup layout, the `updates` settings key) never changes without a new decision record, pre-releases included ([ADR 0019](docs/decisions/0019-update-the-zip-install-from-github-releases.md)).

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

- Merge with `--no-ff`.
- After a merge, always delete the merged `feature/*`, `release/*` or `hotfix/*` branch, local and remote (`git branch -d <branch>`, then `git push origin --delete <branch>`). Use `-d`, not `-D`: if git refuses, the branch is not merged. Never delete `main` or `develop`.
- Never commit directly to `main` or `develop`, and never force-push them. Agents work on a `feature/*` branch.

### Pull requests and CI

Decided in [ADR 0007](docs/decisions/0007-github-actions-ci-cd-on-the-free-private-plan.md); merging on green in [ADR 0011](docs/decisions/0011-merge-labelled-prs-when-ci-is-green.md); e2e on CI only for PRs into `main` in [ADR 0017](docs/decisions/0017-e2e-on-ci-only-for-prs-into-main.md); the public repo in [ADR 0018](docs/decisions/0018-make-the-repository-public.md). The repo is public on GitHub Free: Actions minutes on standard runners are free. A ruleset on `main` and `develop` blocks force-pushes and deletion, and takes changes only through a PR merged with a merge commit. Workflows may use only GitHub-owned actions pinned to a full commit SHA. Releases are immutable: once published, their tag and zip never change.

- `develop` is the default branch, so new PRs target it. Pick `main` for `release/*` and `hotfix/*` PRs.
- Open PRs as drafts while working. Mark ready for review when done.
- CI runs no e2e on PRs into `develop`. Before marking one ready, run `npm run e2e` locally if it changes more than `docs/**` and `*.md`, and paste the summary in the PR. PRs into `main` run e2e on CI once ready.
- Merge only when `checks` and `test` are green. Nothing enforces it.
- To have a PR into `develop` merge itself once CI is green, add the `merge-when-green` label, only when the user asks for the merge. It merges with a merge commit, deletes the branch, and closes the issues the PR names with `Closes #N` (GitHub ignores closing keywords in a merge made with the workflow token). PRs into `main` and PRs from forks are merged by hand.

| Workflow | When | What it runs |
|---|---|---|
| `checks` | Every PR to `develop` or `main`, except a `release/*` or `hotfix/*` merge back into `develop` (Linux) | lint, typecheck, `npm audit --audit-level=high`, `scripts/check-pr.mjs` (commit headers, branch target) |
| `test` | PRs that change more than `docs/**` and `*.md`, except a `release/*` or `hotfix/*` merge back into `develop` (Windows); by hand | Jobs on their own VMs ([ADR 0016](docs/decisions/0016-windows-tests-in-parallel-jobs.md)): `unit` runs `npm test` in two halves (half 2: the files listed in `test.yml`, half 1: every other file); `e2e` runs `npm run e2e` as two Playwright shards only on PRs into `main` that are not drafts, and on runs by hand (`gh workflow run test.yml --ref <branch>`) |
| `merge-when-green` | `checks` or `test` finishes; a PR into `develop` is labelled or marked ready (Linux) | Merges a PR into `develop` from a branch of this repo (never a fork) that has the `merge-when-green` label and is not a draft, once `checks` and `test` passed on its head commit (no `test` run only for docs-only PRs). A back-merge from `release/*` or `hotfix/*` needs those runs from its PR into `main`, with every `e2e` step passed |
| `release` | Push of a `vX.Y.Z-rc.N` tag (a plain `vX.Y.Z` tag runs nothing) | Tag must match `package.json`; `npm run make`; GitHub pre-release with the zip |
| `vendor-updates` | Weekly, and by hand | Draft PR per newer Temurin 21 JRE or Karate 2.x (pins in `scripts/vendor.json`, [ADR 0012](docs/decisions/0012-weekly-draft-prs-for-the-bundled-jre-and-karate.md)); a Karate bump re-records the event fixtures. Mark it ready for review to run CI (unit tests, no e2e) |

Dependabot opens grouped update PRs into `develop` monthly, plus security PRs when an alert appears. It does not see the JRE and Karate pins in `scripts/vendor.json`: `vendor-updates` bumps them (`npm run vendor:bump -- <jre|karate>` by hand).

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

Decided in [ADR 0001](docs/decisions/0001-solid-router-spa-without-tanstack-start.md): Electron + SolidJS 2 RC + TanStack Router (no TanStack Start) + Tailwind CSS v4, packaged with Electron Forge. Pin exact versions (no `^`) and upgrade the Solid set together. The [stack spike](docs/research/2026-10-24-stack-spike/README.md) proved this set builds and packages on Windows; copy its configs when scaffolding.

- Scaffold with `npx create-electron-app@8.0.1 init <dir> --template=vite --typescript`, then strip the `^` ranges it writes.
- `@kobalte/core` alpha needs `"overrides": { "@kobalte/core": { "solid-js": "$solid-js", "@solidjs/web": "$@solidjs/web" } }`.
- `npm ls` exits 1 (`ELSPROBLEMS`) because `@tanstack/solid-router@2.0.0-rc.9` declares unused Solid-1 peers. Expected; not a broken install.
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

## Karate integration rules

Full detail: [docs/inventory/karate-core.md §19](docs/inventory/karate-core.md#19-integration-surface-for-a-gui). Where they disagree, the [CLI spike](docs/research/2026-10-24-karate-cli-spike/README.md) wins: it ran Karate 2.1.4 for real. Its `samples/` are real event-stream fixtures.

- Spawn from the Electron main process only, without a shell: `java -Dstdout.encoding=UTF-8 -Dstderr.encoding=UTF-8 -cp "<karate.jar>;<project classpath roots>" io.karatelabs.Main run --no-color -w <root> -o <absolute> -f html,karate:jsonl ...` with `cwd` = project root. Relative paths resolve from the process cwd, not `-w`.
- Remove `KARATE_OPTIONS`, `JAVA_TOOL_OPTIONS`, `JDK_JAVA_OPTIONS` and `_JAVA_OPTIONS` from the child environment; they override the app's values.
- Tail `<output>/karate-json/karate-events.jsonl` for live progress. There is no `PROGRESS` event: count depth-0 `SCENARIO_EXIT` against the dry-run total. Filter `callDepth == 0` for the test tree. Key nodes by feature + `refId` (slugs collide). Check `skipped` before `passed` (dry run sets both). Ignore `OUTLINE_ENTER.numExamples`.
- `-D` is dry run, `-p` is the pom file, `-n` is the scenario-name filter. Do not invent flags; check `RunCommand.java`.
- Tags: `@a`, `~@a`, `@a,@b` (OR), repeated `-t` (AND), or Karate JS `anyOf('@a','@b') && not('@c')`. `(@a or @b) and not @c` fails with exit 3.
- Exit codes: `0` pass, `1` test failures (also after `taskkill`), `2` usage error, `3` parse or runtime error (one broken feature aborts the whole run). A missing path, unknown `-e` or unmatched `-n` exit `0` silently: preflight with a dry run and warn on 0 scenarios.
- On Windows, cancel with `taskkill /PID <pid> /T /F` and record "cancelled" in the app. `child.kill()` leaves the JVM running when spawned through a shell or launcher.
- HTTP request/response detail exists only as text in step logs inside `FEATURE_EXIT`. OSS Karate v2 has no debug server. Live step and HTTP events need our own Java `RunListener` passed with `--listener`.
- The HTTP inspector (F11) parses that step-log text in `src/main/karate/http.ts`; the text is Karate's, not a contract. After any Karate version change, run `npm run fixtures` to re-record `test/fixtures/events/` (including `run-http*.jsonl`), check the recordings still hold the http sample project's calls (spec `docs/specs/2026-10-25-http-inspector.md`, Acceptance criteria), and keep `src/main/karate/http.test.ts` green. The `vendor-updates` bump PR re-records them on its own.
