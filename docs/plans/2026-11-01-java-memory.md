# Java Memory Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Start every Karate JVM with a small starting heap, a 2 GB ceiling and an exit at the first `OutOfMemoryError`, and tell the person in plain words when a run ran out of memory.

**Architecture:** `buildArgs` in `src/main/karate/command.ts` adds three constant flags after the `@argfile` and before `-Dstdout.encoding`. The same file exports the out-of-memory check and its sentence. The runner sets a flag when a run's console lines hold the check's marker and emits the sentence as a notice once the run has ended; a failed dry run with the marker takes the sentence as its error.

**Tech Stack:** TypeScript (Electron main), Vitest (`*.test.ts` in Node), the bundled Temurin 21 and Karate 2.1.4.

**Spec:** [docs/specs/2026-11-01-java-memory.md](../specs/2026-11-01-java-memory.md)

## Global Constraints

- Flags, exactly: `-Xms64m -Xmx2g -XX:+ExitOnOutOfMemoryError`, in that order.
- They go after `@<argFile>` when there is one, and before `-Dstdout.encoding=UTF-8`; on dry runs and runs alike.
- The marker is a line that starts with `Terminating due to java.lang.OutOfMemoryError`: what Java prints, with `-XX:+ExitOnOutOfMemoryError`, before it exits with code 3. A test's own output (a logged response body) may name the error anywhere else in a line.
- The sentence, exactly: `Java ran out of memory: Grace lets it use 2 GB. Run fewer tests or fewer threads at once.`
- No setting, no environment variable, no project file feeds these values.
- Never run `npm test` and `npm run e2e` or `npm run bench` at the same time.
- Commits follow AGENTS.md: scopes `runner`, `docs`, `bench`. The PR says `Closes #149`.

## Review Focus

