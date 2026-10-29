import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import {
  cleanUp,
  T,
  setup,
  fixture,
  byName,
  until,
  projectDir,
  javaCommandLines,
  holdSlow,
  folders,
  record,
  seed,
  stored,
} from "../../test/runner";
import type { StoredTest } from "../shared/history";
import { NO_FILTER } from "../shared/tags";
import { loadProjectEnv, loadProjectFilter, saveProjectEnv } from "./settings";

afterEach(cleanUp);

describe("Runner rerun", () => {
  const smoke = { include: ["smoke"], exclude: [], hideNonMatching: false };
  const GONE = "This run is no longer in the history";

  it("rerunFailed runs only the failed tests that can run", T, async () => {
    const { runner, events, all, last, dataDir } = setup();
    const root = fixture("history");
    await runner.open(root);
    await runner.run("d:");
    const first = last("history").runs[0];
    writeFileSync(path.join(root, "flag.json"), '{ "fixed": true }');
    const tree = last("project").tree!;
    const keys = [byName(tree, "fixable").key, byName(tree, "row 2").key];
    const from = events.length;
    await runner.rerunFailed(first.id);
    expect(last("runStarted")).toMatchObject({ keys, total: 2 });
    const updated = all("scenarios")
      .filter((e) => events.indexOf(e) > from)
      .flatMap((e) => e.updates.map((u) => u.key));
    expect([...new Set(updated)].sort()).toEqual([...keys].sort());
    expect(events.slice(from).flatMap((e) => (e.type === "notice" ? [e.message] : []))).toEqual([
      "1 failed row of a dynamic Examples table can't be run alone. Run its outline.",
    ]);
    const [, id] = folders(dataDir, root);
    const r = record(dataDir, root, id);
    expect(r).toMatchObject({ selection: null, failedOf: first.startedAt, outcome: "finished" });
    expect(r.tests.map((t) => [t.name, t.status])).toEqual([
      ["fixable", "passed"],
      ["row 2", "failed"],
    ]);
    expect(last("history").runs[0]).toMatchObject({
      id,
      failedOf: first.startedAt,
      passed: 1,
      failed: 1,
      total: 2,
    });
  });

  it("rerunFailed passes its targets in a file", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const root = fixture("env");
    const release = holdSlow(root);
    await runner.open(root);
    const slow = byName(last("project").tree!, "slow");
    const [id] = seed(dataDir, root, 1, {
      selection: null,
      failedOf: 1,
      tests: [stored(slow, "failed")],
    });
    const file = path.join(projectDir(dataDir, root), "run-targets.txt");
    const run = runner.rerunFailed(id);
    await until(() => all("runStarted").length > 0);
    const target = `${path.join(root, "tests", "env.feature")}:${slow.line}`;
    expect(readFileSync(file, "utf8")).toBe(`"${target.replace(/\\/g, "\\\\")}"\r\n`);
    const lines = javaCommandLines(root);
    expect(lines).toHaveLength(1);
    expect(lines[0]).toContain(`@${file}`);
    expect(lines[0]).not.toContain("env.feature");
    release();
    await run;
    expect(last("runFinished").outcome).toBe("finished");
    expect(last("runStarted").keys).toEqual([slow.key]);
    expect(existsSync(file)).toBe(false);
  });

  it("rerun switches environment and filter back", T, async () => {
    const { runner, last, dataDir } = setup();
    const root = fixture("history");
    await runner.open(root);
    await runner.setEnv("qa");
    runner.setFilter(smoke);
    await runner.run("f:tests/fix.feature");
    const green = byName(last("project").tree!, "stays green").key;
    expect(last("runStarted").keys).toEqual([green]);
    const { id } = last("history").runs[0];
    await runner.setEnv(null);
    runner.setFilter(NO_FILTER);
    await runner.rerun(id);
    expect(last("environment").view.env).toBe("qa");
    expect(loadProjectEnv(dataDir, root).env).toBe("qa");
    expect(last("filter").filter).toEqual(smoke);
    expect(loadProjectFilter(dataDir, root)).toEqual(smoke);
    expect(last("runStarted")).toMatchObject({ keys: [green], total: 1, env: "qa" });
    expect(last("runFinished").summary).toMatchObject({ scenarioCount: 1, scenariosPassed: 1 });
    const [, second] = folders(dataDir, root);
    expect(record(dataDir, root, second)).toMatchObject({
      env: "qa",
      filter: smoke,
      selection: { kind: "feature", key: "f:tests/fix.feature" },
      failedOf: null,
    });
  });

  it("rerun with a vanished included tag runs nothing, never more", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const root = fixture("clean");
    await runner.open(root);
    runner.setFilter({ include: ["wip"], exclude: [], hideNonMatching: false });
    await runner.run("d:");
    expect(last("runStarted").total).toBe(2);
    const { id } = last("history").runs[0];
    runner.setFilter(NO_FILTER);
    const file = path.join(root, "tests", "basic.feature");
    writeFileSync(file, readFileSync(file, "utf8").replace(/ ?@wip/g, ""));
    await runner.rerun(id);
    expect(last("notice").message).toBe("Nothing to run");
    expect(all("runStarted")).toHaveLength(1);
    expect(folders(dataDir, root)).toEqual([id]);
  });

  it("rerun of a deleted test says the tests of this run no longer exist", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const root = fixture("clean");
    await runner.open(root);
    await runner.run(byName(last("project").tree!, "failing match").key);
    const { id } = last("history").runs[0];
    const file = path.join(root, "tests", "basic.feature");
    const text = readFileSync(file, "utf8");
    const edited = text.replace(/@wip\r?\nScenario: failing match[\s\S]*?(?=Scenario:)/, "");
    expect(edited).not.toBe(text);
    writeFileSync(file, edited);
    await runner.rerun(id);
    expect(last("notice").message).toBe("The tests of this run no longer exist");
    expect(all("runStarted")).toHaveLength(1);
    expect(folders(dataDir, root)).toEqual([id]);
  });

  it("rerun of a rerun-failed run runs all its tests and keeps failedOf", T, async () => {
    const { runner, last, dataDir } = setup();
    const root = fixture("history");
    await runner.open(root);
    const tree = last("project").tree!;
    const fixable = byName(tree, "fixable");
    const row2 = byName(tree, "row 2");
    const gone: StoredTest = {
      ...stored(fixable, "failed"),
      refId: "[9:99]",
      name: "gone",
      line: 99,
      key: "s:tests/fix.feature#9",
    };
    const [id] = seed(dataDir, root, 1, {
      selection: null,
      failedOf: 1234,
      tests: [
        stored(fixable, "passed"),
        stored(row2, "failed"),
        stored(byName(tree, "dynamic 2"), "failed"),
        gone,
      ],
    });
    await runner.rerun(id);
    expect(last("notice").message).toBe(
      "1 failed test no longer exists. 1 failed row of a dynamic Examples table can't be run alone. Run its outline.",
    );
    expect(last("runStarted")).toMatchObject({ keys: [fixable.key, row2.key], total: 2 });
    const [, second] = folders(dataDir, root);
    const r = record(dataDir, root, second);
    expect(r).toMatchObject({ selection: null, failedOf: 1234 });
    expect(r.tests.map((t) => [t.name, t.status])).toEqual([
      ["fixable", "failed"],
      ["row 2", "failed"],
    ]);

    // with no test left, nothing runs
    const [onlyGone] = seed(dataDir, root, 1, { selection: null, failedOf: 1234, tests: [gone] });
    await runner.rerunFailed(onlyGone);
    expect(last("notice").message).toBe("1 failed test no longer exists. Nothing to run.");
    expect(folders(dataDir, root)).toHaveLength(3);
  });

  it("cancel during a re-run's environment switch runs nothing", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const root = fixture("env");
    const qa = { include: ["env=qa"], exclude: [], hideNonMatching: false };
    const [id] = seed(dataDir, root, 1, { env: "qa", filter: qa });
    await runner.open(root);
    const rerun = runner.rerun(id); // its dry run is spawned before rerun() first awaits
    await runner.cancel();
    await rerun;
    expect(last("notice").message).toBe("Cancelled");
    expect(last("environment").view.env).toBeNull();
    expect(loadProjectEnv(dataDir, root).env).toBeNull();
    expect(last("filter").filter).toEqual(NO_FILTER);
    expect(loadProjectFilter(dataDir, root)).toEqual(NO_FILTER);
    expect(all("runStarted")).toHaveLength(0);
    expect(folders(dataDir, root)).toEqual([id]);
  });

  it("cancel after a re-run's environment switch, with no process, runs nothing", T, async () => {
    const { runner, all, last, dataDir, onEvent } = setup();
    const root = fixture("env");
    const [id] = seed(dataDir, root, 1, { env: "qa" });
    await runner.open(root);
    let cancel: Promise<void> | null = null;
    onEvent((e) => {
      // the switch's dry run has exited and its filter is sent: the env tags are read next, no JVM is up
      if (e.type === "filter" && all("project").length === 2) cancel ??= runner.cancel();
    });
    await runner.rerun(id);
    await cancel;
    expect(last("notice").message).toBe("Run cancelled");
    expect(all("project")).toHaveLength(2); // open, then the switch: no pre-run dry run
    expect(all("runStarted")).toHaveLength(0);
    expect(folders(dataDir, root)).toEqual([id]);
    expect(last("history").runs.map((r) => r.id)).toEqual([id]);
  });

  it("a re-run whose environment has a parse error switches and runs nothing", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const root = fixture("broken");
    const [id] = seed(dataDir, root, 1, { env: "qa" });
    await runner.open(root);
    await runner.rerun(id);
    expect(last("project").error).toContain("syntax.feature");
    expect(all("project")).toHaveLength(2); // open, then the switch: no pre-run dry run
    expect(last("environment").view.env).toBe("qa");
    expect(loadProjectEnv(dataDir, root).env).toBe("qa");
    expect(all("runStarted")).toHaveLength(0);
    expect(folders(dataDir, root)).toEqual([id]);
  });

  it("rerun checks the target environment's secrets first", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const root = fixture("env");
    saveProjectEnv(dataDir, root, {
      env: null,
      properties: { shared: [], byEnv: { qa: [{ name: "api.token", secret: "Zm9v" }] } },
    });
    const [id] = seed(dataDir, root, 1, { env: "qa" });
    await runner.open(root);
    for (const action of [() => runner.rerun(id), () => runner.rerunFailed(id)]) {
      await action();
      expect(last("notice").message).toBe(
        "Secret api.token (qa) can't be read. Replace it in Properties.",
      );
    }
    expect(all("project")).toHaveLength(1); // no discovery
    expect(last("environment").view.env).toBeNull();
    expect(loadProjectEnv(dataDir, root).env).toBeNull();
    expect(all("runStarted")).toHaveLength(0);
    expect(folders(dataDir, root)).toEqual([id]);
  });

  it("rerun refuses an unknown id", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const root = fixture("env");
    await runner.open(root);
    // a damaged record is not listed either
    const damaged = path.join(projectDir(dataDir, root), "runs", "20200101-000000-001");
    mkdirSync(damaged, { recursive: true });
    writeFileSync(path.join(damaged, "run.json"), "{");
    const ids = [
      "20200101-000000-000",
      "20200101-000000-001",
      "..\\..\\x",
      "20261025-143205-123\\..",
    ];
    for (const id of ids) {
      await runner.rerun(id);
      expect(last("notice").message).toBe(GONE);
      await runner.rerunFailed(id);
      expect(last("notice").message).toBe(GONE);
    }
    expect(all("notice")).toHaveLength(ids.length * 2);
    expect(all("project")).toHaveLength(1);
    expect(all("runStarted")).toHaveLength(0);
  });
});
