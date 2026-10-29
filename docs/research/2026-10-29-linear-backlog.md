# Moving the backlog to Linear

Checked on 2026-10-29 against Linear's pricing page and docs, Anthropic's Claude Code plugin directory, and the backlog of `hpagniez/grace` (issues, `AGENTS.md`, `.github/`). No Linear workspace was created: anything that needs one (the in-app plan table, a real sync) is marked as not checked. Issue: #114.

**Bottom line:** stay on GitHub Issues and the Grace project. [ADR 0014](../decisions/0014-backlog-in-github-issues-and-a-project.md) stands, so no decision record. Linear's GitHub integration would remove one manual step (moving a multi-PR feature to `building`) and tell "cancelled" from "done". In exchange, the backlog of a public repo would become private, outside reports (#86) would live in two places with two IDs, every agent machine and the CI would need a Linear login or key, and the free plan stops new issues at 250.

## The backlog today

- 49 issues, all opened by the maintainer: 21 open, 28 closed (all as completed, none as not planned).
- `docs/features.md` maps F01–F45 to issue numbers. About 20 other Markdown files under `docs/` cite `#N`, some of them PRs (issues and PRs share one number sequence).
- Four files act on issues: `AGENTS.md` (doc rules, the project and field IDs for the manual move), `.github/workflows/merge-when-green.yml` (closes the issues a merged PR names with `Closes #N`), `.github/pull_request_template.md` (the `Issue:` line) and ADR 0014.

## 1. What the free plan limits

From the pricing page:

| | Free | Basic |
|---|---|---|
| Price | $0 | $10 per user per month, billed yearly |
| Members | unlimited | unlimited |
| Teams | 2 | 5 |
| Issues | 250 | unlimited |
| File upload | 10 MB | unlimited |
| SSO | Google | Google |

- Guest accounts and private teams are listed on the Business card, not on Free or Basic.
- Archived issues: the pricing page says only "250 issues". A third-party guide (t0ggles, 2026-08-28) quotes the in-app Settings > Plans table as "Issues (excluding archive)". Not checked here. Closed issues auto-archive after a period each team sets.
- At the limit: "If you have over 250 issues, you will no longer be able to create new issues" (billing docs, in the section on cancelling a paid plan). "Nothing will be deleted."
- The pricing page's table did not render its check marks in text, so whether the GitHub integration and API are on Free is not confirmed by Linear. The same third-party guide lists the GitHub integration as Free.

48 of the 49 issues were opened on 2026-10-28, most of them the F01–F45 import, so the past rate says little. With 21 open, the cap is far off if closed issues archive. It is not far off once outside users file issues on a public repo: every synced copy is a Linear issue.

## 2. PR linking and automatic status

Linear's GitHub integration (docs page "GitHub"):

- A PR links to an issue when the Linear ID (for example `GRA-12`) is in the branch name, the PR title, or after a magic word in the title, description or a commit. Closing words: `close`, `closes`, `fix`, `fixes` and their forms. Non-closing: `ref`, `refs`, `references`, `related to`. `skip` or `ignore` before the ID unlinks.
- Status changes are configurable on PR drafted, opened, review requested, ready for merge and merged. Default: In Progress when a PR opens, Done when it merges.
- Rules can be set per target branch (regex), so PRs into `develop` and into `main` can move issues differently.
- "With multiple PRs, the issue changes status only when the final linked PR reaches the required state."
- With a non-closing word, the merge status is not applied.

Against ADR 0014's bad consequences:

- Multi-PR features: with Linear, the first PR that names `refs GRA-12` moves the issue to In Progress. No `gh project item-edit` with three IDs. This is the one real gain. A spec-only PR that names the issue would move it too, unless it leaves the ID out.
- "Not planned" shows as `done`: Linear has a Canceled state. On GitHub the close reason already tells them apart, and no issue has been closed as not planned yet.
- `merge-when-green` closes issues itself because GitHub ignores closing keywords in a merge made with the workflow token. Linear reads the merged PR through its own GitHub app, so `Fixes GRA-12` should apply whoever merges. Not tested.
- Backlog not in git, workflows set up by hand in a web UI: same with Linear.

## 3. Outside users (#86)

The repo is public ([ADR 0018](../decisions/0018-make-the-repository-public.md)). Today anyone can read the backlog and file an issue with a GitHub account.