1. **A run with an `@argfile`** (properties set): the flags must sit after `@file`, since the launcher expands `@file` only before the main class. Pinned in Task 1 (`puts the argument file first…`).
2. **An `OutOfMemoryError` inside a scenario that Karate would catch**: the JVM must still exit at once, not carry on. Pinned in Task 2 (the second scenario never prints).
3. **A cancelled run whose output happens to hold the marker**: a cancel stays a cancel; the notice is only about memory, so it may still show. No extra test: the outcome logic is untouched.
4. **A response body that names `java.lang.OutOfMemoryError`** (the service under test failed, and Karate logs its body): no notice. Pinned in Task 1 (`outOfMemory` ignores a line that does not start with the JVM's text).
5. **Two runs in a row, the first out of memory**: the second must not repeat the notice. Pinned in Task 2 (the flag is per run).

---

### Task 1: The flags and the out-of-memory check

**Files:**
- Modify: `src/main/karate/command.ts` (`buildArgs`, new exports)
- Test: `src/main/karate/command.test.ts`

**Interfaces:**
- Produces: `export const OUT_OF_MEMORY: string` (the sentence) and `export function outOfMemory(lines: string[]): boolean` (true when a line starts with `Terminating due to java.lang.OutOfMemoryError`).

- [ ] **Step 1: Write the failing tests**

In `describe("buildArgs")`, put `"-Xms64m", "-Xmx2g", "-XX:+ExitOnOutOfMemoryError"` at the start of `base`; the existing tests then pin the order for dry runs, runs and the argument file (`["@A.txt", ...base, ...]`). Add:

```ts
describe("outOfMemory", () => {
  it("finds Java's out-of-memory line among the others", () => {
    expect(outOfMemory(["a", "Terminating due to java.lang.OutOfMemoryError: Java heap space"])).toBe(true);
    expect(outOfMemory(["Terminating due to java.lang.OutOfMemoryError: Metaspace"])).toBe(true);
    expect(outOfMemory(['  "error": "java.lang.OutOfMemoryError: Java heap space"'])).toBe(false);
    expect(outOfMemory(["scenario failed", "OutOfMemory in a test name"])).toBe(false);
    expect(outOfMemory([])).toBe(false);
  });
  it("says what happened and what to do", () => {
    expect(OUT_OF_MEMORY).toBe(
      "Java ran out of memory: Grace lets it use 2 GB. Run fewer tests or fewer threads at once.",
    );
  });
});
```

- [ ] **Step 2: Run them and see them fail**

Run: `npx vitest run src/main/karate/command.test.ts`
Expected: FAIL, `buildArgs` lacks the flags and `outOfMemory` is not exported.

- [ ] **Step 3: Implement**

A module constant `HEAP = ["-Xms64m", "-Xmx2g", "-XX:+ExitOnOutOfMemoryError"]` with a one-line comment pointing at the spec, spread in `buildArgs` right after the `@argFile` entry. `OUT_OF_MEMORY` and `outOfMemory` beside it.

- [ ] **Step 4: Run them and see them pass**

Run: `npx vitest run src/main/karate/command.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

`perf(runner): start java with a small heap and a 2 gb ceiling`

### Task 2: Say so when Java ran out of memory

**Files:**
- Modify: `src/main/runner.ts` (the run's line callback and its end; `dryRunError`)
- Test: `src/main/runner.runs.test.ts`

**Interfaces:**
- Consumes: `OUT_OF_MEMORY`, `outOfMemory` from Task 1.

- [ ] **Step 1: Write the failing test**

```ts
it("says so when Java runs out of memory, and the next run does not", T, async () => {
  const { runner, events, all, last } = setup();
  const root = fixture("clean");
  writeFileSync(
    path.join(root, "tests", "big.feature"),
    [
      "Feature: big",
      "",
      "Scenario: asks for more than the limit",
      "  * def bytes = Java.type('java.lang.reflect.Array').newInstance(Java.type('java.lang.Byte').TYPE, 2147483000)",
      "",
      "Scenario: after it",
      "  * print 'after it ran'",
      "",
    ].join("\n"),
  );
  await runner.open(root);
  await runner.run("f:tests/big.feature");
  expect(last("runFinished")).toMatchObject({ outcome: "failed", exitCode: 3 });
  expect(printed(events).some((l) => l.includes("java.lang.OutOfMemoryError"))).toBe(true);
  expect(printed(events).some((l) => l.includes("after it ran"))).toBe(false);
  expect(last("notice").message).toBe(OUT_OF_MEMORY);
  const notices = all("notice").length;
  await runner.run(byName(last("project").tree!, "after it").key);
  expect(last("runFinished").outcome).toBe("finished");
  expect(all("notice")).toHaveLength(notices);
});
```

Use the feature's real key from the tree (`byName(tree, "big.feature")` or the key format the other tests use) if `f:tests/big.feature` is not it.

- [ ] **Step 2: Run it and see it fail**

Run: `npx vitest run src/main/runner.runs.test.ts -t "out of memory"`
Expected: FAIL, no notice.

- [ ] **Step 3: Implement**

In the run, a `let oom = false` set by the line callback (`if (outOfMemory(lines)) oom = true`) before it emits the console lines; after `store(done, …)`, `if (oom) emit({ type: "notice", message: OUT_OF_MEMORY })`. In `dryRunError`, return `OUT_OF_MEMORY` when `outOfMemory(lines)`.

- [ ] **Step 4: Run it and see it pass**

Run: `npx vitest run src/main/runner.runs.test.ts`
Expected: PASS, all tests in the file.

- [ ] **Step 5: Commit**

`feat(runner): say so when java runs out of memory`

### Task 3: Prove it and record it

**Files:**
- Create: `docs/research/2026-11-01-java-memory/` (README with the spike's numbers, `measure.mjs`, `summary.mjs`, raw `*.jsonl`)
- Create: `docs/checks/2026-11-01-java-memory.md` and `docs/checks/2026-11-01-java-memory.bench.json`
- Modify: `docs/specs/2026-11-01-java-memory.md` (numbers in Cost), `bench/limits.json` (lower `run.memory.java*` only if the bench shows it)

- [ ] **Step 1: Package the build without the change**

`git worktree add <scratch>/base origin/develop`, `npm ci`, copy `vendor/`, `npm run package`; its exe is `<scratch>/base/out/Grace-win32-x64/Grace.exe`.

- [ ] **Step 2: Bench**

Run (alone on the machine): `npm run bench -- --release --a <scratch>/base/out/Grace-win32-x64/Grace.exe --label-a develop --label-b java-memory --out <scratch>/bench`
Expected: exit 0; the 10,000-test run passes; no `run.*` measure worse beyond its noise.

- [ ] **Step 3: Record**

Copy `bench.json` to `docs/checks/2026-11-01-java-memory.bench.json`; write the check record (machine, commits, table of `run.*` medians A vs B, what still needs a person). Lower a `run.memory.java*` limit to the new median plus its noise only when that is below the current limit.

- [ ] **Step 4: Commit**

`docs: record the java memory bench` and, if a limit moved, `perf(bench): lower the java memory limits`.
