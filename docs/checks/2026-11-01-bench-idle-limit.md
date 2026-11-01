# Bench: idle CPU after the still cursor, a lower `idle.cpu.shown`

Date: 2026-11-01
Commit measured: `d8b0cf60` (merge of #171, the console cursor that blinks only while Grace works; `package.json` version 1.0.0-rc.7). Its app code is the same as `develop` at `510cd723` (merge of #167), which added only the bench, workflows and docs.
Machine: the reference machine of the [baseline](2026-10-31-bench-baseline.md) (Windows 11 Pro, Ryzen 7 5800X, 32 GB), same screen layout. The bench logged `windows open at 3140, 246 on DISPLAY2, not the main screen (work area 2560 x 1392 at 2560, 0)`. That is the same right-hand screen as the baseline, under the name Windows now gives it.

**Result: `idle.cpu.shown` goes from at most 3,343 ms (noise 1,781) to at most 1,453 ms (noise 1,375).** No other limit changes.

## What was run

- `npm run make` on the branch, then `node --no-warnings bench/bench.mjs --release --b out/Grace-win32-x64/Grace.exe --b-zip out/make/zip/win32/x64/Grace-win32-x64-1.0.0-rc.7.zip --label-b "develop d8b0cf60"`. Release mode, one build: 1 warm-up and 5 rounds. `Result: pass` against the limits. The run is in [2026-11-01-bench-idle-limit.bench.json](2026-11-01-bench-idle-limit.bench.json).
- Machine load before each round: 2.7 % to 5.3 %.
- A first try of the same command, started from Git Bash, ended in round 3 with exit 127 and no message, and left no process behind. The first baseline's first try ended the same way. The second try, started from PowerShell, ran to the end.

## `idle.cpu.shown`

CPU ms over 60 s, shown and idle, rounds 1 to 5: 16, 31, 1,391, 125, 78. The median is 78 ms; the baseline's was 1,562. Round 3 has one spike: nothing in the run's log explains it, and its load before the round was 3.1 %. By `makeLimits`' rule the noise is the rounds' spread, so the spike sets the margin: 78 + 1,375 = 1,453. A later run without such a spike can lower it again by an ordinary commit.

Only this limit changes, since it is what #171 fixed. A full `--limits-from` over this run would have changed the others both ways: lower for some (for example `smooth.run.frame` 33 → 22 ms), higher for others within run-to-run noise (for example `start.paint.fresh` 598 → 1,029 ms, `run.memory.java` 356.4 → 433.8 MB). A raise needs a decision record, and none is called for.
