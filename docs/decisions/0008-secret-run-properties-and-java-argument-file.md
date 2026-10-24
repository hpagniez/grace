---
status: accepted
date: 2026-10-25
decision-makers: Tusquito
---

# Store secret run properties with Electron safeStorage and pass all run properties in a Java argument file

## Context and Problem Statement

F08 lets a person set system properties for runs, per environment, and mark some as secret (tokens, passwords). Karate reads them with `karate.properties`, so they must reach the JVM as `-D` options. A `-D` argument is visible to any process that lists command lines, and a plain settings file is readable by anything that reads the user's files. Where are secrets kept, and how do properties reach Java?

## Considered Options

Storage:
1. Electron `safeStorage` (Windows DPAPI), ciphertext in `settings.json`.
2. Windows Credential Manager through a native module (`keytar`).
3. The app's own encryption with a key file.

Transport:
1. All properties in one Java argument file (`java @file`), deleted after the run.
2. Plain properties as arguments, secrets in the file.
3. All properties as `-D` arguments.

## Decision Outcome

Chosen: storage option 1 and transport option 1.

- `safeStorage` is built into Electron and ties the ciphertext to the Windows user. `keytar` would add a native module rebuilt for each Electron version, and it is archived. A key file next to the data protects nothing.
- One argument file keeps every value off the command line with a single, tested escaping path. Checked on 2026-10-25 with the bundled Java 21: `java @args.txt` passed `"-Da=x y \"q\""` exactly; inside quotes `\` must be doubled; the file is read in the Windows code page, not UTF-8.

### Consequences

- Good: no secret on the command line, on screen after entry, or in plain text in settings.
- Good: no new dependency.
- Bad: settings with secrets cannot move to another Windows account, machine or app data folder (Electron keeps the DPAPI-protected key in the app data folder's `Local State`); secrets must be entered again there.
- Bad: the plain value exists on disk for the length of a run, in a user-only folder.
- Bad: values must survive the code page; ones that cannot are refused.
- Neutral: Karate's own reports can still contain secrets unless the project masks them with `configure logging mask`.
