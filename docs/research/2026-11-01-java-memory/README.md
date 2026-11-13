---
summary: "Java memory: -Xms64m -Xmx2g cuts the 10,000-test peak from 455 to 318 MB for 5% more CPU; serial GC does not help, class-data sharing costs 19 MB"
---

# Java's memory during a Karate run: heap flags, collector, class-data sharing

Checked on 2026-11-01 on the reference desktop (Windows 11 Pro, Ryzen 7 5800X, 16 threads, 32 GB), with the bundled Temurin 21.0.12.1 and Karate 2.1.4, for [#149](https://github.com/hpagniez/grace/issues/149). Spec: [docs/specs/2026-11-01-java-memory.md](../../specs/2026-11-01-java-memory.md).

**Bottom line:** the suites need little heap (under 120 MB left after a collection, on 10,000 tests and up to 32 threads). Most of Java's peak is young-generation space that the default 512 MB starting heap lets G1 fill before it collects, plus the JIT's working memory. `-Xms64m -Xmx2g` cuts the 10,000-test peak from 455 to 318 MB and the dry run's from 471 to 238 MB, for about 5 % more of Java's CPU and no new risk of running out of memory on any machine with 8 GB or more. The serial collector does not help. Class-data sharing starts Java about 200 ms sooner but costs about 19 MB. C1 alone halves Java's CPU: a follow-up.

## Method

`measure.mjs` starts Java as `buildArgs` does (same flags, `-f html,karate:jsonl`, the project as the classpath, the option variables removed), plus the flags under test and `-Xlog:gc` to a file. It samples Java's private working set every 250 ms with `bench/system.mjs`' sampler, the measure `npm run bench` reports as `run.memory.java`, and reads Java's CPU time the same way. From the GC log it takes the largest heap left after a collection (about the live data) and the largest committed heap. The projects are the bench's, from `writeBigProject` (5 features per module: 2,000 tests; 25: 10,000), each run on one or more threads (`-T`), or as a dry run (`-D`). No app runs: these are Java's own figures.

`explore.jsonl` holds the first single runs; `rounds.jsonl` the three candidates in 3 interleaved rounds (2 for the dry run) and one run each on 16 and 32 threads; `cpu.jsonl` 5 rounds on Java's CPU; `cds.jsonl` the class-data sharing check. `summary.mjs <file>` prints the medians.

```bash
node --no-warnings measure.mjs <repo> 10k 1 b -- -Xms64m -Xmx2g
node summary.mjs rounds.jsonl
```

## Where the memory goes

Native Memory Tracking at exit, 2,000 tests, 1 thread, `-Xmx64m` (`-XX:NativeMemoryTracking=summary -XX:+PrintNMTStatistics`), committed MB:

| Area | MB |
|---|---|
| Arena chunks (the JIT compilers' working memory) | 122 |
| GC (G1's own structures; mostly committed, little touched) | 63 |
| Java heap | 53 |
| Code cache | 27 |
| Metaspace | 24 |
| Shared class space (the JDK's CDS archive) | 12 |
| Compiler, symbols, threads, other | 21 |

Even with a 64 MB heap the run peaked at 284 MB of private working set, so the heap is only part of the peak.

## The candidates

Java's peak in MB, median of 3 rounds (the dry run: 2), Java CPU in seconds:

| Case | No flag | `-Xmx256m` | `-Xms64m -Xmx2g` |
|---|---|---|---|
| 2,000 tests, 1 thread | 305 (16.8 s) | 244 (16.9 s) | 291 (13.5 s) |
| 10,000 tests, 1 thread | 455 (29.0 s) | 333 (34.5 s) | 318 (29.9 s) |
| 2,000 tests, 8 threads | 384 (16.0 s) | 223 (14.9 s) | 263 (14.2 s) |
| 10,000 tests, 8 threads | 623 (51.7 s) | 504 (53.6 s) | 466 (48.4 s) |
| 10,000 tests, dry run | 471 (12.2 s) | 309 (11.9 s) | 238 (13.3 s) |
| 10,000 tests, 16 threads (1 run) | | 494 | 571 |
| 10,000 tests, 32 threads (1 run) | | 433 | 535 |

The largest heap left after a collection: 16 to 27 MB on 2,000 tests, 45 to 116 MB on 10,000. The spread between rounds was large (2,000 tests, 1 thread, no flag: 233 to 307 MB), so a single run says little.

Other flags, single runs on 10,000 tests, 1 thread (no flag: 434 to 471 MB, 29 to 41 s of CPU):

| Flags | Peak MB | Java CPU s |
|---|---|---|
| `-Xmx128m` | 334 | 47.9 |
| `-Xmx192m` | 329 | 44.7 |
| `-Xmx384m` | 330 | 43.9 |
| `-XX:+UseSerialGC` | 439 | 32.5 |
| `-XX:+UseSerialGC -Xmx256m` | 345 | 36.0 |
| `-XX:CICompilerCount=2` | 432 | 29.5 |
| `-XX:TieredStopAtLevel=1` (C1 only) | 382 | 16.4 |

Below 256 MB the peak stops falling and garbage collection costs more CPU. `-Xmx512m` and `-Xmx1g` change nothing on this machine: the starting heap is already 512 MB.

## Java's CPU, a second look

The release bench read more of Java's CPU on 10,000 tests with the flags, on a busier machine. Java alone on a quiet machine, 10,000 tests, one thread, 5 interleaved rounds (`cpu.jsonl`): no flag 34.6 s (465 MB, 21 to 27 collections), `-Xms64m -Xmx2g` 36.3 s (318 MB, 36 to 50), `-Xms128m -Xmx2g` 35.3 s (344 MB, 25 to 33). The smaller start costs about 5 % of Java's CPU in extra young collections, inside the rounds' spread; the wall time did not change.

## Class-data sharing

`-XX:SharedArchiveFile=<file> -XX:+AutoCreateSharedArchive` with the target flags: the first dry run wrote a 9.8 MB archive (the project's folders on the classpath did not stop it). Then 4 interleaved dry runs of 2,000 tests each way: 1,333 ms with the archive against 1,539 ms without (3.1 s of CPU against 3.5 s), and a peak of 187 MB against 168 MB.

## Running out of memory

With `-XX:+ExitOnOutOfMemoryError`, a step that asks for one array of about 2 GB (`Java.type('java.lang.reflect.Array').newInstance(Java.type('java.lang.Byte').TYPE, 2147483000)`) makes Java print `Terminating due to java.lang.OutOfMemoryError: Java heap space` and exit with code 3 at once; the next scenario never runs. A dry run does not run `karate-config.js`: the same allocation there leaves the dry run passing.
