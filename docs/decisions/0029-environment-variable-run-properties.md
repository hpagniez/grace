---
status: accepted
date: 2026-11-04
decision-makers: Tusquito
---

# Pass environment variable run properties in the Karate process's environment

## Context and Problem Statement

[ADR 0008](0008-secret-run-properties-and-java-argument-file.md) keeps run properties encrypted with `safeStorage` and passes them to Java as `-D` options in an argument file, off the command line. Many suites read their secrets with `java.lang.System.getenv` instead, so the same values work in CI ([#230](https://github.com/hpagniez/grace/issues/230)). How do such properties reach Karate, and how are they stored?

## Considered Options

1. Set them in the environment of the Karate process Grace starts, from the same encrypted rows, marked `"envVar": true`.
2. Pass them as `-D` options and ask projects to read either: not what the suites do; it changes their code.
3. A launcher script that sets them and starts Java: a second process and a file holding the values, for what the spawn's environment does.

## Decision Outcome

Chosen: option 1. The Karate CLI leaves environment variables to the shell; Grace's spawn is that shell. The row keeps ADR 0008's storage and masking; only its transport differs.

### Consequences

- Good: no file and no command line argument for these values; nothing new on disk.
- Good: settings saved before keep loading: a row without `envVar` is a system property.
- Bad: unlike `-D` options, every process Karate itself starts (`karate.exec`, a forked mock server) inherits the variables.
- Bad: a process of the same Windows user can read another process's environment, as it can read its command line; the argument file of ADR 0008 is on disk for a run's length all the same.
- Neutral: values keep ADR 0008's printable ASCII rule, though an environment would take any Unicode, so the masker's forms stay exact.

## More Information

Spec: [docs/specs/2026-11-04-env-var-properties.md](../specs/2026-11-04-env-var-properties.md).
