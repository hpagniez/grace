---
summary: "Java memory limit (#149): run.memory.java@10k falls from 481.6 to 319.1 MB and the 10,000-test run passes; the bench still misses elsewhere"
---

# Bench: Java's memory limit (#149)

Date: 2026-11-01
Builds: A = `develop` at `73849d53` (merge of #172), B = `feature/java-memory` at `9b11bd60`, labelled `fc0dabe4` in the bench output: the same code, before one commit message on the branch was shortened (A plus `-Xms64m -Xmx2g -XX:+ExitOnOutOfMemoryError` on every Java Grace starts, and the out-of-memory notice). Both packaged with `npm run package` from their own checkouts; B was the build `npm run e2e` packaged.
Machine: the reference machine of the [baseline](2026-10-31-bench-baseline.md) (Windows 11 Pro, Ryzen 7 5800X, 32 GB). The bench logged `windows open at 3140, 246 on DISPLAY1, not the main screen (work area 2560 x 1392 at 2560, 0)`: the same right-hand screen, under the name Windows gave it this time.
Spec: [docs/specs/2026-11-01-java-memory.md](../specs/2026-11-01-java-memory.md).

**Result: `run.memory.java@10k` falls from 481.6 to 319.1 MB, and the 10,000-test run passes.** `run.memory.java@10k` is lowered from 529.2 to 341.1 MB (noise 22). The bench's `Result: miss` comes from `start.tree.fresh` and `smooth.expand.idle.*@10k`, which this change cannot reach (below).

## What was run

- `node --no-warnings bench/bench.mjs --release --a <A>\Grace.exe --label-a "develop 73849d53" --label-b "feature/java-memory fc0dabe4"`, from PowerShell. Release mode, two builds: one warm-up each, then 5 rounds each in the order a, b, b, a, a, b, b, a, a, b, 12:59 to 13:38. No zip given, so `size.zip` reads `not measured`; the change adds no file. The run is in [2026-11-01-java-memory.bench.json](2026-11-01-java-memory.bench.json).
- Machine load before each round: A 1.1 % to 7.9 %, B 4.5 % to 28.9 % (other sessions' agents were working; no test run or other bench ran).
- A first try at 12:47 overlapped another session's full `npm test` batch. It was stopped in its warm-up and its numbers were thrown away.
- Then, on the quiet machine, Java alone on the 10,000-test project, 5 interleaved rounds per flag set (`docs/research/2026-11-01-java-memory/measure.mjs`, raw in `cpu.jsonl`), to check Java's CPU.

## The run measures

Medians of 5 rounds; the noise is the limit file's.

| Measure | A | B | Change | Noise | Verdict |
|---|---|---|---|---|---|
| `run.memory.java` (MB) | 308 | 297.4 | −10.6 | 49 | same |
| `run.memory.java@10k` (MB) | 481.6 | 319.1 | −162.5 | 34 | better |
| `run.memory.app` (MB) | 296.5 | 297.6 | +1.1 | 23 | same |
| `run.memory.app@10k` (MB) | 577 | 579.6 | +2.6 | 29 | same |
| `run.cpu.java` (ms) | 16,969 | 18,094 | +1,125 | 4,453 | same |
| `run.cpu.java@10k` (ms) | 40,750 | 47,125 | +6,375 | 5,391 | see below |
| `run.cpu.app` (ms) | 3,953 | 4,484 | +531 | 1,281 | same |
| `run.cpu.app@10k` (ms) | 12,797 | 14,688 | +1,891 | 2,718 | same |
| `run.finishedLag` (ms) | 133 | 153 | +20 | 27 | same |
| `run.finishedLag@10k` (ms) | 317 | 365 | +48 | 118 | same |

Java's peak on 10,000 tests, per round: A 483, 497, 482, 470, 477; B 322, 314, 319, 304, 325.

### Java's CPU on 10,000 tests

In the bench, B's `run.cpu.java@10k` rounds read 47.1, 45.2, 47.8, 48.6 and 41.0 s against A's 43.5, 39.9, 41.4, 39.2 and 40.8 s: 6.4 s more, just over the noise. B's rounds also ran on a busier machine (4.5 % to 28.9 % load before them, against 1.1 % to 7.9 %), and its last round, 41.0 s, matched A.

The check with Java alone on a quiet machine, 10,000 tests, one thread, 5 interleaved rounds:

| Flags | Java CPU, median (range) | Peak MB, median | Collections |
|---|---|---|---|
| none (A) | 34.6 s (33.8 to 36.7) | 465 | 21 to 27 |
| `-Xms64m -Xmx2g` (B) | 36.3 s (34.6 to 37.6) | 318 | 36 to 50 |
| `-Xms128m -Xmx2g` | 35.3 s (34.0 to 38.0) | 344 | 25 to 33 |

So the cost is real but small: about 1.7 s (5 %) of Java CPU on 10,000 tests for 147 MB (32 %) less memory, from the extra young collections of the smaller heap, and inside the rounds' spread. The run took the same time (16.0 s against 16.4 s). A 128 MB start halves the CPU cost and gives back 26 MB; 64 MB stays, since memory and CPU both count and the memory gain is the larger.

## The misses

- `start.tree.fresh`: A 1,788, B 2,058 ms (limit 1,963). B's rounds read 2,408, 2,388, 1,755, 2,058, 1,723 ms; `start.tree.reused`, which runs the same dry run, reads 1,946 against 1,943. The first two of B's fresh starts were slow and the last three as fast as A's, which reads as the machine, not the flags; the Java-only check found the 10,000-test dry run faster with the flags (5.8 s against 6.2 s).
- `smooth.expand.idle.paint@10k` (A 47, B 61, limit 53) and `smooth.expand.idle.frame@10k` (A 38, B 50, limit 47): these are taken after the tree is shown and before Run all, when no Java runs, so Java's flags cannot change them. B's last round read 46 and 42, like A's.

## Limits

`run.memory.java@10k`: 529.2 → 341.1 MB, noise 34 → 22, from B's rounds by `makeLimits`' rule (median 319.1 plus the rounds' spread, 21.3, rounded up). The 2,000-test `run.memory.java` stays at 356.4: the change moved it by 10.6 MB, inside the noise, and A's rounds there spread over 84 MB, so a limit from B's narrow spread would catch noise. No other limit changes.

## What still needs a person

- Nothing for this change. The next release bench will show whether `start.tree.fresh` and `smooth.expand.idle.*@10k` miss again; if they do, they are not this change's.
