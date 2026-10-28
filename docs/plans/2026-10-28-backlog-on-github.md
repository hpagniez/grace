# Backlog on GitHub Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move the 45 rows of `docs/features.md` to GitHub issues on a project board, and change the repo's rules to match.

**Architecture:** A one-off Node script parses `docs/features.md`, creates one issue per row through `gh`, closes the done ones and rewrites `docs/features.md` as a frozen map of old IDs to issues. The project and its workflows are set up by hand in the web UI. The script is deleted once it has run; git history keeps it.

**Tech Stack:** Node 24 ES modules, `gh` 2.96, Vitest 5.

**Spec:** [ADR 0014](../decisions/0014-backlog-in-github-issues-and-a-project.md).

## Global Constraints

- Branch `feature/backlog-on-github`. Conventional Commits, AGENTS.md rules. No new dependency.
- Repo `hpagniez/grace`. Labels `must`, `should`, `could`. Project `Grace`, owned by `hpagniez`, `Status` options `idea`, `building`, `done`.
- Links in issue bodies are absolute: `https://github.com/hpagniez/grace/blob/develop/docs/<path>`.
- Only Task 2 writes to GitHub, and only the maintainer runs it.
- Dated specs, plans, checks, research notes and accepted decision records keep their F-IDs and `features.md` links.
- Run one unit file with `npx vitest run <file>`.

## Review Focus

1. **Relative links** in row cells and notes (`specs/…`, `inventory/…#…`, `../…`): they resolve against `docs/` in the repo but 404 in an issue. Test: Task 1 `absoluteLinks`.
2. **In-page anchors** such as `[notes](#f11-http-inspector)`: the notes sit in the same issue body, so the link must become plain text. Test: Task 1 `absoluteLinks`.
3. **A second run of `--apply`** after a failure halfway: it must not create duplicates. Test: Task 1 `migrate` skips rows whose `Old backlog ID:` marker is already in an issue.
4. **Titles cut from the Feature cell** at the first `": "`: one with a backtick or over 80 characters reads badly in the issue list. Test: Task 1 checks every title of the real file.
5. **Workflow race:** closing an issue seconds after it was added can let "Item added → idea" land after "Item closed → done". The script creates every issue first, then closes the done ones, and Task 2 Step 6 counts the board.

---

### Task 1: Migration script

**Files:**
- Create: `scripts/backlog-to-issues.mjs`, `scripts/backlog-to-issues.test.ts`

**Interfaces:**
- Produces, all exported from `scripts/backlog-to-issues.mjs`:
  - `parseBacklog(markdown: string): Row[]`, where `Row = { id: string, title: string, feature: string, priority: "must" | "should" | "could", status: string, surface: string, spec: string, notes: string }`. `title` is the Feature cell up to its first `": "`, or the whole cell. `notes` is the body of the `### <id> …` section under "Idea notes", or `""`.
  - `absoluteLinks(md: string): string`: `[t](#anchor)` becomes `t`; `[t](path)` resolves `path` with `new URL(path, "https://github.com/hpagniez/grace/blob/develop/docs/")`; `http(s)` links are unchanged.
  - `issueBody(row: Row): string`: the feature, `**Karate surface:** …`, `**Spec:** …` (`none` for `—`), the notes under `## Notes` when present, and a last line `Old backlog ID: <id>`. Every part goes through `absoluteLinks`.
  - `frozenBacklog(rows: Row[], numbers: Map<string, number>): string`: the new `docs/features.md`.
  - `migrate({ apply: boolean }, run = execRun): Map<string, number>`: `run(cmd, args, input?)` returns stdout, as `execRun` does in `scripts/bump-pr.mjs`.
- The script runs as `node scripts/backlog-to-issues.mjs [--apply]`, guarded like `scripts/bump-pr.mjs:204`.

