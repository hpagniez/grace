// The step debugger in the Runner, with the bundled Java, Karate and listener (docs/specs/2026-11-02-step-debugger.md).
import { afterEach, describe, expect, it } from "vitest";
import {
  byName,
  cleanUp,
  fixture,
  javaCommandLines,
  projectDir,
  setup,
  T,
  until,
} from "../../test/runner";
import { readRunRecord } from "./history";

afterEach(cleanUp);

/** The project open with `breakpoints` set; `nth(n)` waits for the n-th pause. */
async function open(name: "debug" | "maven", breakpoints: [string, number][] = []) {
  const s = setup();
  const root = fixture(name);
  await s.runner.open(root);
  for (const [file, line] of breakpoints) s.runner.toggleBreakpoint(file, line);
  const key = (test: string) => byName(s.last("project").tree!, test).key;
  const pauses = () => s.all("debugPaused").map((e) => e.paused);
  const nth = async (n: number) => {
    await until(() => pauses().length >= n);
    return pauses()[n - 1];
  };
  const vars = (n: number) =>
    Object.fromEntries(pauses()[n - 1].vars.map((v) => [v.name, `${v.kind} ${v.value}`]));
  return { ...s, root, key, pauses, nth, vars };
}

describe("debug runs", () => {
  it("a breakpoint pauses before its step, with the variables", T, async () => {
    const d = await open("debug", [["debug.feature", 8]]);
    const done = d.runner.debug(d.key("variables"));
    expect(await d.nth(1)).toMatchObject({
      file: "debug.feature",
      line: 8,
      depth: 0,
      prefix: "*",
      keyword: "def",
      text: "user = { id: 7 }",
      key: d.key("variables"),
    });
    expect(d.vars(1)).toEqual({ base: "string from background", name: "string grace" });
    expect(d.last("runStarted").debug).toBe(true);
    d.runner.debugCommand("continue");
    await done;
    expect(d.all("debugResumed")).toHaveLength(1);
    expect(d.last("runFinished")).toMatchObject({
      outcome: "finished",
      summary: { scenariosPassed: 1, scenariosFailed: 0 },
    });
  });

  it("step over stays at the same depth; step into enters the called feature", T, async () => {
    const d = await open("debug", [["debug.feature", 9]]);
    const done = d.runner.debug(d.key("variables"));
    await d.nth(1);
    d.runner.debugCommand("over");
    expect(await d.nth(2)).toMatchObject({ file: "debug.feature", line: 10, depth: 0 });
    const called = d.pauses()[1].vars.find((v) => v.name === "called")!;
    expect(called.kind).toBe("json");
    expect(JSON.parse(called.value)).toMatchObject({ arg: "in", inner: "in!" });
    d.runner.debugCommand("continue");
    await done;

    const again = d.runner.debug(d.key("variables"));
    await d.nth(3);
    d.runner.debugCommand("into");
    expect(await d.nth(4)).toMatchObject({
      file: "called.feature",
      line: 5,
      depth: 1,
      key: d.key("variables"),
    });
    d.runner.debugCommand("continue");
    await again;
  });

  it("a pause asked as the run starts stops before the first step", T, async () => {
    const d = await open("debug");
    const done = d.runner.debug("f:debug.feature");
    await until(() => d.all("runStarted").length === 1);
    d.runner.debugCommand("pause");
    expect(await d.nth(1)).toMatchObject({ file: "debug.feature", line: 4 });
    d.runner.debugCommand("continue");
    await done;
    expect(d.pauses()).toHaveLength(1);
  });

  it("runs on one thread and records a debug run", T, async () => {
    const d = await open("debug");
    d.runner.setThreads(4);
    await d.runner.debug(d.key("second"));
    const record = readRunRecord(projectDir(d.dataDir, d.root), d.last("runStarted").runId)!;
    expect(record).toMatchObject({ threads: 1, debug: true, outcome: "finished" });
    await d.runner.run(d.key("second"));
    const normal = readRunRecord(projectDir(d.dataDir, d.root), d.last("runStarted").runId)!;
    expect(normal.threads).toBe(4);
    expect(normal.debug).toBeUndefined();
  });

  it("takes a breakpoint added while the run goes on", T, async () => {
    const d = await open("debug", [["debug.feature", 8]]);
    const done = d.runner.debug(d.key("variables"));
    await d.nth(1);
    d.runner.toggleBreakpoint("debug.feature", 10);
    expect(d.last("breakpoints").breakpoints).toEqual([
      { file: "debug.feature", line: 8 },
      { file: "debug.feature", line: 10 },
    ]);
    d.runner.debugCommand("continue");
    expect(await d.nth(2)).toMatchObject({ line: 10 });
    d.runner.debugCommand("continue");
    await done;
  });

  it("cancel while paused ends the run cancelled and leaves no Java", T, async () => {
    const d = await open("debug", [["debug.feature", 8]]);
    const done = d.runner.debug(d.key("variables"));
    await d.nth(1);
    await d.runner.cancel();
    await done;
    expect(d.last("runFinished").outcome).toBe("cancelled");
    expect(javaCommandLines(d.root)).toEqual([]);
  });

  it("a socket that closes while paused lets the run go on", T, async () => {
    const d = await open("debug", [["debug.feature", 8]]);
    const done = d.runner.debug(d.key("variables"));
    await d.nth(1);
    (d.runner as unknown as { session: { close(): void } }).session.close();
    await done;
    expect(d.all("debugResumed")).toHaveLength(1);
    expect(d.last("runFinished")).toMatchObject({ outcome: "finished" });
  });

  it("masks secrets in variables and cuts a long value", T, async () => {
    const d = await open("debug", [["big.feature", 6]]);
    d.runner.saveProperties({
      shared: [{ name: "token", value: "s3cr3t", secret: true }],
      byEnv: {},
    });
    const done = d.runner.debug(d.key("big value"));
    const big = (await d.nth(1)).vars;
    expect(big.find((v) => v.name === "token")).toMatchObject({ kind: "string", value: "****" });
    const value = big.find((v) => v.name === "big")!;
    expect(value.value).toHaveLength(65_536);
    expect(value.cut).toBe(102_400 - 65_536);
    d.runner.debugCommand("continue");
    await done;
  });

  it("gives a reloaded window the breakpoints, and refuses a line of no feature", T, async () => {
    const d = await open("debug", [["debug.feature", 8]]);
    expect(d.runner.replay()).toContainEqual({
      type: "breakpoints",
      breakpoints: [{ file: "debug.feature", line: 8 }],
    });
    d.runner.toggleBreakpoint("nope.feature", 3);
    expect(d.last("notice").message).toBe("Invalid request");
    d.runner.toggleBreakpoint("debug.feature", 8);
    expect(d.last("breakpoints").breakpoints).toEqual([]);
  });

  it("matches a breakpoint in a Maven project by the tree's file", T, async () => {
    const d = await open("maven", [["examples/users.feature", 6]]);
    const done = d.runner.debug(d.key("config, classpath data and classpath call"));
    expect(await d.nth(1)).toMatchObject({ file: "examples/users.feature", line: 6 });
    d.runner.debugCommand("continue");
    await done;
  });
});

describe("steps from the feature file", () => {
  it("lists a test's steps before any run", T, async () => {
    const d = await open("debug");
    expect(d.runner.steps(d.key("variables"))!.map((s) => s.line)).toEqual([4, 7, 8, 9, 10]);
    expect(d.runner.steps("f:debug.feature")).toBeNull();
  });
});
