# The owner's machine

The reference machine of `bench/limits.json` is the owner's own PC, and he works on it. It is the only place for the release benchmark, the Windows by-hand rows and the measures CI's runners cannot give.

- Ask the owner before each run that opens windows, needs a quiet machine or installs something: e2e, the bench, a measure, a by-hand check, a toolchain. Say how long it takes and what will show on screen. One yes covers one run. Builds, unit tests and git need no ask.
- Work in separate worktrees under one temp folder, never in his working copy, and with a fresh data folder (`GRACE_USER_DATA`), never his settings. Copy `vendor/` into a worktree, never link it: a recursive delete through a junction empties the checkout's own `vendor/`.
- Right after the work, before anything else, remove everything it made: worktrees, temp folders, build outputs, logs, local branches, toolchains it installed. Delete only what the work made. Then check `git worktree list`, `vendor/` and the process list, and tell him what was removed.
- A check record says what ran there and with whose go. Keep his user name, his paths and captures of his desktop out of it.
