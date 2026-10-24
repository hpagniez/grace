import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { copyFixture } from "../../test/fixtures/copy";
import type { AppEvent } from "../shared/events";
import type { TestNode } from "../shared/model";
import { matchingLeaves, NO_FILTER, projectTags, type TagFilter } from "../shared/tags";
import { buildArgs, detectLayout, tagArgs } from "./karate/command";
import { readAllEvents } from "./karate/events";
import { spawnKarate } from "./karate/process";
import { buildTree, leafKeys } from "./karate/tree";
import { Runner } from "./runner";
import { loadProjectFilter } from "./settings";
import { vendorPaths } from "./vendor";

const vendor = vendorPaths(path.resolve("vendor"));
const temps: string[] = [];
const runners: Runner[] = [];
const T = { timeout: 120_000 };

afterEach(async () => {
  await Promise.all(runners.splice(0).map((r) => r.dispose()));
  // Windows may hold a file briefly after a JVM exits
  for (const d of temps.splice(0)) rmSync(d, { recursive: true, force: true, maxRetries: 5 });
});

type Of<K extends AppEvent["type"]> = Extract<AppEvent, { type: K }>;

function setup(dataDir = mkdtempSync(path.join(tmpdir(), "kk-data-"))) {
  temps.push(dataDir);
  const events: AppEvent[] = [];
  const runner = new Runner({ vendor, dataDir, emit: (e) => events.push(e) });
  runners.push(runner);
  const all = <K extends AppEvent["type"]>(type: K) =>
    events.filter((e): e is Of<K> => e.type === type);
  const last = <K extends AppEvent["type"]>(type: K) => all(type).at(-1)!;
  return { runner, events, dataDir, all, last };
}

function fixture(name: "clean" | "broken" | "maven" | "edge" | "tags"): string {
  const d = copyFixture(name);
  temps.push(d);
  return d;
}

const leaves = (tree: TestNode | null) => (tree ? leafKeys(tree).length : -1);

function byName(tree: TestNode, name: string): TestNode {
  const all = (n: TestNode): TestNode[] => [n, ...n.children.flatMap(all)];
  const hit = all(tree).find((n) => n.name === name);
  if (!hit) throw new Error(`no node named ${name}`);
  return hit;
}

async function until(check: () => boolean, ms = 60_000): Promise<void> {
  const end = Date.now() + ms;
  while (!check()) {
    if (Date.now() > end) throw new Error("timed out waiting");
    await new Promise((r) => setTimeout(r, 50));
  }
}

function hashTree(root: string): Map<string, string> {
  const out = new Map<string, string>();
  for (const e of readdirSync(root, { recursive: true, withFileTypes: true })) {
    if (!e.isFile()) continue;
    const full = path.join(e.parentPath, e.name);
    out.set(
      path.relative(root, full).split(path.sep).join("/"),
      createHash("sha1").update(readFileSync(full)).digest("hex"),
    );
  }
  return out;
}

const ourJava = () =>
  execFileSync(
    "powershell",
    [
      "-NoProfile",
      "-NonInteractive",
      "-Command",
      "(Get-CimInstance Win32_Process -Filter \"Name='java.exe'\").ExecutablePath",
    ],
    { encoding: "utf8" },
  )
    .toLowerCase()
    .includes(path.resolve(vendor.jreDir).toLowerCase());

