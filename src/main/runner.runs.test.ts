import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import {
  cleanUp,
  T,
  setup,
  fixture,
  leaves,
  byName,
  until,
  hashTree,
  ourJava,
} from "../../test/runner";

afterEach(cleanUp);

describe("Runner runs", () => {
  it("runs all with live progress", T, async () => {
    const { runner, events, last } = setup();
    await runner.open(fixture("clean"));
    expect(runner.isBusy()).toBe(false);
    const run = runner.run("d:");
    expect(runner.isBusy()).toBe(true);
    await run;
    expect(runner.isBusy()).toBe(false);
    expect(last("runStarted")).toMatchObject({ selection: "d:", total: 9 });
    const types = events.map((e) => e.type);
    const firstUpdate = types.indexOf("scenarios");
    expect(firstUpdate).toBeGreaterThan(-1);
    expect(firstUpdate).toBeLessThan(types.indexOf("runFinished"));
    const done = last("runFinished");
    expect(done).toMatchObject({ outcome: "finished", exitCode: 1, reportAvailable: true });
    expect(done.summary).toMatchObject({ scenariosPassed: 8, scenariosFailed: 1 });
    expect(runner.reportPath()).not.toBeNull();
    expect(existsSync(runner.reportPath()!)).toBe(true);
    await runner.open(fixture("maven"));
    expect(runner.reportPath()).toBeNull();
  });

  it("runs one outline row", T, async () => {
    const { runner, last } = setup();
    await runner.open(fixture("clean"));
    await runner.run("s:tests/outline.feature#1.2");
    expect(last("runStarted")).toMatchObject({
      total: 1,
      keys: ["s:tests/outline.feature#1.2"],
    });
    expect(last("runFinished").summary).toMatchObject({ scenarioCount: 1, scenariosPassed: 1 });
  });

  it("runs the row clicked when its table has comment and blank lines", T, async () => {
    const { runner, all, last } = setup();
    await runner.open(fixture("edge"));
    const key = "s:tests/rows.feature#1.2";
    await runner.run(key);
    expect(last("runStarted")).toMatchObject({ keys: [key], total: 1 });
    expect(last("runFinished").summary).toMatchObject({ scenarioCount: 1, scenariosPassed: 1 });
    const finals = all("scenarios")
      .flatMap((e) => e.updates)
      .filter((u) => u.status !== "running");
    expect(finals).toMatchObject([{ key, status: "passed" }]);
  });

  it("refuses a row of a dynamic Examples table", T, async () => {
    const { runner, all, last } = setup();
    await runner.open(fixture("edge"));
    const rows = byName(last("project").tree!, "dynamic <a>").children;
    expect(rows.map((r) => r.line)).toEqual([null, null]);
    await runner.run(rows[0].key);
    expect(last("notice").message).toBe("Run the outline to run its rows");
    expect(all("runStarted")).toHaveLength(0);
  });

  it("re-discovers before running", T, async () => {
    const { runner, all, last } = setup();
    const root = fixture("clean");
    await runner.open(root);
    const oldKey = byName(last("project").tree!, "failing match").key;
    const file = path.join(root, "tests", "basic.feature");
    const text = readFileSync(file, "utf8");
    const edited = text.replace(
      /@smoke\r?\n/,
      (m) => `Scenario: inserted first\n  * def y = 2\n\n${m}`,
    );
    expect(edited).not.toBe(text);
    writeFileSync(file, edited);
    await runner.run(oldKey);
    expect(all("project")).toHaveLength(2);
    const tree = last("project").tree!;
    expect(leaves(tree)).toBe(10);
    const newKey = byName(tree, "failing match").key;
    expect(newKey).not.toBe(oldKey);
    expect(last("runStarted")).toMatchObject({ selection: newKey, keys: [newKey], total: 1 });
    expect(last("runFinished").summary).toMatchObject({ scenarioCount: 1, scenariosFailed: 1 });
    const updated = new Set(all("scenarios").flatMap((e) => e.updates.map((u) => u.key)));
    expect([...updated]).toEqual([newKey]);
  });

  it("refuses a test that changed", T, async () => {
    const { runner, all, last } = setup();
    const root = fixture("clean");
    await runner.open(root);
    const oldKey = byName(last("project").tree!, "failing match").key;
    const file = path.join(root, "tests", "basic.feature");
    writeFileSync(file, readFileSync(file, "utf8").replace("failing match", "failing compare"));
    await runner.run(oldKey);
    expect(last("notice").message).toBe("This test changed; the tree was refreshed");
    expect(all("runStarted")).toHaveLength(0);
  });

  it("rejects a run while another is active", T, async () => {
    const { runner, all, last } = setup();
    await runner.open(fixture("clean"));
    const first = runner.run("f:tests/slow.feature");
    await until(() => all("runStarted").length > 0);
    await runner.run("d:");
    await runner.refresh();
    expect(all("notice").map((n) => n.message)).toEqual([
      "A run is already in progress",
      "A run is already in progress",
    ]);
    await runner.cancel();
    await first;
    expect(all("runStarted")).toHaveLength(1);
    expect(all("project")).toHaveLength(2); // open + the first run's re-discovery
    expect(last("runFinished").outcome).toBe("cancelled");
  });

  it("cancel during the pre-run discovery runs nothing", T, async () => {
    const { runner, all } = setup();
    await runner.open(fixture("clean"));
    const run = runner.run("d:"); // its dry run is spawned before run() first awaits
    await runner.cancel();
    await run;
    expect(all("runStarted")).toHaveLength(0);
    expect(all("runFinished")).toHaveLength(0);
    expect(all("notice").map((n) => n.message)).toEqual(["Run cancelled"]);
    expect(ourJava()).toBe(false);
  });

  it("cancel interrupts and leaves no java", T, async () => {
    const { runner, all, last, dataDir } = setup();
    await runner.open(fixture("clean"));
    const pidFile = path.join(dataDir, "running.pid");
    const run = runner.run("f:tests/slow.feature");
    await until(() => all("scenarios").some((e) => e.updates.some((u) => u.status === "running")));
    expect(existsSync(pidFile)).toBe(true);
    await runner.cancel();
    await run;
    expect(last("runFinished")).toMatchObject({ outcome: "cancelled", reportAvailable: false });
    expect(all("scenarios").some((e) => e.updates.some((u) => u.status === "interrupted"))).toBe(
      true,
    );
    expect(runner.reportPath()).toBeNull();
    expect(ourJava()).toBe(false);
    expect(existsSync(pidFile)).toBe(false);
  });

  it("does not modify the project", T, async () => {
    const { runner, last } = setup();
    const root = fixture("clean");
    const before = hashTree(root);
    await runner.open(root);
    await runner.run("d:");
    expect(last("runFinished").outcome).toBe("finished");
    const after = hashTree(root);
    const outside = (m: Map<string, string>) =>
      new Map([...m].filter(([p]) => !p.split("/").includes("karate-temp")));
    expect(outside(after)).toEqual(outside(before));
  });

  it("a pid file that cannot be written fails the run and leaves no java", T, async () => {
    const { runner, all, last, dataDir } = setup();
    await runner.open(fixture("clean"));
    const pidFile = path.join(dataDir, "running.pid");
    mkdirSync(pidFile); // writing a file there fails
    try {
      await runner.run("f:tests/slow.feature");
      expect(last("runFinished")).toMatchObject({ outcome: "failed", reportAvailable: false });
      expect(last("notice").message).toContain("EISDIR");
      expect(all("runStarted")).toHaveLength(0);
      expect(ourJava()).toBe(false);
    } finally {
      rmSync(pidFile, { recursive: true, force: true });
    }
  });

  it("dispose kills a run and removes the pid file", T, async () => {
    const { runner, all, dataDir } = setup();
    await runner.open(fixture("clean"));
    const run = runner.run("f:tests/slow.feature");
    await until(() => all("runStarted").length > 0);
    await runner.dispose();
    await run;
    expect(ourJava()).toBe(false);
    expect(existsSync(path.join(dataDir, "running.pid"))).toBe(false);
  });
});
