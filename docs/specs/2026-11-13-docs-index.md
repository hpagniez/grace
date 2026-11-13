---
status: built
issue: 373
summary: "Frontmatter on specs, plans, checks and research, and a generated README index per docs folder, so agents and people find the live doc in one read"
---

# Docs index: find the right doc in one read

## Why

`docs/` holds 193 Markdown files and 29,232 lines. The owner sees agents fail in three ways:

1. They miss a doc that exists.
2. They act on a stale one: a finished plan or an old spec reads like a live one.
3. They read too much: they open whole files to learn whether they matter.

Agents are the first reader. The owner reads the same files on GitHub, and may read them in Obsidian.

Today only decision records carry a `status` in frontmatter. Specs keep theirs in a prose line; plans, checks and research notes have none. No folder has an index.

## Goals

1. One read of a folder's index tells an agent which docs matter and which are live.
2. A doc's status has one home and stays true.
3. The same files serve a person on GitHub and in Obsidian, with no second copy.

## Non-goals

- A search tool or a docs site ([ADR 0002](../decisions/0002-plain-markdown-docs.md) stands: plain Markdown, read with no tooling).
- A dead-link check: no broken link is known today; it is its own PR if one shows.
- A `paths:` field from code to docs: an agent searches the index lines first. Add it on the matcher of `crates/xtask/src/rules.rs` if agents still miss docs.
- Any change to `docs/inventory/`, `docs/process/` and `docs/features.md`: the docs map already names them.
- Any Obsidian file in the repository.

## Anti-goals

- An index written by hand: it drifts.
- A status that says `approved` after the feature is built.
- More text loaded in every session.

## Approaches

1. **Frontmatter and a generated index file per folder. Chosen by the owner on 2026-11-13.** One read for an agent; GitHub shows the folder's `README.md`; Obsidian reads the fields. Cost: two PRs that each add a doc both change an index, and the second regenerates it.
2. **Frontmatter and a command only**, nothing generated. No conflict and no drift, but nothing to browse on GitHub, and an agent with no Rust toolchain ready pays a build to ask.
3. **A search tool** (full-text or embeddings behind an MCP server, or Obsidian's). A process on each machine, absent in cloud sessions, against ADR 0002, and still blind to stale docs without a status.
4. **An archive folder for finished docs.** Breaks the links from issues, PR bodies and other docs.

## Requirements

### R1. Frontmatter

R1.1. An entry is a tracked file of one of these kinds:

| Folder | Entries | Fields |
|---|---|---|
| `docs/specs/` | each `*.md` | `status`, `summary`, `issue` |
| `docs/plans/` | each `*.md` | `status`, `summary`, `issue` |
| `docs/checks/` | each `*.md` | `summary` |
| `docs/research/` | each `*.md`, and the `README.md` of each folder directly under it | `summary` |
| `docs/decisions/` | each `*.md` | none added: `status` as today, the title as the summary |

A folder's own `README.md` is the index, not an entry. Other files (images, attachment folders under `docs/checks/`) are not entries.

R1.2. The frontmatter is YAML between the first two `---` lines, one field per line.

R1.3. `status` of a spec or a plan is one of:

| Value | Means |
|---|---|
| `draft` | written, not approved by the owner |
| `approved` | approved, not yet fully built |
| `built` | the PR that finishes it is merged |
| `superseded` | replaced or given up; the text says by what |

R1.4. `summary` is one line of at most 160 characters, in double quotes, with no `"` and no `\` inside: valid YAML whatever it says. It says what the doc decides or found, not what it is about: a check's result, a research note's verdict, a spec's behaviour.

R1.5. `issue` is the issue number, or a list `[64, 231]`. It is left out only when the work has no issue.

R1.6. The prose `Status:` of today's specs goes: the frontmatter is its one home. The state of long work stays in its tracking issue.

R1.7. Accepted decision records are not edited.

### R2. The index

R2.1. `cargo xtask docs` writes `README.md` in `docs/specs/`, `docs/plans/`, `docs/checks/`, `docs/research/` and `docs/decisions/`. Each starts with a line that says it is generated and by which command.

R2.2. One line per entry: a relative Markdown link with the doc's title, then its issue when it has one, then its summary.

R2.3. Specs and plans are grouped under a heading per status, in this order: `approved`, `draft`, `built`, `superseded`. Decisions are grouped by status, `accepted` first. Inside a group, and in checks and research, the newest comes first.

R2.4. The command's output depends only on the tracked docs: the same docs give the same bytes on Windows and Linux.

### R3. The check

`cargo xtask check-rules` also fails, with one message per failure, when:

- an entry lacks a field of R1.1, or holds a `status` outside R1.3 or a `summary` outside R1.4;
- a folder's `README.md` is not what `cargo xtask docs` writes. The message names the command.

It reads the git index, as it does today.

### R4. Status stays true

R4.1. `cargo xtask check-pr` fails a PR whose body says `Closes #N` and names a `Spec:` or a `Plan:` whose status is not `built`. A doc with an `issue` the PR does not close is left as it is: the guide of a project when one phase closes, a spec of several issues until the last one closes.

R4.2. A spec-only PR opens with `draft`. The session that gets the owner's approval sets `approved`.

### R5. Agents are told

R5.1. `.claude/rules/docs.md` holds the rules: the fields, the statuses, the command, and how to read an index. A `built` or `superseded` plan is history. A `built` spec is how the feature behaves. A `superseded` spec is history.

R5.2. The area rules files `electron.md`, `gpui.md` and `karate.md` each get one line: read the specs and decisions indexes first, and open only the docs whose line matches. `both-apps.md` gets none: every path it covers is also under `electron.md` or `gpui.md`. `AGENTS.md` gets none: it is 8,996 bytes of its 9,000.

R5.3. On a merge conflict in an index, run `cargo xtask docs` and commit the result.

### R6. The existing docs

The same PR gives every entry its fields (146 files: 47 specs, 50 plans, 32 checks, 17 research) and adds the five indexes. A spec or plan is `built` when its issue is closed by a merged PR; the builder checks each with `gh`, and lists for the owner any it cannot decide.

### R7. Obsidian

`.obsidian/` goes in `.gitignore`. Nothing else: the frontmatter is valid YAML, and every link is a relative Markdown link.

## Acceptance checks

- `cargo xtask docs` on a clean checkout changes no file.
- Removing `summary` from one spec makes `cargo xtask check-rules` fail and name the file and the field.
- Editing a folder's `README.md` by hand makes `cargo xtask check-rules` fail and name `cargo xtask docs`.
- A new spec with `status: draft` appears under Draft in `docs/specs/README.md` after `cargo xtask docs`, with its title, issue and summary.
- A PR body with `Closes #N` and a `Spec:` whose status is `approved` fails `cargo xtask check-pr`; with `Part of #N` it passes.
- No file under `docs/decisions/` other than its `README.md` changes.
- `AGENTS.md` and `WORKFLOW.md` do not change.
- Opening `docs/specs/` on GitHub shows the index.

## Left to the builder

- The module layout in `crates/xtask`, on the standard library only, with unit tests on pure functions as the other commands have.
- The exact text of the index lines, the headings and the error messages.
- How to write the 146 summaries: a subagent per folder, on the smallest model that does it well.

## Decisions made by the agent

- No decision record: this changes how agents work, which is a rule change, and it is undone by deleting the fields and the indexes.
- Decision records get no `summary` field: their titles already state the decision, and accepted records are not edited.
- Checks and research notes get no `status`: they are records of a date.
- R4.1 is in: without it the status drifts, which is failure 2.

## Cost

Cost: none. No change to either app.