- A Linear workspace is members only. Guest accounts are a Business feature. Nothing on the pricing page offers a public read-only view on Free.
- So outside users would keep filing on GitHub. GitHub Issues Sync copies them into Linear: one-way (GitHub to Linear) or two-way, for one repo at a time. It syncs title, description, status, assignee, labels, sub-issues, comments (only the synced thread) and priority, and only for issues created after it is turned on.
- Each report then exists twice, with a GitHub number and a Linear ID. ADR 0014 moved off `docs/features.md` partly to have one place where outside users and agents meet; this splits it again. Linear-only issues (filed by agents in Linear) would stay invisible to outside users unless the sync is two-way.
- Several open-source teams keep GitHub for outside reports and pull only labelled ones into Linear (Cal.com's handbook, SyncLinear). That fits a team with paid members who triage. It adds a triage step for a single maintainer.

## 4. Agents and CI

| | GitHub today | Linear |
|---|---|---|
| Agents read and write | `gh` CLI, already logged in on the maintainer's computer; one token covers every worktree | Linear's MCP server `https://mcp.linear.app/mcp` (OAuth 2.1, or a bearer API key; `/mcp/readonly` for read-only). Anthropic's plugin directory has Linear's official plugin, which registers this server. No official CLI in Linear's docs |
| API | REST and GraphQL through `gh api` | GraphQL; an API key gets 2,500 requests and 3,000,000 complexity points per hour |
| CI | `GITHUB_TOKEN` (`issues: write`), no secret to manage | A `LINEAR_API_KEY` repository secret for any workflow step that touches the backlog |
| Cloud sessions and subagents | `gh` works wherever the repo is cloned with a token | Each environment needs its own Linear login or key |

The MCP server gives agents typed tools (find, create, update issues, projects, comments), which is nicer than shell commands. It does not remove the need for `gh`: PRs, CI runs, releases and labels stay on GitHub, so agents would need both.

## 5. Cost of a move

- Import: Linear's GitHub importer brings titles, descriptions, labels, projects, comments and sub-issues; closed issues optionally, into the archive. It does not bring custom fields, so the project's `Status` field is lost. It does not say whether GitHub numbers are kept, so expect new `GRA-N` IDs.
- References: `docs/features.md` and about 20 other Markdown files cite `#N`. Either a second map (`#N` to `GRA-N`) or two IDs per feature from then on.
- Rules and tooling: rewrite the doc rules and the manual-move commands in `AGENTS.md`, the PR template's `Issue:` line, the closing step of `merge-when-green`, and the `Closes #N` / `Part of #N` convention.
- A new decision record that supersedes ADR 0014.

## 6. What it would replace in ADR 0014

| ADR 0014 | With Linear |
|---|---|
| One issue per feature on GitHub, body with `Karate surface:` and `Spec:` | One Linear issue; same body |
| Priority labels `must`, `should`, `could` | Linear's priority field, or the same labels |
| Project `Grace`, `Status` idea / building / done, four built-in workflows | Linear workflow states; status moved by the GitHub integration |
| Manual move to `building` for multi-PR features | Not needed |
| `Closes #N` / `Part of #N` | `Fixes GRA-N` / `refs GRA-N` |
| `docs/features.md` maps F01–F45 to issues | Maps them to Linear IDs, or to both |
| Outside users file and follow issues on GitHub | Outside users file on GitHub; copies in Linear; the backlog itself is private |

## Verdict

Stay. The gain is one manual step that happens once per multi-PR feature and is already written down in `AGENTS.md`. The costs hit what ADR 0014 and ADR 0018 chose on purpose: one public place for the backlog, and nothing to set up beyond `gh`.

Look again if any of these change:

- Several regular contributors join and want cycles, triage or estimates.
- Linear offers a public view or free guests, so outside users can follow the backlog there.
- The manual move to `building` keeps being forgotten. A cheaper fix then is a workflow step on `pull_request` (opened, ready for review) that moves the issues named with `Part of #N`, on GitHub.

## Sources

- Linear pricing: https://linear.app/pricing
- Linear billing and plans: https://linear.app/docs/billing-and-plans
- Linear GitHub integration: https://linear.app/docs/github
- Linear GitHub importer: https://linear.app/docs/github-to-linear
- Linear archive: https://linear.app/docs/delete-archive-issues
- Linear MCP server: https://linear.app/docs/mcp
- Linear API: https://linear.app/docs/api-and-webhooks and https://linear.app/developers/rate-limiting
- Linear plugin for Claude Code: https://github.com/anthropics/claude-plugins-official (`external_plugins/linear`)
- Free plan limits, third party: https://t0ggles.com/blog/linear-free-plan-limits (2026-08-28)
- Outside reports kept on GitHub: https://handbook.cal.com/engineering/how-we-work-on-tickets and https://linear.app/integrations/synclinear
- This repo: `gh issue list --state all`, `AGENTS.md`, `.github/workflows/merge-when-green.yml`, `.github/pull_request_template.md`
