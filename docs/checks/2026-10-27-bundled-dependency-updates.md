# Check: bundled dependency updates (F33)

Date: 2026-10-27
Branch: `feature/bundled-dependency-updates`
Machine: Windows 11 Pro 10.0.26300 (x64), Node 24.15.0, bundled Temurin 21.0.12.1 and Karate 2.1.4.
Spec: [Bundled dependency updates](../specs/2026-10-27-bundled-dependency-updates.md).

No app code changed, so the packaged app was not checked by hand. CI runs e2e on the PR once it is marked ready.

## Automated part (run by an agent)

| Check | Result |
|---|---|
| `npm test` | 691/691 passed, including the new `scripts/bump-vendor.test.ts` (21) and `scripts/bump-pr.test.ts` (11) |
| `npm run typecheck`, `npm run lint` | passed |
| `vendor-updates.yml`, `checks.yml` | parse as YAML; no `${{ }}` inside a `run` line |

## By hand (run by an agent)

1. **`npm run vendor` follows a changed pin.**
   - Run on a `vendor/` unpacked by the old script (no marker): the JRE was unpacked once more from the cached zip and `vendor/.jre-sha256` written. The next run printed `vendor ready` without unpacking.
   - Pins set to Temurin `21.0.12+8` and Karate `2.1.3` (URLs and checksums from the Adoptium and GitHub APIs), with a stray file added to `vendor/licenses/temurin`: `vendor/jre/release` showed `21.0.12+8-LTS`, `vendor/karate.jar` hashed to the 2.1.3 digest, and the stray file was gone.
   - Pins set back: `21.0.12.1+1-LTS` and the 2.1.4 hash again.
   - Result: passed.
2. **The bump script reproduces today's pins.** `karate.version` set to `2.1.3` and `jre.version` to `21.0.11+9` in `scripts/vendor.json`. `npm run vendor:bump -- karate` printed `karate: 2.1.3 -> 2.1.4` and `npm run vendor:bump -- jre` printed `jre: 21.0.11+9 -> 21.0.12.1+1`. `git diff scripts/vendor.json` was then empty: the live APIs gave exactly today's URLs and SHA-256, LICENSE included. Result: passed.
3. **`bump-pr.mjs --check`** for Karate `2.1.3` to `2.1.4` (read-only `gh` and `git` calls): printed that the branch is new and wrote `todo=true`. The real compare `v2.1.3...v2.1.4` lists 188 files, without `RunCommand.java`. Result: passed.

## Still needs a person

1. Turn on **Settings → Actions → General → Workflow permissions → "Allow GitHub Actions to create and approve pull requests"**. It is off today (`can_approve_pull_request_reviews: false`); without it the workflow cannot open PRs.
2. After merge, the spec's "First real run after merge" check:
   1. Push a throwaway branch from `develop` whose `scripts/vendor.json` pins Karate `2.1.3` (`version`, `url`, `sha256` `c0126f3ad907e4e92a9944a0bfa4deae96beae599daf8513f47169516b5714c0`, `file`).
   2. `gh workflow run vendor-updates.yml --ref <that branch>`.
   3. Expected: a draft PR "build(deps): bump karate from 2.1.3 to 2.1.4" from `feature/bump-karate-2.1.4`, with a fixtures commit recorded on the Windows runner; the JRE job opens nothing.
   4. Mark it ready: `checks` and `test` run and pass.
   5. Close the PR without merging and delete both branches.
   - Result: not run yet.
