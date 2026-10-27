# Bundled Dependency Updates Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A weekly workflow opens a draft PR when a newer Temurin 21 JRE or Karate 2.x is out, with the new pin and checksum, and with the event fixtures re-recorded for a Karate bump.

**Architecture:** Pins move to `scripts/vendor.json`. `scripts/bump-vendor.mjs` rewrites one pin from the upstream API (local and CI). `scripts/bump-pr.mjs` is the CI side: it decides whether a version was handled, commits, pushes, opens the draft PR and supersedes older bot-only drafts, through `git` and `gh`. `.github/workflows/vendor-updates.yml` wires them: Linux for detection and the JRE, Windows only to re-record fixtures.

**Tech Stack:** Node 24 built-ins only (`fetch`, `node:crypto`, `node:child_process`), GitHub Actions, `gh` CLI (preinstalled on runners), Vitest 5 for unit tests (`scripts/*.test.ts` run in the `node` project).

**Spec:** [docs/specs/2026-10-27-bundled-dependency-updates.md](../specs/2026-10-27-bundled-dependency-updates.md)

**After the final review:** Tasks 3 and 4 changed as the spec's "Changes from the final review" says: `openBumpPr(o, run)` with an injectable runner, `supersedePlan(prs, dep, to, commitsOf)`, `runCommandStatus(filenames)`, and a `karate-pr` Linux job after the read-only Windows job.

## Global Constraints

- No new dependency. Scripts are plain `.mjs` with JSDoc, like `scripts/fetch-vendor.mjs` and `scripts/check-pr.mjs`.
- Commits follow ADR 0003 (`scripts/check-pr.mjs` checks them): header ≤ 72 characters, lower-case subject.
- Branch per bump: `feature/bump-<jre|karate>-<to>`, `+` replaced by `_` (`feature/bump-jre-21.0.13_11`).
- Bump commit headers: `build(deps): bump temurin jre from <from> to <to>`, `build(deps): bump karate from <from> to <to>`; fixtures: `test(fixtures): record events with karate <to>`.
- Bot identity: name `github-actions[bot]`, email `41898282+github-actions[bot]@users.noreply.github.com`.
- Actions pinned to the same SHAs as `test.yml`: `actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1`, `actions/setup-node@949feb2413d6458794dcd2491c4babbbce0c15c1 # v7.1.0`.
- Upstream endpoints: `https://api.adoptium.net/v3/assets/latest/21/hotspot?os=windows&architecture=x64&image_type=jre`, `https://api.github.com/repos/karatelabs/karate/releases?per_page=100`, `https://raw.githubusercontent.com/karatelabs/karate/v<ver>/LICENSE`.
- Never run `npm test` and `npm run e2e` at the same time (AGENTS.md).

## Review Focus

1. Temurin versions with and without the fourth part (`21.0.13+11` vs `21.0.13.1+2`, `21.0.12.1+1`): compared as `[major, minor, security, patch ?? 0, build]`, never as text. Test in Task 2.
2. An Adoptium `release_name` that does not parse (`jdk-21.0.13+11-ea`) or a pinned `version` in `vendor.json` that does not parse: the script fails, naming the value, and writes nothing. Test in Task 2.
3. The LICENSE download fails (404) during a Karate bump: fail, `vendor.json` unchanged. Test in Task 2.
4. An older bump PR whose commits include a person's (email other than the bot's), or which is marked ready: kept open and named, never closed. A PR of the other dependency, or with an equal or higher version, is never touched. Test in Task 3.
5. `npm run vendor` on a `vendor/` unpacked before F33 (no marker): the JRE is unpacked once more, then left alone on the next run. By-hand check in Task 1.

---

### Task 1: Pins in `scripts/vendor.json`, `npm run vendor` follows them

**Files:**
- Create: `scripts/vendor.json`
- Modify: `scripts/fetch-vendor.mjs` (pins at lines 13-27, `fetchVerified`, `setupJre`, `setupLicenses`, `main`)
- Modify: `.github/workflows/test.yml`, `.github/workflows/release.yml` (vendor cache `key`)
- Test: `scripts/fetch-vendor.test.ts` (unchanged, must stay green)

