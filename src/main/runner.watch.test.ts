import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { byName, cleanUp, fixture, projectDir, setup, T, until } from "../../test/runner";
import { NO_FILTER } from "../shared/tags";
import { loadProjectWatch, saveProjectWatch } from "./settings";
import { QUIET_MS } from "./watch";

afterEach(cleanUp);

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const PASSING = "Scenario: one\n  * match 1 == 1\n";
/** Saves `tests/<name>.feature` in the tags project. */
const save = (root: string, name: string, body = PASSING) =>
  writeFileSync(path.join(root, "tests", `${name}.feature`), `Feature: ${name}\n\n${body}`);
const BROKEN = "Scenario: bad gherkin\n  Given def x = \n  Examples:\n  | oops\nScenario Outline\n";

/** The tags project, open, with watch mode on. */
async function watching() {
  const s = setup();
  const root = fixture("tags");
  await s.runner.open(root);
  s.runner.setWatch(true);
  /** Operations started after this call: a watch run starts one through `guarded`. */
  const startsAfter = () => {
    const before = s.all("busy").filter((b) => b.busy).length;
    return () => s.all("busy").filter((b) => b.busy).length - before;
  };
  /** Past a batch's quiet time, and idle again. */
  const settle = async () => {
    await sleep(QUIET_MS + 500);
    await until(() => !s.runner.isBusy());
  };
  return { ...s, root, startsAfter, settle };
}

