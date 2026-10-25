---
status: accepted
date: 2026-10-25
decision-makers: Tusquito
---

# Keep run history as one folder per run with its own run.json

## Context and Problem Statement

F10 keeps the last runs of each project so they can be listed, their reports opened, and re-run. Until now each project had one run folder, emptied before every run, and nothing about a run outlived the window. The history must survive restarts, stay bounded on disk, and never let a bad write lose every run. Data stored from now on also feeds trends (F15), so its layout is hard to change once runs accumulate.

## Considered Options

1. One folder per run (`projects/<hash>/runs/<yyyyMMdd-HHmmss-SSS>/`), passed to Karate as `-o`, with an app-written `run.json` in it.
2. Karate's own report backups (`-B true`), which rename the previous output folder when the next run starts.
3. One `history.json` index per project plus the run folders.

## Decision Outcome

Chosen: option 1, keeping the 20 newest folders per project.

- A damaged `run.json` loses one run; listing reads at most 20 small files; retention is deleting folders.
- `-B true` stamps a folder with the time of the next run, has no retention, and still needs a summary file of our own.
- A shared index is rewritten by every run, so one bad write loses all history, and it can drift from the folders.
- `run.json` holds the run's environment, tag filter, selection, outcome and each test's status and duration, never property values, error text or logs. It carries `version: 1`.
- Re-run failed passes its `file:LINE` targets in a picocli argument file (`@run-targets.txt`, UTF-8, quoted, `\` doubled), because hundreds of targets overflow Windows' 32,767-character command line. Probed on 2026-10-25 with Karate 2.1.4 and Java 21: paths with a space and an `é` work; a missing file runs nothing and exits 0.

### Consequences

- Good: no new dependency; each run's report stays where Karate wrote it.
- Good: per-test results are stored from day one, ready for F15.
- Bad: F15 sees at most the 20 newest runs of a project until a later decision widens the storage.
- Bad: up to 20 copies of Karate's report per project, which can contain secret values unless the project sets `configure logging mask`.
- Bad: a future change to `run.json` needs a version bump and reading old versions, or old runs drop out of the list.
- Neutral: the single run folder of version 1.0 is deleted; history starts empty.