**Interfaces:**
- Produces: `scripts/vendor.json` with keys `jre` (`version`, `url`, `sha256`, `file`), `karate` (`version`, `url`, `sha256`, `file`), `license` (`url`, `sha256`, `file`). Values today: JRE `21.0.12.1+1`, the current URL, SHA-256, `file: "jre.zip"`; Karate `2.1.4`, current URL, SHA-256, `file: "karate-2.1.4.jar"`; license as today, `file: "karate-LICENSE.txt"`. Formatted as `JSON.stringify(pins, null, 2) + "\n"`.
- Produces: marker file `vendor/.jre-sha256` holding the JRE zip's SHA-256 (no newline).

- [ ] **Step 1: Create `scripts/vendor.json`** with the values above, copied from the current constants.
- [ ] **Step 2: `fetch-vendor.mjs` reads the pins** with `JSON.parse(readFileSync(join(root, "scripts", "vendor.json"), "utf8"))` (not a JSON import: Vitest imports this module for `verifySha256`). Drop the three constants. `verifySha256` keeps its signature and export.
- [ ] **Step 3: Add `async function hasSha256(file, sha256): Promise<boolean>`** (false when the file is missing or does not match) and use it in `fetchVerified` for the cached download.
- [ ] **Step 4: JRE follows its pin (R2.1).** `setupJre` returns early only when `vendor/jre/bin/java.exe` exists and `vendor/.jre-sha256` exists and equals the pinned `sha256`. After a successful unpack it writes the marker.
- [ ] **Step 5: Jar follows its pin (R2.2).** In `main`, copy the verified download over `vendor/karate.jar` when `!(await hasSha256(jar, karate.sha256))`; keep the final `verifySha256`.
- [ ] **Step 6: Licences rebuilt (R2.3).** `setupLicenses` starts with `rm(out, { recursive: true, force: true })`.
- [ ] **Step 7: Cache keys (R1.3).** In both workflows: `key: vendor-${{ runner.os }}-${{ hashFiles('scripts/fetch-vendor.mjs', 'scripts/vendor.json') }}`.
- [ ] **Step 8: By-hand check (spec "npm run vendor follows a changed pin", Review Focus 5).**
  Run `npm run vendor` (fresh: downloads, writes the marker). Run it again: prints `vendor ready` without downloading. Set the `karate` pin to 2.1.3 (`karate-2.1.3.jar`, URL and `digest` from `gh api repos/karatelabs/karate/releases/tags/v2.1.3`) and the `jre` pin to the previous Java 21 GA build (from `https://api.adoptium.net/v3/assets/feature_releases/21/ga?os=windows&architecture=x64&image_type=jre&page_size=2`, second entry). Run `npm run vendor`: `vendor/jre/release` shows the older version, `vendor/karate.jar` hashes to the 2.1.3 digest, `vendor/licenses/temurin` holds only the older JRE's files. `git checkout scripts/vendor.json`, run again: back to today's versions. Expected: every step as described; record the result for Task 5.
- [ ] **Step 9: Run** `npx vitest run scripts/fetch-vendor.test.ts`. Expected: 2 passed.
- [ ] **Step 10: Commit**

```bash
git add scripts/vendor.json scripts/fetch-vendor.mjs .github/workflows/test.yml .github/workflows/release.yml
git commit -m "build(vendor): move the pins to vendor.json and follow a changed pin"
```

### Task 2: `scripts/bump-vendor.mjs`

**Files:**
- Create: `scripts/bump-vendor.mjs`, `scripts/bump-vendor.test.ts`
- Modify: `package.json` (`"vendor:bump": "node scripts/bump-vendor.mjs"`, after `"vendor"`)

**Interfaces:**
- Consumes: `scripts/vendor.json` (Task 1).
- Produces (exports):
  - `parseJreVersion(text: string): number[] | null`: `/^(\d+)\.(\d+)\.(\d+)(?:\.(\d+))?\+(\d+)$/` → `[major, minor, security, patch ?? 0, build]`.
  - `parseKarateVersion(text: string): number[] | null`: `/^(\d+)\.(\d+)\.(\d+)$/`.
  - `compareVersions(a: number[], b: number[]): number`: sign of the first differing part, 0 if equal.
  - `pickJre(assets: unknown, pinned: string): { version: string, url: string, sha256: string } | null`
  - `pickKarate(releases: unknown, pinned: string): { version: string, url: string, sha256: string, file: string } | null`
  - `bump(dep: "jre" | "karate", opts?: { pinsPath?: string, fetch?: typeof fetch }): Promise<{ from: string, to: string } | null>`: reads the pins, asks upstream, writes `vendor.json` only when it changes and every check passed; null when up to date.
