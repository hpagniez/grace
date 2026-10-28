# Contributing to Grace

Grace is a small project with one maintainer. AI agents write most of the code, and people review it. Bug reports, ideas and pull requests are all welcome.

## Report a bug or ask for a feature

- **Bug:** open an issue with the bug form. Give your Grace and Windows versions and paste the console output. Remove secret values, tokens and personal paths first.
- **Feature:** open an issue with the feature form. The maintainer gives it a priority label (`must`, `should` or `could`) and puts it on the [project board](https://github.com/users/hpagniez/projects/2).
- **Security problem:** do not open an issue. See [SECURITY.md](SECURITY.md).

## Before you write code

Pick an issue, or open one, and say in it that you are working on it. A feature gets a design spec in `docs/specs/` before it is built, with its acceptance criteria in Gherkin. For a large feature, agree on the spec first, in the issue or in a spec-only pull request.

## Set up

You need Windows 11 and Node 24.

```bash
npm install
npm run vendor
npm start
```

`npm run vendor` downloads the bundled Java runtime and Karate jar, once.

## Make the change

- Branch from `develop` and name the branch `feature/<topic>`, in kebab case.
- Write each commit header as a [Conventional Commit](https://www.conventionalcommits.org/en/v1.0.0/), for example `feat(runner): run a single outline example`. CI checks every header.
- Add Vitest unit tests for logic and components, and a Playwright end-to-end test for what a user sees.
- Before you push, run `npm run lint`, `npm run typecheck` and `npm test`. If you changed more than `docs/**` and `*.md`, also run `npm run e2e`. Never run `npm test` and `npm run e2e` at the same time.
- Do not copy code, text or assets from Karate Labs' paid products.

## Open a pull request

1. Open it into `develop` as a draft and fill in the template. Strike out the items that do not apply.
2. Paste the `npm run e2e` summary, then mark it ready for review.
3. CI on a pull request from a fork waits until the maintainer approves the run.
4. The maintainer reviews it and merges it with a merge commit.

## AI agents

Agents are welcome. [AGENTS.md](AGENTS.md) is their brief and holds the full rules: the git flow, CI, Solid 2 and how Grace drives Karate. Read what your agent wrote before you open the pull request: you are its author.

## Licence

By contributing, you agree that your contribution is licensed under the [MIT licence](LICENSE) of this repository.