describe("Runner discovery", () => {
  it("opens the clean project", T, async () => {
    const { runner, last } = setup();
    const root = fixture("clean");
    await runner.open(root);
    const p = last("project");
    expect(p.project).toEqual({ root, layout: "standalone" });
    expect(leaves(p.tree)).toBe(9);
    expect(p.error).toBeNull();
    expect(runner.current()).toBe(p);
  });

  it("opens the maven fixture and runs it", T, async () => {
    const { runner, last } = setup();
    await runner.open(fixture("maven"));
    expect(last("project").project?.layout).toBe("maven");
    await runner.run("d:");
    const done = last("runFinished");
    expect(done.outcome).toBe("finished");
    expect(done.summary).toMatchObject({ scenariosPassed: 1, scenariosFailed: 0 });
  });

  it("reports a syntax error", T, async () => {
    const { runner, last } = setup();
    await runner.open(fixture("broken"));
    const p = last("project");
    expect(p.tree).toBeNull();
    expect(p.error).toContain("syntax.feature");
  });

  it("opens an empty folder", T, async () => {
    const { runner, last } = setup();
    const root = mkdtempSync(path.join(tmpdir(), "kk-empty-"));
    temps.push(root);
    await runner.open(root);
    const p = last("project");
    expect(leaves(p.tree)).toBe(0);
    expect(p.error).toBeNull();
  });

  it("open on a missing folder emits an error", T, async () => {
    const { runner, events } = setup();
    const missing = path.join(tmpdir(), "kk-does-not-exist");
    expect(await runner.open(missing)).toBe(false);
    expect(events).toEqual([
      { type: "busy", busy: true },
      { type: "project", project: null, tree: null, error: `Folder not found: ${missing}` },
      { type: "busy", busy: false },
    ]);
  });

  it("replay restores a window that loads during discovery", T, async () => {
    const { runner } = setup();
    expect(runner.replay()).toEqual([{ type: "busy", busy: false }]);
    const opening = runner.open(fixture("clean"));
    // The window may load before the first project event: it still learns the runner is busy.
    expect(runner.replay()).toEqual([{ type: "busy", busy: true }]);
    await opening;
    expect(runner.replay()).toEqual([
      runner.current(),
      { type: "filter", filter: NO_FILTER },
      { type: "busy", busy: false },
    ]);
  });

  it("cancel during open keeps the project open before", T, async () => {
    const { runner, all, events } = setup();
    expect(await runner.open(fixture("clean"))).toBe(true);
    const before = runner.current();
    const opening = runner.open(fixture("maven"));
    await runner.cancel();
    expect(await opening).toBe(false);
    expect(all("project").at(-1)).toEqual(before);
    expect(runner.current()).toBe(before);
    expect(all("notice").map((n) => n.message)).toEqual(["Cancelled"]);
    expect(events.at(-1)).toEqual({ type: "busy", busy: false });
    // the restored project is the one that runs
    await runner.run("s:tests/basic.feature#1");
    expect(all("runStarted")).toHaveLength(1);
  });

  it("cancel during the first open leaves no project", T, async () => {
    const { runner, all } = setup();
    const opening = runner.open(fixture("clean"));
    await runner.cancel();
    expect(await opening).toBe(false);
    expect(all("project")).toEqual([{ type: "project", project: null, tree: null, error: null }]);
    expect(runner.current()).toBeNull();
    await runner.refresh(); // no project: nothing to refresh
    expect(all("busy")).toHaveLength(2);
  });

  it("cancel during refresh keeps the tree", T, async () => {
    const { runner, all } = setup();
    await runner.open(fixture("clean"));
    const before = runner.current();
    const refreshing = runner.refresh();
    await runner.cancel();
    await refreshing;
    expect(all("project").at(-1)).toEqual(before);
    expect(all("notice").map((n) => n.message)).toEqual(["Cancelled"]);
  });

  it("opens a project whose path has spaces and accents", T, async () => {
    const { runner, last } = setup();
    const base = mkdtempSync(path.join(tmpdir(), "kk-intl-"));
    temps.push(base);
    const root = path.join(base, "Mes tests", "équipe");
    cpSync(fixture("clean"), root, { recursive: true });
    await runner.open(root);
    expect(last("project").error).toBeNull();
    expect(leaves(last("project").tree)).toBe(9);
  });
});

