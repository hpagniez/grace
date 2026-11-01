# Java memory

Date 2026-11-01. Status: approved. Covers [#149](https://github.com/hpagniez/grace/issues/149).

## Intent

Grace starts the bundled Java (Temurin 21) for every dry run and every run with no memory flag. The JVM then sizes itself from the machine: a heap that starts at 1/64 of the RAM and may grow to a quarter of it. On the 32 GB reference machine that is a 512 MB starting heap and an 8 GB ceiling. Grace stays open all day beside IDEs, browsers and containers, and running lightly comes first ([ADR 0023](../decisions/0023-run-lightly-as-a-release-requirement.md)): a run should take the memory its suite needs, and a broken or greedy suite should never take a quarter of the machine.

## What the measurements show

Measured on the reference machine (Windows 11 Pro, Ryzen 7 5800X, 32 GB), Karate 2.1.4 started the way Grace starts it, without the app, on the bench's 2,000- and 10,000-test projects (`e2e/big-project.ts`). Peak memory is Java's private working set, sampled every 250 ms as `npm run bench` counts it (`run.memory.java`). Scripts and raw numbers: [docs/research/2026-11-01-java-memory/](../research/2026-11-01-java-memory/README.md).

- **The suites need very little heap.** The largest heap left after a garbage collection was 17 to 27 MB on 2,000 tests and 45 to 116 MB on 10,000 tests, on 1 to 32 threads.
- **Most of the peak is not live data.** With no flag, G1 fills the young part of the 512 MB starting heap before it collects, so the run touches 300 MB or more of heap it does not need. The rest of the peak is outside the heap: the JIT compilers' working memory (about 120 MB at its peak), compiled code (about 27 MB) and class metadata (about 24 MB).
- **A smaller heap cuts the peak, but only to a point.** Java's peak in MB, median of 3 rounds (the dry run: 2):

  | Case | No flag (today) | `-Xmx256m` | `-Xms64m -Xmx2g` |
  |---|---|---|---|
  | 2,000 tests, 1 thread | 305 | 244 | 291 |
  | 10,000 tests, 1 thread | 455 | 333 | 318 |
  | 2,000 tests, 8 threads | 384 | 223 | 263 |
  | 10,000 tests, 8 threads | 623 | 504 | 466 |
  | 10,000 tests, dry run | 471 | 309 | 238 |

  Java's CPU stayed within the rounds' spread in every case. Going below 256 MB saved nothing more and cost more CPU in garbage collection (10,000 tests: 48 s at 128 MB against 31 to 37 s). On 2,000 tests on one thread the peak barely moves, because the JIT and the code dominate it. On 16 and 32 threads (one run each) the 256 MB ceiling peaked at 494 and 433 MB, the 2 GB one at 571 and 535 MB; the heap left after a collection stayed under 100 MB.
- **The serial collector does not help.** `-XX:+UseSerialGC` gave the same peaks as G1 (10,000 tests: 439 MB with no ceiling, 345 MB at 256 MB) and slower runs on 2,000 tests.
- **Class-data sharing speeds the start but costs memory.** With an archive Java makes itself on the first run (`-XX:SharedArchiveFile=… -XX:+AutoCreateSharedArchive`, 9.8 MB, accepted with the project's folders on the classpath), the dry run of 2,000 tests took 1,333 ms instead of 1,539 and peaked at 187 MB instead of 168 (median of 4). Memory comes before start time, so it is not part of this spec.
- **The JIT costs more CPU than the run.** With only the C1 compiler (`-XX:TieredStopAtLevel=1`), the 10,000-test run used 16 s of Java CPU instead of 41 s in about the same time, and peaked about 60 MB lower. This is not a memory limit and changes how fast long suites run, so it is a follow-up ([#179](https://github.com/hpagniez/grace/issues/179)), not part of this spec.

## Who and what success looks like

The person running suites all day, with Grace open beside everything else.

Success:
- A run of the 10,000-test project peaks well below today's 530 MB, and passes.
- No run, however greedy, takes more than 2 GB of heap.
- When a suite needs more than that, the person is told in plain words that Java ran out of memory, not left with a run that failed for no visible reason.

## Goals, non-goals, anti-goals

**Goals:** the behaviour in the Requirements section below.

**Non-goals:**
- A setting to raise or lower the limit (see Approaches).
- Tuning the JIT (`-XX:TieredStopAtLevel=1`, fewer compiler threads), and class-data sharing. Follow-up #179.
- Choosing another garbage collector.
- Limiting memory outside the heap (`-XX:MaxMetaspaceSize`, `-XX:ReservedCodeCacheSize`, direct memory).
- Reading JVM options from the project or the environment. Grace already strips `JAVA_TOOL_OPTIONS`, `JDK_JAVA_OPTIONS` and `_JAVA_OPTIONS`.

**Anti-goals** (failure even if everything else works):
- A suite that passes today fails because of the limit, on a machine with 8 GB or more.
- A run slows down noticeably, or Java's CPU goes up beyond the bench's noise.
- A run that ran out of memory looks like an ordinary test failure.

## Constraints

- Flags go on the Java command line, after the `@argfile` and before `-cp` (`buildArgs` in `src/main/karate/command.ts`). The launcher expands `@file` only before the main class, so the file stays first.
- The same flags go on the dry run and the run: both are a JVM Grace starts, and the bench's Java peak covers both.
- `npm run bench` measures it ([docs/specs/2026-10-31-light-and-fast.md](2026-10-31-light-and-fast.md)). Lowering a limit in `bench/limits.json` is an ordinary commit; raising one needs a decision record.
- Java exits with code 3 on a fatal error. Grace already shows a run with exit code 3 as failed, its tests still running as interrupted, and Java's output in the console.

### What exists today

- `buildArgs` passes `-Dstdout.encoding=UTF-8 -Dstderr.encoding=UTF-8 -cp …`; no heap flag.
- `spawnKarate` (`src/main/karate/process.ts`) sends Java's stdout and stderr to the same line callback.
- A dry run that exits non-zero sets the project's error to `dryRunError(lines)`: the lines that mention `Exception` or `feature`, or the last 20.
- A run emits `notice` events for problems the person must see; the window shows the latest.
- The thread count spec noted that 32 threads could pass the default heap and said "F12 adds no heap setting".

## Approaches considered

- **A. A tight fixed ceiling (`-Xmx256m`), with a setting to raise it.** The biggest cut at 2,000 tests on many threads (about 150 MB). But 256 MB is only twice the largest heap measured, and real suites carry bigger payloads than the bench's, so some would run out of memory and need the setting: a new Settings section, a stored value, and a choice the person has to understand.
- **B. A small starting heap and a generous fixed ceiling (`-Xms64m -Xmx2g`), no setting (chosen).** The heap starts small and grows only when the garbage collector needs room, so a run takes about what its suite needs. It cut as much as A or more at 10,000 tests and on the dry run, and 40 to 50 MB less than A at 2,000 tests and above 8 threads. 2 GB is the ceiling an 8 GB machine already gives Java today, so a suite that fits there still fits; on the reference machine the ceiling falls from 8 GB to 2 GB. No setting, because a suite that needs more than 2 GB of heap already fails on an 8 GB machine.
- **C. A share of the RAM (`-XX:MaxRAMPercentage`).** Rejected: a suite's needs do not grow with the machine's RAM, and the smallest machines, where memory matters most, would get the smallest ceiling.
- **D. The serial collector, with A or B.** Measured, no gain (see above).

## Requirements

### R1. The flags

1. Every Java that Grace starts for Karate, dry run and run, gets `-Xms64m -Xmx2g -XX:+ExitOnOutOfMemoryError`, after the `@argfile` when there is one and before `-Dstdout.encoding`.
2. The values are constants in `command.ts`. Nothing reads them from the project, a setting or the environment.

### R2. Out of memory

1. `-XX:+ExitOnOutOfMemoryError` makes Java exit at its first `OutOfMemoryError`, so a run never keeps going with some tests failing for lack of memory and the rest slowed by garbage collection.
2. Java then prints a line that starts with `Terminating due to java.lang.OutOfMemoryError` and exits with code 3. When a run's output has that line, Grace emits a notice once the run has ended: `Java ran out of memory: Grace lets it use 2 GB. Run fewer tests or fewer threads at once.` The run is shown as failed, as for any exit code 3, and Java's line stays in the console. Only the start of a line counts: a test's own output, such as a logged response body, may name the error too.
3. When a dry run's output has that line, the project's error is that same sentence instead of `dryRunError`'s pick of lines. A dry run does not run `karate-config.js` or any step, so only a project too big to list could get there.

### R3. The bench

1. `npm run bench -- --a <build without the change>` on the reference machine: the run measures (`run.memory.*`, `run.cpu.*`, `run.finishedLag`, at 2,000 tests and, in release mode, 10,000) are not worse beyond their noise, and the 10,000-test run passes.
2. A `run.memory.java*` limit the bench shows lower is lowered in `bench/limits.json`.

## Cost

- Memory: lower during a run and while Grace lists the tests (see the table above; the bench's figures are in `docs/checks/2026-11-01-java-memory.md`). Idle: nothing changes; no Java runs while idle.
- CPU: no change beyond the bench's noise (`CPU_RESULT`). More frequent young collections on a small heap are offset by touching less memory.
- Start time: no change; the dry run that builds the tree uses the same flags (`DRY_RESULT`).
- Size: none. Three arguments on a command line; no file, no dependency.
- Idle stays at zero: the change adds no timer, poll or process.

## Design decisions already made

The brief's (#149):
- The limit is sized from the bench's Java peaks, and the 10,000-test project must still pass.
- Whether people can raise the limit is decided here: they cannot (approach B).
- The serial collector and class-data sharing are measured, not assumed.

Mine, open to correction:
- Approach B over A: no setting, at the cost of a smaller cut on many threads.
- 2 GB as the ceiling: today's ceiling on an 8 GB machine.
- 64 MB as the starting heap: about the largest live heap measured on 10,000 tests on one thread; G1 grows it from there.
- Exit at the first `OutOfMemoryError` rather than let Karate carry on.
- The JIT and class-data sharing findings go to a follow-up issue (#179).

## Left to the builder

- How the run notices the `OutOfMemoryError` line (a flag set in the run's line callback, for example).
- Test fixtures: a feature that asks Java for one array of about 2 GB (`Java.type('java.lang.reflect.Array').newInstance(Java.type('java.lang.Byte').TYPE, 2147483000)`) fails at once, without using the memory.

## Follow-ups

- [#179](https://github.com/hpagniez/grace/issues/179): the JIT. Measure `-XX:TieredStopAtLevel=1` and fewer compiler threads with the bench, for Java's CPU and the peak outside the heap; class-data sharing with start time.

## Acceptance criteria

```gherkin
Feature: Java memory

  Scenario: Every Java Grace starts gets the memory flags
    Given a project is open
    When Grace starts Java for the dry run or for a run
    Then the command line has "-Xms64m -Xmx2g -XX:+ExitOnOutOfMemoryError" after the argument file and before "-cp"

  Scenario: A run that runs out of memory says so
    Given a feature asks Java for more memory than 2 GB
    When I run it
    Then Java stops at the first OutOfMemoryError
    And the run is shown as failed
    And the console shows Java's OutOfMemoryError line
    And a notice says "Java ran out of memory: Grace lets it use 2 GB. Run fewer tests or fewer threads at once."

  Scenario: A dry run that runs out of memory says so
    Given Java ran out of memory while Grace listed a project's tests
    When the dry run ends
    Then the project's error is "Java ran out of memory: Grace lets it use 2 GB. Run fewer tests or fewer threads at once."

  Scenario: A test's output that names the error is not Java running out of memory
    Given a test logs a response body with "java.lang.OutOfMemoryError" in it
    When the run ends
    Then no notice says Java ran out of memory

  Scenario: The bench shows a run takes less memory
    Given a packaged build with the change and one without it
    When I run "npm run bench -- --release --a <build without the change>" on the reference machine
    Then "run.memory.java@10k" is lower than without the change
    And no "run.*" measure is worse beyond its noise
    And the 10,000-test run passes
```
