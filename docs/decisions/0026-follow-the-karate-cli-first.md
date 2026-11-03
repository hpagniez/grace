---
status: accepted
date: 2026-11-02
decision-makers: Tusquito
---

# Follow the Karate CLI first

## Context and Problem Statement

Grace runs Karate's own jar ([ADR 0006](0006-bundle-jre-and-karate-jar.md)) on Maven, Gradle, standalone and Karate CLI projects. Each feature that touches how a project runs (where its tests, config or libraries are) can either follow what the Karate CLI does or invent Grace's own way. Designing the dependency jars of [#64](https://github.com/hpagniez/grace/issues/64), the first proposal ran the project's Maven or Gradle to list them; the CLI already has a way: its extension folders. Which comes first?

## Considered Options

1. The Karate CLI's way first: for every feature and every kind of project, do what the Karate CLI (the `karate` launcher and the jar's `run`) does; build Grace's own way only where the CLI has none.
2. Per feature, whichever way fits each build tool best.

## Decision Outcome

Chosen: option 1, the owner's rule.

- A design spec starts from what the CLI does (`docs/inventory/karate-core.md`, Karate's `docs/CLI.md`, `RunCommand.java`) and gives it as the first approach. Another approach needs a reason the CLI's cannot serve.
- It holds for Maven and Gradle projects too: a project that runs with `karate run` runs the same in Grace.

### Consequences

- Good: one behaviour to learn, documented by Karate; a project moves between Grace and the CLI unchanged.
- Good: usually less to build and nothing to run: no settings of Grace's own.
- Bad: where the CLI leaves a step to the person (copying a Maven project's jars into `.karate/ext/`), Grace's own way comes on top of the CLI's and must keep the CLI's working: two mechanisms instead of one ([ADR 0027](0027-list-project-libraries-with-its-own-build-tool.md)).

## More Information

Applied first in [docs/specs/2026-11-02-project-libraries.md](../specs/2026-11-02-project-libraries.md). Grace's existing behaviour that predates this rule, such as the Maven and Gradle source and compiled folders on the classpath ([spec](../specs/2026-11-02-project-setup.md)), is unchanged by this record.