- CLI: `node scripts/bump-vendor.mjs <jre|karate>`. Prints `<dep>: <from> -> <to>` or `<dep>: <from> is the latest`. When `GITHUB_OUTPUT` is set, appends `changed=true|false`, `from=<pinned>`, `to=<new or pinned>`. Any error: message on stderr, exit 1. Unknown argument: usage on stderr, exit 2.

Rules the code must follow (spec R3):
- `pickJre`: `assets` must be a non-empty array; use `assets[0]`. `version` = `release_name` without `jdk-`; it must parse, else throw `Adoptium release_name "<x>" is not a Java version`. Different major from the pinned one, or not newer: return null. `binary.package.checksum` must match `/^[0-9a-f]{64}$/` and `binary.package.link` must start with `https://github.com/adoptium/temurin21-binaries/releases/download/`, else throw naming the field.
- `pickKarate`: keep releases with `!draft && !prerelease` and `tag_name` matching `/^v(\d+\.\d+\.\d+)$/`, same major as the pinned one; take the highest. Not newer: null. Then the asset named `karate-<ver>.jar` must exist (else throw `Karate v<ver> has no karate-<ver>.jar asset`), its `digest` must be `sha256:<64 hex>` (else throw `karate-<ver>.jar has no sha256 digest`), and its `browser_download_url` must start with `https://github.com/karatelabs/karate/releases/download/v<ver>/`.
- An unparseable pinned version throws `vendor.json <dep> version "<x>" does not parse`.
- `bump` for Karate also downloads `https://raw.githubusercontent.com/karatelabs/karate/v<ver>/LICENSE` and sets `license.url` and `license.sha256` (SHA-256 of the body). A non-OK response throws `GET <url> failed: <status>`, as `fetchVerified` words it.
- GitHub requests send `Authorization: Bearer <GITHUB_TOKEN>` when the variable is set, and `Accept: application/vnd.github+json`.

- [ ] **Step 1: Write the failing tests** in `scripts/bump-vendor.test.ts` (inline payloads; `bump` gets a fake `fetch` returning `new Response(JSON.stringify(x))` per URL and a temp copy of `vendor.json` in `mkdtempSync(join(tmpdir(), "kk-bump-"))`):
  - `parseJreVersion`: `"21.0.12.1+1"` → `[21,0,12,1,1]`; `"21.0.13+11"` → `[21,0,13,0,11]`; `"21.0.13+11-ea"` → null.
  - `compareVersions`: `21.0.13+11` > `21.0.12.1+1`; `21.0.13.1+2` > `21.0.13+11`; Karate `2.1.10` > `2.1.9`; equal → 0.
  - `pickKarate` with pinned `2.1.4` returns null for each of: newest `v2.1.4`; newest `v2.1.3`; `v3.0.0` (with `v2.1.4` also listed); `v2.2.0` with `prerelease: true`; `v2.2.0.RC1`; a draft `v2.2.0`.
  - `pickKarate` with pinned `2.1.9` and releases `v2.1.9`, `v2.1.10` → `{ version: "2.1.10", file: "karate-2.1.10.jar", sha256: <digest hex>, url: <browser_download_url> }`.
  - `pickKarate` throws `/has no karate-2.1.5.jar asset/` and `/has no sha256 digest/` for the two broken assets.
  - `pickJre` with pinned `21.0.12.1+1`: today's payload (copy the live shape: `release_name`, `binary.package.{checksum, link, name}`) → null; a `jdk-21.0.13+11` payload → its version, link, checksum; a missing `checksum` throws `/checksum/`; `release_name: "jdk-21.0.13+11-ea"` throws `/is not a Java version/`.
  - `bump("karate", …)`: newer release + LICENSE body `"MIT"` → returns `{ from: "2.1.4", to: "2.1.5" }`; the file now holds the 2.1.5 karate entry, `license.url` at `v2.1.5` and `license.sha256` = SHA-256 of `"MIT"`; the `jre` entry is byte-for-byte unchanged; the file ends with `"}\n"`.
  - `bump("karate", …)` rejects and leaves the file byte-for-byte unchanged when: the releases URL answers 500; the LICENSE URL answers 404.
  - `bump("jre", …)` up to date → null, file unchanged.