describe("Runner watch mode", () => {
  it("turning on saves it, sends it and is replayed (R1.3, R1.4)", T, async () => {
    const { runner, last, dataDir, root } = await watching();
    expect(last("watch")).toEqual({ type: "watch", on: true });
    expect(loadProjectWatch(dataDir, root)).toBe(true);
    expect(runner.replay()).toContainEqual({ type: "watch", on: true });
    await runner.dispose();
    const second = setup(dataDir);
    await second.runner.open(root);
    expect(second.last("watch").on).toBe(true);
    await second.runner.open(fixture("clean"));
    expect(second.last("watch").on).toBe(false);
  });

  it("refuses a value that is not a boolean (R1.3)", T, async () => {
    const { runner, last, dataDir, root } = await watching();
    runner.setWatch("yes");
    expect(last("notice").message).toBe("Invalid request");
    expect(last("watch").on).toBe(true);
    expect(loadProjectWatch(dataDir, root)).toBe(true);
  });

  it("applies a choice it cannot save, and says so (R1.3)", T, async () => {
    const { runner, last, dataDir } = await watching();
    rmSync(path.join(dataDir, "settings.json")); // open saved it: replace it with a folder
    mkdirSync(path.join(dataDir, "settings.json")); // writing a file there fails
    runner.setWatch(false);
    expect(last("notice").message).toMatch(/^Watch mode could not be saved: /);
    expect(last("watch").on).toBe(false);
  });

  it("runs a saved feature as its selection, and its run starts no other (R3)", T, async () => {
    const { all, last, dataDir, root, settle } = await watching();
    save(root, "w1");
    await until(() => all("runFinished").length === 1);
    const started = last("runStarted");
    expect(started.selection).toBe("f:tests/w1.feature");
    expect(started.keys).toEqual([byName(last("project").tree!, "one").key]);
    const record = JSON.parse(
      readFileSync(path.join(projectDir(dataDir, root), "runs", started.runId, "run.json"), "utf8"),
    );
    expect(record.selection.key).toBe("f:tests/w1.feature");
    await sleep(1500); // Karate read every feature
    await settle();
    expect(all("runStarted")).toHaveLength(1);
  });

  it("runs nothing while off", T, async () => {
    const { runner, root, startsAfter, settle } = await watching();
    runner.setWatch(false);
    const starts = startsAfter();
    save(root, "w1");
    await settle();
    expect(starts()).toBe(0);
  });

  it("runs two features saved together as one run with no selection (R3.3)", T, async () => {
    const { all, last, root, settle } = await watching();
    save(root, "w1");
    save(root, "w2");
    await until(() => all("runFinished").length === 1);
    await settle();
    expect(all("runStarted")).toHaveLength(1);
    expect(last("runStarted").selection).toBe(last("project").tree!.key);
    expect(last("runStarted").keys).toHaveLength(2);
  });

  it("runs only the tests that match the filter (R3.2)", T, async () => {
    const { runner, all, last, root } = await watching();
    runner.setFilter({ ...NO_FILTER, include: ["smoke"], exclude: [] });
    save(root, "w1", `@smoke\n${PASSING.replace("one", "tagged")}\n${PASSING}`);
    await until(() => all("runFinished").length === 1);
    expect(last("runStarted").keys).toEqual([byName(last("project").tree!, "tagged").key]);
  });

  it("skips a feature with nothing to run without a notice (R3.2)", T, async () => {
    const { all, root, startsAfter, settle } = await watching();
    const notices = all("notice").length;
    const starts = startsAfter();
    save(root, "helper", `@ignore\n${PASSING}`);
    await until(() => starts() === 1); // the dry run before a run
    await settle();
    expect(all("runStarted")).toHaveLength(0);
    expect(all("notice")).toHaveLength(notices);
  });

  it("runs a save made during a run once the run ends (R4.1)", T, async () => {
    const { runner, all, root } = await watching();
    const running = runner.run("d:");
    save(root, "w1");
    await running;
    await until(() => all("runFinished").length === 2);
    expect(all("runStarted").map((e) => e.selection)).toEqual(["d:", "f:tests/w1.feature"]);
    expect(all("notice").map((n) => n.message)).not.toContain("A run is already in progress");
  });

  it("Cancel drops the saves waiting (R4.2)", T, async () => {
    const { runner, all, root, settle } = await watching();
    const running = runner.run("d:");
    save(root, "w1");
    await sleep(QUIET_MS + 200); // the batch is waiting
    expect(runner.isBusy()).toBe(true);
    await runner.cancel();
    await running;
    await settle();
    expect(all("runStarted").map((e) => e.selection)).not.toContain("f:tests/w1.feature");
  });

  it("turning watch mode off drops the saves waiting (R4.4)", T, async () => {
    const { runner, all, root, settle } = await watching();
    const running = runner.run("d:");
    save(root, "w1");
    await sleep(QUIET_MS + 200);
    expect(runner.isBusy()).toBe(true);
    runner.setWatch(false);
    await running;
    await settle();
    expect(all("runStarted").map((e) => e.selection)).toEqual(["d:"]);
  });

  it("closing the project stops watching (R1.5)", T, async () => {
    const { runner, last, root, startsAfter, settle } = await watching();
    runner.close();
    expect(last("watch").on).toBe(false);
    const starts = startsAfter();
    save(root, "w1");
    await settle();
    expect(starts()).toBe(0);
  });

  it("opening another project stops watching the first (R1.5)", T, async () => {
    const { runner, root, startsAfter, settle } = await watching();
    await runner.open(fixture("clean"));
    const starts = startsAfter();
    save(root, "w1");
    await settle();
    expect(starts()).toBe(0);
  });

  it("a cancelled open keeps the project's watcher (R1.5)", T, async () => {
    const { runner, all, last, root } = await watching();
    const opening = runner.open(fixture("clean"));
    await runner.cancel();
    expect(await opening).toBe(false);
    save(root, "w1");
    await until(() => all("runFinished").length === 1);
    expect(last("runStarted").selection).toBe("f:tests/w1.feature");
  });

  it("watches a project whose dry run failed, and runs the fix (R1.4)", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const root = fixture("tags");
    save(root, "w1", BROKEN);
    saveProjectWatch(dataDir, root, true);
    await runner.open(root);
    expect(last("project").error).not.toBeNull();
    expect(last("watch").on).toBe(true);
    save(root, "w1");
    await until(() => all("runFinished").length === 1);
    expect(last("runStarted").selection).toBe("f:tests/w1.feature");
  });

  it("refuses the toggle while a project is being opened (R1.2)", T, async () => {
    const { runner, last, onEvent, dataDir } = setup();
    const root = fixture("tags");
    onEvent((e) => {
      if (e.type === "opening" && e.root) runner.setWatch(true);
    });
    await runner.open(root);
    expect(last("notice").message).toBe("A run is already in progress");
    expect(last("watch").on).toBe(false);
    expect(loadProjectWatch(dataDir, root)).toBe(false);
  });

  it("a watcher error stops watch mode in memory only (R2.5)", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const root = fixture("tags");
    writeFileSync(path.join(root, "karate-pom.json"), JSON.stringify({ paths: ["tests"] }));
    await runner.open(root);
    runner.setWatch(true);
    rmSync(path.join(root, "tests"), { recursive: true });
    await until(() => all("notice").some((n) => n.message.startsWith("Watch mode stopped: ")));
    expect(last("watch").on).toBe(false);
    expect(loadProjectWatch(dataDir, root)).toBe(true);
  });

  it("dispose stops it before a waiting batch runs", T, async () => {
    const { runner, root, startsAfter } = await watching();
    const starts = startsAfter();
    save(root, "w1");
    await runner.dispose();
    await sleep(QUIET_MS + 500);
    expect(starts()).toBe(0);
  });
});