- [ ] **Step 1: Write the failing tests** in `scripts/backlog-to-issues.test.ts`:
  - `parseBacklog` on the real `docs/features.md`: 45 rows, ids `F01` to `F45` in order, 23 with status `done`, 22 `idea`; `F11` has title `HTTP inspector`, `F02` has title `Run a feature, scenario or single example row`, `F44`'s notes start with `Measurements, causes and estimates`, `F01`'s notes are `""`; no title contains a backtick or exceeds 80 characters.
  - `absoluteLinks("[spec](specs/a.md)")` is `[spec](https://github.com/hpagniez/grace/blob/develop/docs/specs/a.md)`; `"[x](inventory/k.md#19-a)"` keeps `#19-a`; `"[x](../AGENTS.md)"` is `[x](https://github.com/hpagniez/grace/blob/develop/AGENTS.md)`; `"See [notes](#f11-http-inspector)"` is `See notes`; `"[d](https://docs.karatelabs.io/)"` is unchanged.
  - `issueBody` of the parsed `F11` contains `**Karate surface:**`, `**Spec:** [spec](https://github.com/hpagniez/grace/blob/develop/docs/specs/2026-10-25-http-inspector.md)`, `## Notes`, ends with `Old backlog ID: F11`, and contains no `](specs/` or `](#`.
  - `migrate({ apply: false }, fake)`: `fake` records calls; no call is `issue create`, `issue close` or `label create`.
  - `migrate({ apply: true }, fake)` with `issue list` returning `[]` and `issue create` returning `https://github.com/hpagniez/grace/issues/<41 + i>`: 3 `label create` calls with `--force`; 45 `issue create` calls with `--repo hpagniez/grace`, `--label <priority>`, `--project Grace` and `--body-file -`; then 23 `issue close <n> --reason completed`, all after the last create; the returned map has `F01 → 41`.
  - `migrate({ apply: true }, fake)` with `issue list` returning one issue whose body ends with `Old backlog ID: F01`: no create for `F01`, and the map keeps that issue's number.
  - `frozenBacklog` output contains `| F44 | [#84](https://github.com/hpagniez/grace/issues/84) Faster CI without dropping a test |` for `F44 → 84`, and links ADR 0014.
- [ ] **Step 2:** Run `npx vitest run scripts/backlog-to-issues.test.ts`. Expected: FAIL, the module does not exist.
- [ ] **Step 3: Implement** the Interfaces above. `migrate` reads existing markers once with `gh issue list --repo hpagniez/grace --state all --limit 1000 --json number,body`, creates the labels, creates the missing issues in ID order, then closes the done rows. It prints `F01 #41` per row as it goes. With `--apply` the CLI also writes `frozenBacklog(...)` to `docs/features.md`. `frozenBacklog` writes: the `# Features` heading; one paragraph saying the backlog moved to GitHub issues on 2026-10-28 ([ADR 0014](decisions/0014-backlog-in-github-issues-and-a-project.md)) and that the table maps the old IDs, which older specs, plans and notes use, to their issues; then the table `| ID | Issue |`.
- [ ] **Step 4:** Run `npx vitest run scripts/backlog-to-issues.test.ts`, then `npm run lint` and `npm run typecheck`. Expected: PASS.
- [ ] **Step 5: Commit** `chore: add the one-off backlog to issues migration script`.

### Task 2: Project setup and migration run (maintainer only)

Writes to GitHub, cannot be undone without deleting issues, and needs the web UI. An agent stops here and hands these steps to the maintainer.