- [ ] **Step 2: Run** `npx vitest run scripts/bump-vendor.test.ts`. Expected: FAIL (module not found).
- [ ] **Step 3: Implement** `scripts/bump-vendor.mjs` with the exports and rules above. The CLI runs only when the file is the entry point (same `import.meta.url === pathToFileURL(process.argv[1]).href` guard as `fetch-vendor.mjs`).
- [ ] **Step 4: Run** `npx vitest run scripts/bump-vendor.test.ts`. Expected: PASS.
- [ ] **Step 5: By-hand check (spec "The bump script reproduces today's pins").** In `scripts/vendor.json` set `karate.version` to `2.1.3` and `jre.version` to `21.0.11+9`. Run `npm run vendor:bump -- karate` and `npm run vendor:bump -- jre`. Expected: `git diff scripts/vendor.json` is empty. Record for Task 5.
- [ ] **Step 6: Commit**

```bash
git add scripts/bump-vendor.mjs scripts/bump-vendor.test.ts package.json
git commit -m "build(vendor): add a script that bumps a pin to the newest release"
```

### Task 3: `scripts/bump-pr.mjs`

**Files:**
- Create: `scripts/bump-pr.mjs`, `scripts/bump-pr.test.ts`

**Interfaces:**
- Consumes: `parseJreVersion`, `parseKarateVersion`, `compareVersions` from `./bump-vendor.mjs` (Task 2).
- Produces (exports):
  - `BOT_EMAIL = "41898282+github-actions[bot]@users.noreply.github.com"`
  - `branchName(dep, version): string`, `versionFromBranch(dep, branch): string | null`
  - `titleFor(dep, from, to): string`
  - `supersedePlan(prs, dep, to): { close: number[], keep: number[] }`. `prs` is `gh pr list --json number,headRefName,isDraft,commits` output; a commit is `{ authors: [{ email, login, name }] }`. Only heads `feature/bump-<dep>-<v>` with `<v>` parsing and lower than `to`. Close when `isDraft` and every author of every commit has `email === BOT_EMAIL`; otherwise keep.
  - `runCommandStatus(files: { filename: string }[]): "changed" | "unchanged" | "unknown"`: any `filename` ending in `/RunCommand.java` → changed; else 300 or more files → unknown; else unchanged.
  - `prBody({ dep, from, to, keep, runCommand }): string`
- CLI: `node scripts/bump-pr.mjs <jre|karate> <from> <to> [--check]`, run from the repo root with the bumped `vendor.json` (and, for Karate, re-recorded fixtures) in the working tree. Uses `execFileSync("git" | "gh", …, { encoding: "utf8" })`. Appends `todo=true|false` to `GITHUB_OUTPUT`.
  1. `gh pr list --head <branch> --state all --json number` non-empty → log `<branch>: already has a PR`, `todo=false`, stop.
  2. `exists` = `git ls-remote --heads origin <branch>` prints a line.
  3. `--check` and not `exists` → `todo=true`, stop (the Windows job continues).
  4. Not `exists`: `git switch -c <branch>`; `git add scripts/vendor.json`; commit with `titleFor`; for Karate `git add test/fixtures/events` and, if `git diff --cached --quiet` exits 1, commit `test(fixtures): record events with karate <to>`; `git push origin <branch>`. Commits use `git -c user.name=github-actions[bot] -c user.email=<BOT_EMAIL> commit -m …`.
  5. `gh pr list --state open --base develop --limit 100 --json number,headRefName,isDraft,commits` → `supersedePlan`.
  6. Karate: `gh api repos/karatelabs/karate/compare/v<from>...v<to>` → `runCommandStatus(json.files ?? [])`.
  7. `gh pr create --draft --base develop --head <branch> --title <title> --body-file -` with the body on stdin; the new number is the last path part of the printed URL.
  8. For each `close`: `gh pr close <n> --comment "Superseded by #<new>" --delete-branch`. `todo=false`.

`prBody` text (spec R5), lines joined with `\n`, blank line between paragraphs:

```text
Mark this PR ready for review to run `checks` and `test` (e2e included) on the new version. To skip this version, close this PR.

Karate:
- Release notes: https://github.com/karatelabs/karate/releases/tag/v<to>
- Changes: https://github.com/karatelabs/karate/compare/v<from>...v<to>
- `RunCommand.java`: changed | unchanged | unknown, check by hand. Check the flags in AGENTS.md (Karate integration rules) against it.
- Fixtures recorded again with Karate <to> (second commit; time stamps, durations and ports always differ). `src/main/karate/http.test.ts` checks the HTTP calls (F11).
- Check that `RESERVED_KARATE_KEYS` in `src/shared/env.ts` still lists every system property Karate reads.

JRE:
- Release: https://github.com/adoptium/temurin21-binaries/releases/tag/jdk-<to with + as %2B>

(one line per kept PR) #<n> is still open and has changes by people: carry them over or close it.
```

Only the block of the bumped dependency appears; the kept-PR lines only when `keep` is non-empty.

- [ ] **Step 1: Write the failing tests** in `scripts/bump-pr.test.ts`:
  - `branchName("jre", "21.0.13+11")` = `"feature/bump-jre-21.0.13_11"`; `versionFromBranch("jre", that)` = `"21.0.13+11"`; `versionFromBranch("karate", "feature/bump-jre-21.0.13_11")` = null.
  - `titleFor("karate", "2.1.4", "2.1.5")` = `"build(deps): bump karate from 2.1.4 to 2.1.5"`; `commitError` from `./check-pr.mjs` is null for it and for `titleFor("jre", "21.0.12.1+1", "21.0.13.1+12")`.
  - `supersedePlan(prs, "karate", "2.1.6")` with: #1 `feature/bump-karate-2.1.5` draft, bot-only → close; #2 same version, draft, one commit by `hugo@example.com` → keep; #3 `feature/bump-karate-2.1.4` not a draft, bot-only → keep; #4 `feature/bump-karate-2.1.7` draft bot-only → neither; #5 `feature/bump-jre-21.0.13_11` → neither; #6 `feature/home-screen` → neither. Expected `{ close: [1], keep: [2, 3] }`.
  - `runCommandStatus`: a list holding `karate-core/src/main/java/io/karatelabs/cli/RunCommand.java` → changed; 3 other files → unchanged; 300 other files → unknown.
  - `prBody` for Karate contains the release link, the compare link, `` `RunCommand.java`: unchanged``, the fixtures line and the `RESERVED_KARATE_KEYS` line, and no `JRE:`; with `keep: [7]` it ends with `#7 is still open and has changes by people: carry them over or close it.`
  - `prBody` for the JRE contains `releases/tag/jdk-21.0.13%2B11` and no `Karate:`.
- [ ] **Step 2: Run** `npx vitest run scripts/bump-pr.test.ts`. Expected: FAIL (module not found).
- [ ] **Step 3: Implement** `scripts/bump-pr.mjs` (exports above; CLI behind the entry-point guard).
- [ ] **Step 4: Run** `npx vitest run scripts/bump-pr.test.ts`. Expected: PASS.
- [ ] **Step 5: Commit**

```bash
git add scripts/bump-pr.mjs scripts/bump-pr.test.ts
git commit -m "ci: add a script that opens and supersedes the bump prs"
```

### Task 4: The workflow and `checks.yml`

**Files:**
- Create: `.github/workflows/vendor-updates.yml`
- Modify: `.github/workflows/checks.yml` (`on.pull_request`)

**Interfaces:**
- Consumes: `node scripts/bump-vendor.mjs <dep>` outputs `changed`, `from`, `to` (Task 2); `node scripts/bump-pr.mjs <dep> <from> <to> [--check]` output `todo` (Task 3).

