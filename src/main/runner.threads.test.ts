import { mkdirSync, readFileSync, rmSync } from "node:fs";
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
  suiteThreads,
} from "../../test/runner";
import { THREADS_ERROR } from "../shared/threads";
import { loadProjectThreads } from "./settings";

afterEach(cleanUp);

describe("Runner threads", () => {
  const eventsFile = (dataDir: string, root: string, dir: string) =>
    path.join(projectDir(dataDir, root), dir, "karate-json", "karate-events.jsonl");

  it("a run passes the count and discovery passes 1", T, async () => {
    const { runner, last, dataDir } = setup();
    const root = fixture("clean");
    await runner.open(root);
    runner.setThreads(2);
    expect(last("threads")).toEqual({ type: "threads", count: 2 });
    expect(loadProjectThreads(dataDir, root)).toBe(2);
    await runner.run("d:");
    const { runId } = last("runStarted");
    expect(suiteThreads(eventsFile(dataDir, root, path.join("runs", runId)))).toBe(2);
    expect(suiteThreads(eventsFile(dataDir, root, "dryrun"))).toBe(1);
    const stored = JSON.parse(
      readFileSync(path.join(projectDir(dataDir, root), "runs", runId, "run.json"), "utf8"),
    );
    expect(stored.threads).toBe(2);
    expect(last("history").runs[0]).toMatchObject({ id: runId, threads: 2 });
  });

  it("restores the count on open, per project", T, async () => {
    const first = setup();
    const root = fixture("clean");
    await first.runner.open(root);
    first.runner.setThreads(3);
    await first.runner.dispose();
    const second = setup(first.dataDir);
    await second.runner.open(root);
    expect(second.last("threads").count).toBe(3);
    expect(second.runner.replay()).toContainEqual({ type: "threads", count: 3 });
    await second.runner.open(fixture("maven"));
    expect(second.last("threads").count).toBe(1);
  });

  it("a cancelled open restores the count before", T, async () => {
    const { runner, last } = setup();
    await runner.open(fixture("clean"));
    runner.setThreads(3);
    const opening = runner.open(fixture("maven"));
    await runner.cancel();
    expect(await opening).toBe(false);
    expect(last("threads").count).toBe(3);
  });

  it("refuses a count that is not a whole number from 1 to 32", T, async () => {
    const { runner, last, dataDir } = setup();
    const root = fixture("clean");
    await runner.open(root);
    runner.setThreads(2);
    for (const bad of [0, 33, 2.5, NaN, "4", null, -1]) {
      runner.setThreads(bad);
      expect(last("notice").message).toBe(THREADS_ERROR);
      expect(last("threads").count).toBe(2);
      expect(loadProjectThreads(dataDir, root)).toBe(2);
    }
  });

  it("rejects a change while busy", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const root = fixture("clean");
    await runner.open(root);
    const run = runner.run("f:tests/slow.feature");
    await until(() => all("runStarted").length > 0);
    const sent = all("threads").length;
    runner.setThreads(4);
    expect(last("notice").message).toBe("A run is already in progress");
    expect(all("threads")).toHaveLength(sent + 1); // main re-sends the count it holds
    expect(last("threads").count).toBe(1);
    expect(loadProjectThreads(dataDir, root)).toBe(1);
    await runner.cancel();
    await run;
  });

  it("re-run uses the current count, not the run's", T, async () => {
    const { runner, last, dataDir } = setup();
    const root = fixture("clean");
    await runner.open(root);
    runner.setThreads(2);
    await runner.run(byName(last("project").tree!, "passing hello").key);
    const { runId } = last("runStarted");
    runner.setThreads(1);
    await runner.rerun(runId);
    const second = last("runStarted").runId;
    expect(second).not.toBe(runId);
    const stored = (id: string) =>
      JSON.parse(
        readFileSync(path.join(projectDir(dataDir, root), "runs", id, "run.json"), "utf8"),
      );
    expect(stored(runId).threads).toBe(2);
    expect(stored(second).threads).toBe(1);
    expect(suiteThreads(eventsFile(dataDir, root, path.join("runs", second)))).toBe(1);
  });

  it("applies a count it cannot save", T, async () => {
    const { runner, last, dataDir } = setup();
    await runner.open(fixture("clean"));
    rmSync(path.join(dataDir, "settings.json")); // open saved it: replace it with a folder
    mkdirSync(path.join(dataDir, "settings.json")); // writing a file there fails
    runner.setThreads(4);
    expect(last("notice").message).toContain("The thread count could not be saved");
    expect(last("threads").count).toBe(4);
  });

  it("ignores a change with no project", () => {
    const { runner, events } = setup();
    runner.setThreads(4);
    expect(events).toEqual([]);
  });
});
