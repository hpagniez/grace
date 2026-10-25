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
import { fakeSecretBox, type SecretBox } from "./secrets";
import { loadProjectEnv, loadProjectFilter, saveProjectEnv } from "./settings";
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

function setup(
  dataDir = mkdtempSync(path.join(tmpdir(), "kk-data-")),
  secrets: SecretBox = fakeSecretBox(),
) {
  temps.push(dataDir);
  const events: AppEvent[] = [];
  const runner = new Runner({ vendor, dataDir, secrets, emit: (e) => events.push(e) });
  runners.push(runner);
  const all = <K extends AppEvent["type"]>(type: K) =>
    events.filter((e): e is Of<K> => e.type === type);
  const last = <K extends AppEvent["type"]>(type: K) => all(type).at(-1)!;
  return { runner, events, dataDir, all, last };
}

function fixture(name: "clean" | "broken" | "maven" | "edge" | "tags" | "env"): string {
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
    const { runner, last } = setup();
    expect(runner.replay()).toEqual([{ type: "busy", busy: false }]);
    const opening = runner.open(fixture("clean"));
    // The window may load before the first project event: it still learns the runner is busy.
    expect(runner.replay()).toEqual([{ type: "busy", busy: true }]);
    await opening;
    expect(runner.replay()).toEqual([
      runner.current(),
      { type: "filter", filter: NO_FILTER },
      last("environment"),
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
      env: null,
      argFile: null,
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

  it("does not run an outline's @ignore rows when every listed row matches", T, async () => {
    const { runner, all, last } = setup();
    await runner.open(fixture("tags"));
    const outline = byName(last("project").tree!, "speed <n>");
    expect(outline.children).toHaveLength(3); // the @ignore Examples row is not listed
    runner.setFilter(filter(["ol"]));
    await runner.run(outline.key);
    expect(last("runStarted").total).toBe(3);
    expect(last("runFinished").summary).toMatchObject({ scenarioCount: 3, scenariosPassed: 3 });
    expect(all("notice")).toHaveLength(0);
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
      second.last("environment"),
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

describe("Runner environment", () => {
  const names = (tree: TestNode | null) =>
    tree ? matchingLeaves(tree, NO_FILTER).map((n) => n.name) : [];
  const printed = (events: AppEvent[]) =>
    events.flatMap((e) => (e.type === "console" ? e.lines : []));
  /** Event types after `from`, without the busy events. */
  const typesAfter = (events: AppEvent[], from: number) =>
    events
      .slice(from)
      .map((e) => e.type)
      .filter((t) => t !== "busy");

  it("suggestions from overlays and tags", T, async () => {
    const { runner, events, last } = setup();
    await runner.open(fixture("env"));
    expect(last("environment").view).toMatchObject({
      env: null,
      suggestions: ["prod", "qa"],
      overlays: ["qa"],
    });
    expect(typesAfter(events, 0)).toEqual(["project", "filter", "environment"]);
  });

  it("setEnv rediscovers", T, async () => {
    const { runner, events, last, dataDir } = setup();
    const root = fixture("env");
    await runner.open(root);
    expect(names(last("project").tree)).not.toContain("qa only");
    expect(names(last("project").tree)).toContain("not prod");
    const from = events.length;
    await runner.setEnv("qa");
    expect(typesAfter(events, from)).toEqual(["project", "filter", "environment"]);
    expect(names(last("project").tree)).toContain("qa only");
    expect(last("environment").view.env).toBe("qa");
    // the tag of a scenario prod lacks is pruned, as after a Refresh
    runner.setFilter({ include: ["env=qa"], exclude: [], hideNonMatching: false });
    await runner.setEnv("prod");
    const tree = last("project").tree;
    expect(names(tree)).not.toContain("qa only");
    expect(names(tree)).not.toContain("not prod");
    expect(last("filter").filter).toEqual(NO_FILTER);
    expect(loadProjectFilter(dataDir, root)).toEqual(NO_FILTER);
  });

  it("a run uses -e", T, async () => {
    const { runner, events, last } = setup();
    await runner.open(fixture("env"));
    await runner.setEnv("qa");
    await runner.run(byName(last("project").tree!, "always").key);
    expect(printed(events).some((l) => l.includes("karate.env = qa"))).toBe(true);
    expect(last("runStarted").env).toBe("qa");
    expect(runner.reportPath()).not.toBeNull();
    // a result from qa is not the state of prod
    await runner.setEnv("prod");
    expect(runner.reportPath()).toBeNull();
  });

  it("setEnv saves and open restores", T, async () => {
    const first = setup();
    const root = fixture("env");
    const properties = {
      shared: [{ name: "timeout", value: "5000" }],
      byEnv: { qa: [{ name: "api.token", secret: fakeSecretBox().encrypt("tok-123") }] },
    };
    saveProjectEnv(first.dataDir, root, { env: null, properties });
    await first.runner.open(root);
    await first.runner.setEnv("qa");
    expect(loadProjectEnv(first.dataDir, root)).toEqual({ env: "qa", properties });
    await first.runner.dispose();
    const second = setup(first.dataDir);
    await second.runner.open(root);
    expect(second.last("environment").view.env).toBe("qa");
    expect(names(second.last("project").tree)).toContain("qa only");
    expect(second.runner.replay()).toEqual([
      second.runner.current(),
      { type: "filter", filter: NO_FILTER },
      second.last("environment"),
      { type: "busy", busy: false },
    ]);
  });

  it("the view shows secret rows without their values", T, async () => {
    const root = fixture("env");
    const dataDir = mkdtempSync(path.join(tmpdir(), "kk-data-"));
    const token = fakeSecretBox().encrypt("tok-123");
    saveProjectEnv(dataDir, root, {
      env: null,
      properties: {
        shared: [{ name: "timeout", value: "5000" }],
        byEnv: {
          qa: [
            { name: "api.token", secret: token },
            { name: "broken", secret: "Zm9v" },
          ],
          toString: [],
        },
      },
    });
    const { runner, last } = setup(dataDir);
    await runner.open(root);
    const { view } = last("environment");
    expect(view.suggestions).toEqual(["prod", "qa", "toString"]);
    expect(view.secretsAvailable).toBe(true);
    expect(view.properties).toEqual({
      shared: [{ name: "timeout", value: "5000" }],
      byEnv: {
        qa: [
          { name: "api.token", secret: true, state: "set" },
          { name: "broken", secret: true, state: "unreadable" },
        ],
        toString: [],
      },
    });
    expect(JSON.stringify(view)).not.toMatch(/tok-123|Zm9v/);
    expect(JSON.stringify(view)).not.toContain(token);

    const off = setup(dataDir, fakeSecretBox(false));
    await off.runner.open(root);
    const unavailable = off.last("environment").view;
    expect(unavailable.secretsAvailable).toBe(false);
    expect(unavailable.properties.byEnv.qa[0]).toEqual({
      name: "api.token",
      secret: true,
      state: "unreadable",
    });
  });

  it("cancel during setEnv restores the previous environment", T, async () => {
    const { runner, events, all, last, dataDir } = setup();
    const root = fixture("env");
    await runner.open(root);
    await runner.setEnv("qa");
    const before = runner.current();
    const view = last("environment");
    const from = events.length;
    const switching = runner.setEnv("prod");
    await runner.cancel();
    await switching;
    expect(typesAfter(events, from)).toEqual(["project", "filter", "environment", "notice"]);
    expect(all("project").at(-1)).toEqual(before);
    expect(last("environment")).toEqual(view);
    expect(last("notice").message).toBe("Cancelled");
    expect(loadProjectEnv(dataDir, root).env).toBe("qa");
    // the restored environment is the one discovery uses
    await runner.refresh();
    expect(names(last("project").tree)).toContain("qa only");
  });

  it("a parse error still applies and saves the environment", T, async () => {
    const { runner, last, dataDir } = setup();
    const root = fixture("broken");
    await runner.open(root);
    await runner.setEnv("qa");
    expect(last("project").error).toContain("syntax.feature");
    expect(last("environment").view.env).toBe("qa");
    expect(loadProjectEnv(dataDir, root).env).toBe("qa");
  });

  it("invalid env name is refused", T, async () => {
    const { runner, events, all, last, dataDir } = setup();
    const root = fixture("env");
    await runner.open(root);
    for (const name of ["bad name", "-Dx=1", "", "a".repeat(65)]) {
      const from = events.length;
      await runner.setEnv(name);
      expect(last("notice").message).toBe("Invalid environment name");
      // the view in force is re-sent, so a window that shows the typed name reverts
      expect(typesAfter(events, from)).toEqual(["notice", "environment"]);
    }
    await runner.setEnv(null); // the same environment: nothing to do
    expect(all("project")).toHaveLength(1);
    expect(last("environment").view.env).toBeNull();
    expect(loadProjectEnv(dataDir, root).env).toBeNull();
  });

  it("setEnv while busy gets the busy notice", T, async () => {
    const { runner, events, all, last } = setup();
    await runner.open(fixture("env"));
    const run = runner.run(byName(last("project").tree!, "slow").key);
    await until(() => all("runStarted").length > 0);
    const from = events.length;
    await runner.setEnv("qa");
    expect(last("notice").message).toBe("A run is already in progress");
    expect(typesAfter(events, from).filter((t) => t === "notice" || t === "environment")).toEqual([
      "notice",
      "environment",
    ]);
    await runner.cancel();
    await run;
    expect(all("environment")).toHaveLength(2);
    expect(last("environment").view.env).toBeNull();
  });

  it("a discovery that throws keeps the environment before", T, async () => {
    const { runner, events, last, dataDir } = setup();
    const root = fixture("env");
    await runner.open(root);
    const before = runner.replay();
    // a file where the run folders go: emptying the dry run folder throws
    const runs = path.join(
      dataDir,
      "projects",
      createHash("sha1").update(root).digest("hex").slice(0, 12),
    );
    rmSync(runs, { recursive: true, force: true, maxRetries: 5 });
    writeFileSync(runs, "");
    const from = events.length;
    await expect(runner.setEnv("qa")).rejects.toThrow();
    expect(typesAfter(events, from)).toEqual([]);
    expect(runner.replay()).toEqual(before);
    expect(loadProjectEnv(dataDir, root).env).toBeNull();
    rmSync(runs);
    await runner.refresh(); // discovers with the environment before
    expect(names(last("project").tree)).not.toContain("qa only");
  });

  it("KARATE_ENV in the parent environment is ignored", T, async () => {
    const saved = process.env.KARATE_ENV;
    process.env.KARATE_ENV = "prod";
    try {
      const { runner, events, last } = setup();
      await runner.open(fixture("env"));
      expect(names(last("project").tree)).toContain("not prod");
      await runner.run(byName(last("project").tree!, "always").key);
      expect(printed(events).some((l) => l.includes("karate.env = null"))).toBe(true);
      expect(last("runStarted").env).toBeNull();
    } finally {
      if (saved === undefined) delete process.env.KARATE_ENV;
      else process.env.KARATE_ENV = saved;
    }
  });
});
