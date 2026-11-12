# Way of working

General rules for agents, for any project. The project's own rules are in `AGENTS.md`. Names in `code` are superpowers skills: use them where that plugin is installed; the steps stand without it.

## Tiers

Name the tier in your first message of a task; the owner can overrule it. When the task outgrows its tier, stop, say so and step up.

| | Small | Feature | Project |
|---|---|---|---|
| When | A fix, a doc or config change, a dependency bump, a refactor that changes no behaviour; nothing hard to reverse | New behaviour a person using the product sees, or a change across several parts | Several PRs, a hard-to-reverse choice (dependency, architecture, file format), or work no one can check from one diff |
| Design | Two or three lines in the PR body | A short design file the owner approves (`brainstorming`) | A full design, read by a reviewer agent, then approved by the owner; a decision record per hard-to-reverse choice |
| Plan | None | A plan file (`writing-plans`) | A plan file per phase |
| Build | In the session | In the session (`executing-plans`) | An agent per task where tasks are independent (`subagent-driven-development`), else in the session |
| Review | Your own check | One fresh reviewer on the whole branch (`requesting-code-review`) | A reviewer per task, then one on the whole branch |
| Issue | Only for a reported bug or unfinished work | One | One to track, one per phase |

## The floor, at every tier

- Test first for any change in behaviour: the failing test, then the code (`test-driven-development`). A doc, config or mechanical change needs none.
- Find a bug's cause before fixing it (`systematic-debugging`).
- Call nothing done, fixed or passing without the command's output in the same message (`verification-before-completion`).
- One branch, one PR, one logical change per commit.

## The repository is the memory

A decision goes in a decision record, a rule in the rules files, the state of long work in its tracking issue, edited in place. Write a rule the owner gives in conversation into the rules files in the same session. Nothing an agent needs later lives only in a conversation or a plugin's memory. One to-do list per session: the one the app shows.

## Subagents

Use one only for independent work, or work that would flood the session with file contents. Look-ups go to the Explore agent, which loads no rules.

| Task | Model | Effort |
|---|---|---|
| Finding code, look-ups, renames | the smallest | low |
| A plan task with a precise brief, tests, a review of one task | the middle one | medium |
| Design, an unknown cause, a whole-branch review | the largest the project allows | high |

Move up one step when the work comes back weak; never retry on the same one.

## Scripts

- One script command and one home per project; its `AGENTS.md` names them.
- Rust first, then Python, then JavaScript. A project that sets no language uses the first it already builds with. Go one language down only when the one above cannot do the job (it must run inside another runtime), and say why in the PR. No new shell script: a CI step is a one-liner calling the script command.
- In any language: start a program with an argument list, never through a shell; the standard library before a dependency; every script has tests.
- Never hard-code where a toolchain lives or its version: the version is the language's pin file, the place the toolchain's own variables.

## Housekeeping

Done means clean:

- A task ends with its PR merged or closed, its branch deleted locally and on the remote, its worktree removed, the default branch up to date.
- Nothing is parked: a branch has a PR within a day; a PR untouched for 7 days is finished or closed with a note.
- Everything made outside the checkout goes under `<temp>/agent-work/<repository>/<task>/` and is removed when the task ends. Never make a junction or symbolic link.
- A leftover is something with no open PR behind it: a branch, a worktree, a stash, untracked files on the default branch, a scratch folder. Work in progress is not one. Run the project's tidy check, when it has one, at the start of a session.
- Delete on your own only what cannot lose work: a merged branch (`git branch -d`), a clean worktree of a merged branch, an empty folder. Show anything else to the owner and ask.

## The person's machine

Ask before a run that opens windows, needs a quiet machine or installs something. Say how long it takes and what will show. One yes covers one run. Builds, unit tests and git need no ask.

## Pull requests

Draft while working; ready when the project's checks pass; merged only on the owner's word or the project's merge rule; the branch deleted with the merge.

When sources disagree: the project's rules, then this file, then superpowers, then ponytail.