describe("Runner runs", () => {
  it("runs all with live progress", T, async () => {
    const { runner, events, last } = setup();
    await runner.open(fixture("clean"));
    await runner.run("d:");
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

describe("Runner tag filter", () => {
  const filter = (include: string[], exclude: string[] = [], hideNonMatching = false) => ({
    include,
    exclude,
    hideNonMatching,
  });

  function removeWip(root: string) {
    const file = path.join(root, "tests", "basic.feature");
    const text = readFileSync(file, "utf8");
    const edited = text.replace(/ ?@wip/g, "");
    expect(edited).not.toBe(text);
    writeFileSync(file, edited);
  }

  /** A dry run spawned like the Runner's, with `-t` from `f`. */
  async function dryRunTree(root: string, f: TagFilter): Promise<TestNode> {
    const outDir = mkdtempSync(path.join(tmpdir(), "kk-out-"));
    temps.push(outDir);
    const { classpath, featureRoots } = detectLayout(root);
    const args = buildArgs({
      jar: vendor.jar,
      classpath,
      root,
      outDir,
      dryRun: true,
      tags: tagArgs(f),
      targets: featureRoots,
    });
    expect(await spawnKarate(vendor.java, args, root, () => {}).exit).toBe(0);
    return buildTree(readAllEvents(path.join(outDir, "karate-json", "karate-events.jsonl")), "x");
  }

  it("lists inherited feature and examples tags", T, async () => {
    const { runner, last } = setup();
    await runner.open(fixture("tags"));
    expect(projectTags(last("project").tree!)).toEqual(["fast", "feat", "ol", "other", "slow"]);
  });

  it("runs all with a filter", T, async () => {
    const { runner, last } = setup();
    await runner.open(fixture("clean"));
    runner.setFilter(filter(["smoke"], ["wip"]));
    expect(last("filter").filter).toEqual(filter(["smoke"], ["wip"]));
    await runner.run("d:");
    expect(last("runStarted").total).toBe(2);
    expect(last("runFinished").summary).toMatchObject({ scenarioCount: 2, scenariosPassed: 2 });
  });

  it("filters a feature run", T, async () => {
    const { runner, last } = setup();
    await runner.open(fixture("clean"));
    runner.setFilter(filter([], ["wip"]));
    await runner.run("f:tests/basic.feature");
    expect(last("runStarted").total).toBe(2);
    expect(last("runFinished").summary).toMatchObject({ scenarioCount: 2, scenariosPassed: 2 });
  });

  it("runs the matching rows of an outline", T, async () => {
    const { runner, last } = setup();
    await runner.open(fixture("tags"));
    const outline = byName(last("project").tree!, "speed <n>");
    runner.setFilter(filter(["fast"]));
    await runner.run(outline.key);
    const fast = outline.children.slice(0, 2).map((r) => r.key);
    expect(last("runStarted")).toMatchObject({ keys: fast, total: 2 });
    expect(last("runFinished").summary).toMatchObject({ scenarioCount: 2, scenariosPassed: 2 });
  });

  it("refuses a clicked test that no longer matches", T, async () => {
    const { runner, all, last } = setup();
    const root = fixture("clean");
    await runner.open(root);
    const key = byName(last("project").tree!, "passing hello").key;
    runner.setFilter(filter(["smoke"]));
    const file = path.join(root, "tests", "basic.feature");
    const text = readFileSync(file, "utf8");
    const edited = text.replace(/@smoke(\r?\nScenario: passing hello)/, "$1");
    expect(edited).not.toBe(text);
    writeFileSync(file, edited);
    await runner.run(key);
    expect(last("notice").message).toBe("This test no longer matches the filter");
    expect(all("runStarted")).toHaveLength(0);
  });

  it("a vanished include tag runs nothing", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const root = fixture("clean");
    await runner.open(root);
    runner.setFilter(filter(["wip"]));
    removeWip(root);
    await runner.run("d:");
    expect(last("notice").message).toBe("Nothing to run");
    expect(all("runStarted")).toHaveLength(0);
    // the pre-run discovery neither prunes nor saves
    expect(all("filter")).toHaveLength(2);
    expect(runner.replay()).toContainEqual({ type: "filter", filter: filter(["wip"]) });
    expect(loadProjectFilter(dataDir, root)).toEqual(filter(["wip"]));
  });

  it("refresh prunes vanished tags and saves", T, async () => {
    const { runner, last, dataDir } = setup();
    const root = fixture("clean");
    await runner.open(root);
    runner.setFilter(filter(["wip"], ["slow"], true));
    removeWip(root);
    await runner.refresh();
    expect(last("filter").filter).toEqual(filter([], ["slow"], true));
    expect(loadProjectFilter(dataDir, root)).toEqual(filter([], ["slow"], true));
  });

  it("restores the filter on open", T, async () => {
    const first = setup();
    const root = fixture("clean");
    await first.runner.open(root);
    first.runner.setFilter(filter(["smoke"], ["wip"], true));
    await first.runner.dispose();
    const second = setup(first.dataDir);
    await second.runner.open(root);
    const saved = { type: "filter", filter: filter(["smoke"], ["wip"], true) };
    expect(second.last("filter")).toEqual(saved);
    expect(second.runner.replay()).toEqual([
      second.runner.current(),
      saved,
      { type: "busy", busy: false },
    ]);
  });

  it("a cancelled open restores the filter before", T, async () => {
    const { runner, last } = setup();
    await runner.open(fixture("clean"));
    runner.setFilter(filter(["smoke"]));
    const opening = runner.open(fixture("maven"));
    await runner.cancel();
    expect(await opening).toBe(false);
    expect(last("filter").filter).toEqual(filter(["smoke"]));
    await runner.run("d:");
    expect(last("runStarted").total).toBe(3);
  });

  it("applies a filter it cannot save", T, async () => {
    const { runner, last, dataDir } = setup();
    await runner.open(fixture("clean"));
    mkdirSync(path.join(dataDir, "settings.json")); // writing a file there fails
    runner.setFilter(filter(["smoke"]));
    expect(last("notice").message).toContain("The filter could not be saved");
    expect(last("filter").filter).toEqual(filter(["smoke"]));
    await runner.run("d:");
    expect(last("runStarted").total).toBe(3);
  });

  it("rejects a filter change while busy", T, async () => {
    const { runner, all, last } = setup();
    await runner.open(fixture("clean"));
    const run = runner.run("f:tests/slow.feature");
    await until(() => all("runStarted").length > 0);
    runner.setFilter(filter(["smoke"]));
    expect(last("notice").message).toBe("A run is already in progress");
    expect(last("filter").filter).toEqual(NO_FILTER);
    expect(runner.replay()).toContainEqual({ type: "filter", filter: NO_FILTER });
    await runner.cancel();
    await run;
  });

  it("Karate agrees with the matching function", T, async () => {
    const root = fixture("tags");
    const tree = await dryRunTree(root, NO_FILTER);
    // keys number rows by their position among the rows listed, so compare Karate's refIds
    const ids = (n: TestNode[]) => n.map((l) => `${l.file}|${l.refId}`);
    const cases: [TagFilter, number][] = [
      [filter(["fast"]), 2],
      [filter(["fast", "other"]), 3],
      [filter([], ["slow"]), 3],
    ];
    for (const [f, count] of cases) {
      const expected = ids(matchingLeaves(tree, f));
      expect(expected).toHaveLength(count);
      expect(ids(matchingLeaves(await dryRunTree(root, f), NO_FILTER))).toEqual(expected);
    }
  });
});
