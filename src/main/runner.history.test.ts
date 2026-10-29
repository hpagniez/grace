import {
  closeSync,
  existsSync,
  mkdirSync,
  openSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { holdFolder } from "../../test/hold";
import {
  cleanUp,
  T,
  setup,
  fixture,
  byName,
  until,
  projectDir,
  folders,
  recordFile,
  record,
  seed,
} from "../../test/runner";
import { NO_FILTER } from "../shared/tags";
import { fakeSecretBox } from "./secrets";

afterEach(cleanUp);

describe("Runner history", () => {
  const SECRET = "s3cret-token-123";

  it("a run writes run.json in its own folder", T, async () => {
    const { runner, events, last, dataDir } = setup();
    const root = fixture("clean");
    await runner.open(root);
    await runner.run("d:");
    const [id, ...more] = folders(dataDir, root);
    expect(more).toEqual([]);
    expect(last("runStarted").runId).toBe(id);
    const r = record(dataDir, root, id);
    expect(r).toMatchObject({
      version: 1,
      startedAt: last("runStarted").startedAt,
      env: null,
      threads: 1,
      filter: NO_FILTER,
      selection: { kind: "folder", file: "", line: null, key: "d:" },
      failedOf: null,
      outcome: "finished",
      exitCode: 1,
    });
    expect(r.durationMillis).toBeGreaterThan(0);
    expect(r.tests).toHaveLength(9);
    expect(r.tests.filter((t) => t.status === "failed")).toMatchObject([
      { kind: "scenario", file: "tests/basic.feature", name: "failing match" },
    ]);
    expect(r.tests.filter((t) => t.status === "passed")).toHaveLength(8);
    expect(existsSync(path.join(projectDir(dataDir, root), "run"))).toBe(false);
    // listed once the run's end events were sent, with its report
    const types = events.map((e) => e.type);
    expect(types.lastIndexOf("history")).toBeGreaterThan(types.indexOf("runFinished"));
    expect(last("history").runs).toMatchObject([
      { id, passed: 8, failed: 1, total: 9, outcome: "finished", reportAvailable: true },
    ]);
    expect(runner.reportPath()).toBe(
      path.join(path.dirname(recordFile(dataDir, root, id)), "karate-summary.html"),
    );
  });

  it("run.json holds no secret", T, async () => {
    const { runner, last, dataDir } = setup();
    const root = fixture("env");
    await runner.open(root);
    await runner.setEnv("qa");
    runner.saveProperties({
      shared: [],
      byEnv: { qa: [{ name: "api.token", value: SECRET, secret: true }] },
    });
    await runner.run(byName(last("project").tree!, "always").key);
    expect(last("runFinished").outcome).toBe("finished");
    const [id] = folders(dataDir, root);
    const text = readFileSync(recordFile(dataDir, root, id), "utf8");
    expect(text).not.toContain(SECRET);
    expect(JSON.parse(text)).toMatchObject({ env: "qa", outcome: "finished" });
  });

  it("two runs keep two folders, newest first", T, async () => {
    const { runner, last, dataDir } = setup();
    const root = fixture("env");
    await runner.open(root);
    const key = byName(last("project").tree!, "always").key;
    await runner.run(key);
    await runner.run(key);
    const ids = folders(dataDir, root);
    expect(ids).toHaveLength(2);
    const runs = last("history").runs;
    expect(runs.map((r) => r.id)).toEqual([...ids].reverse());
    expect(runs[0].startedAt).toBeGreaterThan(runs[1].startedAt);
  });

  it("retention keeps 20", T, async () => {
    const { runner, last, dataDir } = setup();
    const root = fixture("env");
    const seeded = seed(dataDir, root, 20);
    await runner.open(root);
    expect(last("history").runs).toHaveLength(20);
    await runner.run(byName(last("project").tree!, "always").key);
    const ids = folders(dataDir, root);
    expect(ids).toHaveLength(20);
    expect(ids).not.toContain(seeded[0]);
    expect(ids.slice(0, 19)).toEqual(seeded.slice(1));
    expect(last("history").runs).toHaveLength(20);
    expect(last("history").runs[0].id).toBe(ids[19]);
  });

  it("a refused run creates no folder", T, async () => {
    const first = setup();
    const root = fixture("env");
    await first.runner.open(root);
    first.runner.saveProperties({
      shared: [{ name: "api.token", value: SECRET, secret: true }],
      byEnv: {},
    });
    await first.runner.dispose();
    const settings = path.join(first.dataDir, "settings.json");
    const cipher = fakeSecretBox().encrypt(SECRET);
    writeFileSync(settings, readFileSync(settings, "utf8").replace(cipher, "Zm9v"));

    const { runner, all, last, dataDir } = setup(first.dataDir);
    await runner.open(root);
    await runner.run("d:");
    expect(last("notice").message).toContain("can't be read");
    expect(all("runStarted")).toHaveLength(0);
    expect(folders(dataDir, root)).toEqual([]);
    expect(last("history").runs).toEqual([]);
    expect(all("history")).toHaveLength(1); // the one of open
  });

  it(
    "a cancelled run is recorded as cancelled with notRun for tests that never started",
    T,
    async () => {
      const { runner, all, last, dataDir } = setup();
      const root = fixture("clean");
      await runner.open(root);
      const run = runner.run("f:tests/slow.feature");
      await until(() =>
        all("scenarios").some((e) => e.updates.some((u) => u.status === "running")),
      );
      await runner.cancel();
      await run;
      const [id] = folders(dataDir, root);
      const r = record(dataDir, root, id);
      expect(r).toMatchObject({ outcome: "cancelled", exitCode: null });
      expect(r.tests.map((t) => [t.name, t.status])).toEqual([
        ["slow loop", "interrupted"],
        ["smoke and slow", "notRun"],
      ]);
      expect(last("history").runs).toMatchObject([
        { id, outcome: "cancelled", passed: 0, failed: 0, total: 2, reportAvailable: false },
      ]);
    },
  );

  it("dispose records the killed run as cancelled", T, async () => {
    const { runner, all, dataDir } = setup();
    const root = fixture("clean");
    await runner.open(root);
    const run = runner.run("f:tests/slow.feature");
    await until(() => all("runStarted").length > 0);
    await runner.dispose();
    // written before dispose returned, not when the run's promise settles
    const [id, ...more] = folders(dataDir, root);
    expect(more).toEqual([]);
    expect(record(dataDir, root, id)).toMatchObject({ outcome: "cancelled" });
    await run;
  });

  it("replay includes history", T, async () => {
    const { runner, last } = setup();
    const root = fixture("env");
    await runner.open(root);
    expect(last("history").runs).toEqual([]);
    await runner.run(byName(last("project").tree!, "always").key);
    expect(last("history").runs).toHaveLength(1);
    expect(runner.replay()).toEqual([
      runner.current(),
      { type: "filter", filter: NO_FILTER },
      { type: "threads", count: 1 },
      last("environment"),
      last("history"),
      last("flaky"),
      { type: "opening", root: null },
      { type: "busy", busy: false },
    ]);
  });

  it("every rediscovery sends history, also a cancelled one", T, async () => {
    const { runner, events, last, dataDir } = setup();
    const root = fixture("env");
    const [id] = seed(dataDir, root, 1);
    await runner.open(root);
    expect(last("history").runs.map((r) => r.id)).toEqual([id]);
    await runner.refresh();
    await runner.setEnv("qa");
    expect(events.filter((e) => e.type === "history")).toHaveLength(3);
    const from = events.length;
    const switching = runner.setEnv("prod");
    await runner.cancel();
    await switching;
    expect(events.slice(from).filter((e) => e.type === "history")).toHaveLength(1);
    expect(last("history").runs.map((r) => r.id)).toEqual([id]);
  });

  it("a run that fails to start is recorded as failed", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const root = fixture("clean");
    await runner.open(root);
    const pidFile = path.join(dataDir, "running.pid");
    mkdirSync(pidFile); // writing a file there fails
    try {
      const before = Date.now();
      await runner.run("f:tests/slow.feature");
      expect(all("runStarted")).toHaveLength(0);
      const [id] = folders(dataDir, root);
      const r = record(dataDir, root, id);
      expect(r).toMatchObject({ outcome: "failed", exitCode: null });
      expect(r.startedAt).toBeGreaterThanOrEqual(before);
      expect(r.startedAt).toBeLessThanOrEqual(Date.now());
      expect(r.tests.map((t) => t.status)).toEqual(["notRun", "notRun"]);
      expect(last("history").runs).toMatchObject([{ id, outcome: "failed", total: 2 }]);
    } finally {
      rmSync(pidFile, { recursive: true, force: true });
    }
  });

  it("an error before the run ends leaves no folder", T, async () => {
    const { runner, last, dataDir } = setup();
    const root = fixture("env");
    await runner.open(root);
    runner.saveProperties({
      shared: [{ name: "timeout", value: "5000", secret: false }],
      byEnv: {},
    });
    const blocker = path.join(projectDir(dataDir, root), "run-args.txt");
    mkdirSync(blocker); // writing the argument file there fails
    try {
      await expect(runner.run(byName(last("project").tree!, "always").key)).rejects.toThrow();
      expect(folders(dataDir, root)).toEqual([]);
    } finally {
      rmSync(blocker, { recursive: true, force: true });
    }
  });

  it("a record that cannot be written is a notice, not an error", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const root = fixture("env");
    await runner.open(root);
    const run = runner.run(byName(last("project").tree!, "slow").key);
    await until(() => all("runStarted").length > 0);
    // a folder where the file goes: writing it fails
    mkdirSync(recordFile(dataDir, root, last("runStarted").runId));
    await run;
    expect(last("runFinished").outcome).toBe("finished");
    expect(last("notice").message).toContain("EISDIR");
    expect(runner.reportPath()).not.toBeNull();
  });

  it("the history sample project fails where the spec says", T, async () => {
    const { runner, last, dataDir } = setup();
    const root = fixture("history");
    await runner.open(root);
    await runner.run("d:");
    const [id] = folders(dataDir, root);
    const failed = record(dataDir, root, id)
      .tests.filter((t) => t.status === "failed")
      .map((t) => t.name);
    expect(failed).toEqual(["fixable", "row 2", "dynamic 2"]);
    expect(last("history").runs).toMatchObject([{ passed: 3, failed: 3, total: 6 }]);
  });

  it("clearHistory deletes the runs and emits an empty history", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const root = fixture("env");
    seed(dataDir, root, 2);
    await runner.open(root);
    await runner.run(byName(last("project").tree!, "always").key);
    expect(runner.reportPath()).not.toBeNull();
    expect(last("history").runs).toHaveLength(3);

    // refused while busy
    const run = runner.run(byName(last("project").tree!, "slow").key);
    await until(() => all("runStarted").length === 2);
    runner.clearHistory();
    expect(last("notice").message).toBe("A run is already in progress");
    expect(folders(dataDir, root)).toHaveLength(4);
    await runner.cancel();
    await run;

    // a process running inside a run folder holds it
    const [held] = folders(dataDir, root);
    const release = await holdFolder(path.join(projectDir(dataDir, root), "runs", held));
    try {
      runner.clearHistory();
      expect(last("notice").message).toBe("Some runs could not be deleted");
      expect(folders(dataDir, root)).toEqual([held]);
    } finally {
      await release();
    }

    const notices = all("notice").length;
    runner.clearHistory();
    expect(all("notice")).toHaveLength(notices);
    expect(folders(dataDir, root)).toEqual([]);
    expect(last("history").runs).toEqual([]);
    expect(runner.reportPath()).toBeNull();
  });

  it("clearHistory keeps the toolbar report of a run it could not delete", T, async () => {
    const { runner, last, dataDir } = setup();
    const root = fixture("env");
    await runner.open(root);
    await runner.run(byName(last("project").tree!, "always").key);
    const report = runner.reportPath()!;
    expect(report).not.toBeNull();
    const fd = openSync(report, 0x10000000); // libuv's UV_FS_O_EXLOCK: no sharing, so no delete
    try {
      runner.clearHistory();
      expect(last("notice").message).toBe("Some runs could not be deleted");
    } finally {
      closeSync(fd);
    }
    expect(last("history").runs).toMatchObject([{ reportAvailable: true }]);
    expect(runner.reportPath()).toBe(report);
    runner.clearHistory();
    expect(folders(dataDir, root)).toEqual([]);
    expect(runner.reportPath()).toBeNull();
  });

  it("runReportPath is null for a cancelled run and for an unknown id", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const root = fixture("env");
    await runner.open(root);
    await runner.run(byName(last("project").tree!, "always").key);
    const done = last("runStarted").runId;
    const run = runner.run(byName(last("project").tree!, "slow").key);
    await until(() => all("runStarted").length === 2);
    await runner.cancel();
    await run;
    const cancelled = last("runStarted").runId;
    expect(runner.runReportPath(done)).toBe(
      path.join(projectDir(dataDir, root), "runs", done, "karate-summary.html"),
    );
    expect(runner.runReportPath(cancelled)).toBeNull();
    expect(runner.runReportPath("20200101-000000-000")).toBeNull();
    expect(runner.runReportPath(`${done}\\..\\${done}`)).toBeNull();
  });
});
