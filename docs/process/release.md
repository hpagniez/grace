# Releases and hotfixes

The steps of a `release/*` or `hotfix/*` branch. The branch model is in [AGENTS.md](../../AGENTS.md#git-workflow).

- Release and hotfix branch names: [ADR 0009](../decisions/0009-release-and-hotfix-branches-named-after-their-tag.md); release candidates only: [ADR 0015](../decisions/0015-release-candidates-until-a-stable-release-is-decided.md).
- No stable release while the app is in development: every version is a release candidate of the first stable release, `1.0.0-rc.N`, in the branch, `package.json` and tag. Each release or hotfix bumps `N`. Stable `v1.0.0` needs a decision record that supersedes ADR 0015.
- Before a `release/*` or `hotfix/*` PR into `main`, check the public docs against the version it ships: `README.md` (features, install and update steps, status, roadmap) and its screenshots in `docs/images/`. Fix them on the branch; `npm run screens:readme` retakes the screenshots. The packaged-app check record, `docs/checks/<date>-<release|hotfix>-<version>-packaged.md`, gets a `## Public docs` section: what was checked, and what changed or why nothing had to. `cargo xtask check-pr` fails the PR without it.
- Before a `release/*` or `hotfix/*` PR into `main`, also run the release benchmark on the reference machine, the one named in `bench/limits.json`. Run `npm run make`, download and unzip the previous release's zip, then run `node bench/bench.mjs --release --a <prev exe> --a-zip <prev zip> --b-zip out/make/zip/win32/x64/<zip> --label-a "<prev version> <sha7>" --label-b "<version> <sha7>"`, the labels naming the measured commits. Copy `bench-out/bench.json` to `docs/checks/<record>.bench.json` (the check record's file name with `.md` replaced) and paste `bench-out/bench.md`, a `### Bench:` table that ends with the `Result: pass` or `Result: miss` line, under `## Performance` in the record. On a miss (a measure over its limit, a measure with a limit that was not measured, or a measure worse than the previous release beyond its noise margin), write a `### Analysis` there (for each measure that missed: how much it moved, which change caused it, why, and whether it can be fixed) and stop for the owner's `Decision:`. `scripts/check-pr.mjs` fails the PR without the section, without the JSON or with one that has no previous release or fewer than 5 rounds, without a `Result:` line the JSON supports, on a miss without `Decision: go`, and on `Decision: no-go`.
- Never write a release's `Decision:` line. An agent writes the `### Analysis`; the owner writes `Decision: go` or `Decision: no-go`.
- PRs into `main` are merged by hand, never by `merge-when-green`.

## The branches

| Branch | Cut from | Merges into | Notes |
|---|---|---|---|
| `release/vX.Y.Z-rc.N` | `develop` | `main` and `develop` | Named after its tag, e.g. `release/v1.0.0-rc.3`. Version bump and fixes only. Tag on `main`. |
| `hotfix/vX.Y.Z-rc.N` | `main` | `main` and `develop` | Named after its tag, e.g. `hotfix/v1.0.0-rc.4`. Urgent fix to a released version. |

## The `release` workflow

| Workflow | When | What it runs |
|---|---|---|
| `release` | Push of a `vX.Y.Z-rc.N` tag (a plain `vX.Y.Z` tag runs nothing); by hand | Tag must match `package.json`; `npm run make`; SignPath signs `Grace.exe` once a person approves, when the `SIGNPATH_ORGANIZATION_ID` variable is set ([ADR 0024](../decisions/0024-sign-grace-exe-with-signpath-foundation.md)); GitHub pre-release with the zip. By hand: build and test-sign, publish nothing |