- [ ] **Step 1: Write `vendor-updates.yml`.** First line comment: `# Weekly draft PRs for a newer bundled JRE or Karate jar (ADR 0012). Marking one ready for review runs checks and test.`
  - `on`: `schedule: - cron: "0 6 * * 1"` (Mondays 06:00 UTC) and `workflow_dispatch`.
  - `permissions`: `contents: write`, `pull-requests: write`.
  - `concurrency`: `group: vendor-updates`, `cancel-in-progress: false`.
  - Job `jre` (`ubuntu-latest`, 10 min): checkout, setup-node 24, step `bump` = `node scripts/bump-vendor.mjs jre` with `GITHUB_TOKEN: ${{ github.token }}`; if `steps.bump.outputs.changed == 'true'`: `node scripts/bump-pr.mjs jre "$FROM" "$TO"` with `GH_TOKEN`, `FROM`, `TO` from the step outputs (env, never `${{ }}` inside `run`).
  - Job `karate-check` (`ubuntu-latest`, 10 min, `outputs: todo: ${{ steps.pr.outputs.todo }}`): same, with `--check` on the `bump-pr` step (`id: pr`). A branch with no PR gets its PR here, with no Windows job.
  - Job `karate` (`needs: karate-check`, `if: needs.karate-check.outputs.todo == 'true'`, `windows-latest`, 30 min, `shell: bash`): checkout, setup-node, `bump` step again; if changed: `node scripts/fetch-vendor.mjs && node scripts/capture-fixtures.mjs` with `TEMP` and `TMP` = `${{ runner.temp }}` and the comment `# the default temp folder is an 8.3 short path (C:\Users\RUNNER~1\...) that the fixture sanitiser may miss`; then `node scripts/bump-pr.mjs karate "$FROM" "$TO"`.
- [ ] **Step 2: `checks.yml`**: `pull_request` gets `types: [opened, synchronize, reopened, ready_for_review]`, with the comment `# ready_for_review: a bump PR opened by vendor-updates starts CI only when a person marks it ready`.
- [ ] **Step 3: Parse the YAML** of both files with any local YAML parser (no project dependency), for example `python -c "import sys, yaml; [yaml.safe_load(open(f)) for f in sys.argv[1:]]" .github/workflows/vendor-updates.yml .github/workflows/checks.yml`. Expected: exit 0. Check by eye that every `${{ }}` sits in `with`, `env`, `if` or `outputs`, never in `run`.
- [ ] **Step 4: Commit**

```bash
git add .github/workflows/vendor-updates.yml .github/workflows/checks.yml
git commit -m "ci: open weekly draft prs for a newer bundled jre or karate"
```

### Task 5: Docs

**Files:**
- Create: `docs/decisions/0012-weekly-draft-prs-for-the-bundled-jre-and-karate.md`, `docs/checks/2026-10-27-bundled-dependency-updates.md`
- Modify: `AGENTS.md` (workflow table, Dependabot line, `npm run fixtures` rule), `docs/features.md` (F33 row)

- [ ] **Step 1: ADR 0012** (MADR minimal, like ADR 0011): context (pins manual, Dependabot blind, `GITHUB_TOKEN` PRs start no CI, no secrets, repo setting off); options 1–3 from the spec; decision: the workflow as built, permissions `contents: write` + `pull-requests: write` (widens ADR 0007 as ADR 0011 does), "Allow GitHub Actions to create and approve pull requests" must be on, CI starts on "Ready for review" (`checks.yml` gets `ready_for_review`), pins in `scripts/vendor.json`, the handled/supersede rules, majors manual. Consequences: replaces ADR 0007's "they stay manual"; one click per bump; first run only after merge; about 4 Linux minutes a month plus about 15 Windows minutes per Karate bump; the setting also lets workflows approve PRs (no effect without branch protection).
- [ ] **Step 2: AGENTS.md.** Workflow table row: `` | `vendor-updates` | Weekly, and by hand | Draft PR per newer Temurin 21 JRE or Karate 2.x (pins in `scripts/vendor.json`, ADR 0012); a Karate bump re-records the event fixtures. Mark it ready for review to run CI | ``. Dependabot line: replace "It does not update the JRE and Karate pins in `scripts/fetch-vendor.mjs`." with "It does not see the JRE and Karate pins in `scripts/vendor.json`: `vendor-updates` bumps them (`npm run vendor:bump -- <jre|karate>` by hand)." The `npm run fixtures` rule: add "The `vendor-updates` bump PR re-records them on its own."
- [ ] **Step 3: Check record** with: machine, date, the two by-hand results from Tasks 1 and 2, `npm test`/lint/typecheck results, and "Still needs a person": turn on the repo setting; after merge, the spec's "First real run after merge" check.
- [ ] **Step 4: features.md** F33 status `done`.
- [ ] **Step 5: Run** `npm run lint`, `npm run typecheck`, `npm test`. Expected: all pass.
- [ ] **Step 6: Commit**

```bash
git add AGENTS.md docs/decisions/0012-weekly-draft-prs-for-the-bundled-jre-and-karate.md docs/checks/2026-10-27-bundled-dependency-updates.md docs/features.md
git commit -m "docs: record the bundled dependency updates decision and checks"
```
