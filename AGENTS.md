# AGENTS.md

karate-kid is a free, open-source desktop test-suite runner for [Karate](https://docs.karatelabs.io/) v2. It wraps the open-source Karate CLI in an Electron GUI. Karate's own GUI runners are paid. This repo is built mostly by AI agents.

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

Never run `npm test` and `npm run e2e` at the same time: their Java leak checks scan every `java.exe` under `vendor/jre` and see each other's processes.

## Docs map

| Path | What it holds |
|---|---|
| [docs/features.md](docs/features.md) | Backlog: what the app does, priority, status, how it gets the data from Karate |
| [docs/inventory/karate-core.md](docs/inventory/karate-core.md) | Every open-source Karate v2 feature, tagged by GUI use; §19 = how the app drives Karate |
| [docs/inventory/commercial.md](docs/inventory/commercial.md) | What Karate Labs' paid tools offer, and the gaps |
| [docs/research/](docs/research/) | Dated research notes with sources |
| [docs/decisions/](docs/decisions/) | Decision records (MADR 4.0 minimal) |
| `docs/specs/` | Design specs, one per feature, written just before building it |
| `docs/checks/` | Dated check records of the packaged app: what was run, on which machine, the result, and what still needs a person |
| `docs/plans/` | Implementation plans, one per spec, task by task (overrides the superpowers default `docs/superpowers/plans/`) |

## Doc rules

- Design specs go in `docs/specs/YYYY-MM-DD-<topic>.md` (this overrides the superpowers default `docs/superpowers/specs/`). Write requirements, then acceptance criteria in Gherkin.
- When a spec is written or a feature ships, update its row in `docs/features.md` in the same change.
- A choice that is hard to reverse (dependency, architecture, file format) gets a new `docs/decisions/NNNN-title.md`. Never edit an accepted record; supersede it.
- Do not copy code, text or assets from Karate Labs' paid products. Learn the feature set only.

## Git workflow

Decided in [ADR 0003](docs/decisions/0003-conventional-commits-and-git-flow.md).

### Commits

[Conventional Commits 1.0.0](https://www.conventionalcommits.org/en/v1.0.0/): `<type>(<scope>)<!>: <subject>`.

- Types: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`.
- Scope is optional: the area touched (`docs`, `runner`, `ui`, `ipc`, `deps`). Reuse an existing scope before inventing one.
- Subject: imperative, lower case, no trailing period, 72 characters max.
- Breaking change: `!` after the scope, plus a `BREAKING CHANGE:` footer.
- One logical change per commit. A spec or feature commit includes its `docs/features.md` row update.

### Branches (git flow)

| Branch | Cut from | Merges into | Notes |
|---|---|---|---|
| `main` | | | Released code only. Tagged `vX.Y.Z` (SemVer). |
| `develop` | `main` | | Integration branch. |
| `feature/<topic>` | `develop` | `develop` | Via PR. Kebab-case topic. |
| `release/X.Y.Z` | `develop` | `main` and `develop` | Version bump and fixes only. Tag on `main`. |
| `hotfix/X.Y.Z` | `main` | `main` and `develop` | Urgent fix to a released version. |

- Merge with `--no-ff`.
- After a merge, always delete the merged `feature/*`, `release/*` or `hotfix/*` branch, local and remote (`git branch -d <branch>`, then `git push origin --delete <branch>`). Use `-d`, not `-D`: if git refuses, the branch is not merged. Never delete `main` or `develop`.
- Never commit directly to `main` or `develop`, and never force-push them. Agents work on a `feature/*` branch.

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
