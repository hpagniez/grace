# Java's JIT during a Karate run: C1 only, fewer compiler threads

Checked on 2026-11-01 and 2026-11-02 on the reference desktop (Windows 11 Pro, Ryzen 7 5800X, 16 threads, 32 GB), with the bundled Temurin 21.0.12.1 and Karate 2.1.4, for [#179](https://github.com/hpagniez/grace/issues/179). Spec: [docs/specs/2026-11-02-java-jit.md](../../specs/2026-11-02-java-jit.md).

**Bottom line:** C1 alone (`-XX:TieredStopAtLevel=1`) cuts Java's CPU by 53 to 75 % on the bench's projects, but runs a suite of heavy JavaScript about 4 times slower (8 threads: 6.5 times), with Java busy all along. A dry run runs no JavaScript, so C1 alone suits it. Two compiler threads (`-XX:CICompilerCount=2`: one C1, one C2, against 12 by default here) cut Java's CPU by 10 to 52 % and its peak by 70 to 195 MB on the bench's projects, and changed nothing on heavy JavaScript; 10,000 tests on one thread ran about 1 s slower than with no flag in most paired rounds.

## Method

`measure.mjs` is the [Java memory research's](../2026-11-01-java-memory/README.md) script with one fix: it listens for Java's exit before it waits for the sampler's first sample, since a 2,000-test run could end first and leave the script waiting forever. It starts Java as `buildArgs` does and samples Java's private working set and CPU every 250 ms with `bench/system.mjs`, the measures `npm run bench` reports as `run.memory.java` and `run.cpu.java`. `rounds.sh` runs the three flag sets in turn, each after Grace's heap flags (`-Xms64m -Xmx2g -XX:+ExitOnOutOfMemoryError`): no flag (`today`), `c1`, `cc2`.

The projects: the bench's 2,000- and 10,000-test ones (`writeBigProject`), on 1 or 8 threads or as a dry run; and `js-project.mjs`'s suite of heavy JavaScript, 100 features of 10 scenarios, each a loop of 300,000 turns in a function from `karate-config.js`, a list of 2,000 objects built with `karate.repeat`, filtered, mapped, sent through `JSON.stringify` and `JSON.parse`, and matched. Every scenario passes. On one thread it takes about 80 s.

`explore.jsonl` holds the first single runs; `rounds.jsonl` 3 interleaved rounds of every case; `rounds-2.jsonl` 3 more of 10,000 tests on one thread, and the JavaScript suite on 8 threads. The first round of `rounds-2.jsonl`'s `cc2` on 10,000 tests overlapped a typecheck.

```bash
node js-project.mjs
bash rounds.sh <repo> 3 rounds.jsonl 2k:1 10k:1 10k:8 10k:dry js:1
node ../2026-11-01-java-memory/summary.mjs rounds.jsonl   # groups a dry run with T1: split by "dry" by hand
```

## The candidates

Java's CPU, peak and wall time, median of 3 rounds (10,000 tests on one thread: 6):

| Case | No flag | `-XX:TieredStopAtLevel=1` | `-XX:CICompilerCount=2` |
|---|---|---|---|
| 2,000 tests, 1 thread | 13.7 s, 303 MB, 4.6 s | 4.4 s, 144 MB, 3.6 s | 8.3 s, 177 MB, 4.4 s |
| 10,000 tests, 1 thread | 29.3 s, 333 MB, 11.1 s | 13.9 s, 222 MB, 11.9 s | 26.4 s, 263 MB, 14.0 s |
| 10,000 tests, 8 threads | 49.8 s, 617 MB, 6.5 s | 20.2 s, 391 MB, 4.8 s | 28.4 s, 422 MB, 6.7 s |
| 10,000 tests, dry run | 11.5 s, 297 MB, 3.4 s | 2.9 s, 207 MB, 3.3 s | 5.5 s, 221 MB, 3.8 s |
| Heavy JavaScript, 1 thread | 86.4 s, 336 MB, 82.2 s | 316.6 s, 190 MB, 315.5 s | 82.5 s, 335 MB, 76.6 s |
| Heavy JavaScript, 8 threads | 102.2 s, 952 MB, 12.5 s | 637.2 s, 415 MB, 81.6 s | 98.9 s, 864 MB, 12.3 s |

10,000 tests on one thread, wall time per round, no flag against two threads: 19.5 / 15.8, 13.6 / 16.1, 11.4 / 12.4, 10.8 / 15.6 (the typecheck), 10.1 / 10.6, 10.2 / 11.3 s. The median of the paired differences is 1.0 s longer with two threads. Java's CPU: 34.7 / 30.0, 33.6 / 29.8, 30.3 / 24.0, 28.3 / 28.8, 25.3 / 20.8, 27.4 / 21.6 s.

The dry run with two threads took longer than with no flag in every round (3.5 to 4.0 s against 3.2 to 3.5); with C1 alone it took as long as with no flag.

## Compiler threads

`java -XX:+PrintFlagsFinal -version` with `-XX:ActiveProcessorCount`: Java starts 12 compiler threads on 16 processors, 4 on 8 and 3 on 4, a third of them C1 (at least one). `-XX:CICompilerCount=2` is the smallest count it accepts with both tiers: one of each. On 4 processors it takes one C2 thread away; on 16, three C1 and seven C2 threads.

## Class-data sharing

Not measured again. The [Java memory research](../2026-11-01-java-memory/README.md#class-data-sharing) found a dry run of 2,000 tests 200 ms faster with an archive Java writes on the first run, for 19 MB more at the peak and 9.8 MB on disk.