- [ ] **Step 1:** Grant the token the project scope: `gh auth refresh -h github.com -s project`, approved in the browser. Check: `gh api -i user` prints an `X-Oauth-Scopes` header that contains `project`. The saved login is labelled `Tusquito` but is the `hpagniez` account, which owns the project.
- [ ] **Step 2:** `gh project create --owner hpagniez --title Grace`, then `gh project link <number> --owner hpagniez --repo hpagniez/grace`. Note the project URL `https://github.com/users/hpagniez/projects/<number>`.
- [ ] **Step 3: In the web UI**, rename the `Status` options `Todo` → `idea`, `In Progress` → `building`, `Done` → `done`. Under Workflows, turn on: Item added to project → `idea`; Pull request linked to issue → `building`; Item closed → `done`; Auto-add to project → repo `grace`, filter `is:issue`. Turn off the others. Make the default view a board with `Status` as its columns.
- [ ] **Step 4:** `node scripts/backlog-to-issues.mjs`. Expected: 45 lines, ids `F01` to `F45`, no GitHub writes.
- [ ] **Step 5:** `node scripts/backlog-to-issues.mjs --apply`. Expected: 45 `F.. #..` lines, and `docs/features.md` rewritten as the map. If it stops halfway, run it again: rows already migrated are skipped.
- [ ] **Step 6:** Check: `gh issue list --repo hpagniez/grace --state all --limit 100 --json number --jq length` prints `45`, and `gh project item-list <number> --owner hpagniez --format json --jq '[.items[].status] | group_by(.) | map({(.[0]): length}) | add'` prints `{"done":23,"idea":22}`. Fix any item left in the wrong column by dragging it on the board.

### Task 3: Rules and the frozen map

**Files:**
- Modify: `docs/features.md` (written by Task 2 Step 5), `AGENTS.md:24,36,52`, `.github/pull_request_template.md:23-26,60`, `README.md:7`, `docs/decisions/0014-backlog-in-github-issues-and-a-project.md` (`status: accepted`)
- Delete: `scripts/backlog-to-issues.mjs`, `scripts/backlog-to-issues.test.ts`

- [ ] **Step 1: `AGENTS.md`.**
  - Docs map: the `docs/features.md` row becomes `Frozen map of the old backlog IDs F01–F45 to their issues`. Add a row above it: `[Issues](https://github.com/hpagniez/grace/issues) and the [Grace project](<project URL>)` | `Backlog: one issue per feature, priority as label must, should or could; status idea → building → done on the board`.
  - Doc rules: replace the `docs/features.md` bullet with: "Every feature has an issue with a priority label. Its body holds the feature, a `Karate surface:` line and a `Spec:` line. When a spec is written, point the issue's `Spec:` line at the file on `develop` (`gh issue edit <N> --body-file -`). A PR that builds a feature says `Closes #N`; a spec-only PR, or one of several, says `Part of #N`. The project's workflows move the status. For a feature built over several PRs, move its card to `building` when the first PR opens: `gh project item-add <number> --owner hpagniez --url <issue URL> --format json --jq .id` gives the item ID, then `gh project item-edit --id <item ID> --project-id <project ID> --field-id <Status field ID> --single-select-option-id <building option ID>` ([ADR 0014](docs/decisions/0014-backlog-in-github-issues-and-a-project.md))." Fill in the number and the IDs from `gh project view <number> --owner hpagniez --format json --jq .id` and `gh project field-list <number> --owner hpagniez --format json`.
  - Commits: delete the sentence "A spec or feature commit includes its `docs/features.md` row update."
- [ ] **Step 2: `.github/pull_request_template.md`.** Delete the `Feature:` line. The `Issue:` line becomes `- Issue: <!-- Closes #N (builds the feature), Part of #N (spec, or one of several PRs), or "none" -->`. The `docs/features.md` checkbox becomes `- [ ] Spec PR: the issue's \`Spec:\` line points at the new spec`.
- [ ] **Step 3: `README.md:7`.** Replace "See [docs/features.md](docs/features.md) for what it does and what is planned" with "See the [Grace project](<project URL>) for what it does and what is planned".
- [ ] **Step 4:** Set ADR 0014's front matter to `status: accepted`. Delete the script and its test.
- [ ] **Step 5:** `git grep -n "features\.md" -- AGENTS.md README.md .github` lists only the frozen-map row in `AGENTS.md`. `git grep -n "backlog-to-issues"` lists only this plan. Run `npm run lint`. Expected: PASS.
- [ ] **Step 6: Commit** `docs: move the backlog to github issues`. Open the PR as a draft into `develop`; its body says the 45 issues and the project already exist.
