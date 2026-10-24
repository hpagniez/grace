---
status: accepted
date: 2026-10-24
decision-makers: Tusquito
---

# Keep documentation as plain Markdown in the repo

## Context and Problem Statement

The app is built mostly by AI agents. They need docs they can read and update in the same change as the code, with no build step.

## Considered Options

1. Plain Markdown in `docs/` plus `AGENTS.md`.
2. Option 1 plus a Starlight site with `llms.txt`.
3. A spec-driven toolkit such as GitHub spec-kit.

## Decision Outcome

Chosen: option 1.

- Option 2 only pays off with outside readers. It can be added later on the same files.
- Option 3 overlaps the superpowers plugin's design → plan → build workflow already in use, and generates several files per feature that drift.

Layout:

```
AGENTS.md                 rules for agents; CLAUDE.md imports it
docs/
  features.md             backlog: id, priority, status, Karate surface, spec link
  inventory/              what Karate (OSS and paid) offers; source for features.md
  research/               dated research notes with sources
  decisions/NNNN-*.md     decision records, MADR 4.0 minimal template
  specs/YYYY-MM-DD-*.md   design specs, written only for the feature about to be built
```

### Consequences

- Good: zero tooling; GitHub renders it; agents read it directly.
- Bad: docs drift if agents skip them. Mitigated by the rule in `AGENTS.md` to update `docs/features.md` and the spec in the same change.
