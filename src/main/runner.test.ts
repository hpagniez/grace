import { createHash } from "node:crypto";
import { execFileSync, spawn } from "node:child_process";
import {
  closeSync,
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  openSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { copyFixture } from "../../test/fixtures/copy";
import { holdFolder } from "../../test/hold";
import type { PropertiesEdit, RowEdit } from "../shared/env";
import type { AppEvent } from "../shared/events";
import type { RunRecord, StoredTest } from "../shared/history";
import type { TestNode } from "../shared/model";
import { matchingLeaves, NO_FILTER, projectTags, type TagFilter } from "../shared/tags";
import { THREADS_ERROR } from "../shared/threads";
import { projectFingerprint } from "./fingerprint";
import { createRunDir, writeRunRecord } from "./history";
import { buildArgs, detectLayout, tagArgs } from "./karate/command";
import { readAllEvents } from "./karate/events";
import { spawnKarate } from "./karate/process";
import { buildTree, leafKeys } from "./karate/tree";
import { Runner } from "./runner";
import { fakeSecretBox, type SecretBox } from "./secrets";
import { loadProjectEnv, loadProjectFilter, loadProjectThreads, saveProjectEnv } from "./settings";
import { vendorPaths, type VendorPaths } from "./vendor";

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
  paths: VendorPaths = vendor,
  fingerprint?: (root: string) => Promise<string | null>,
) {
  temps.push(dataDir);
  const events: AppEvent[] = [];
  let listener = (_e: AppEvent) => {};
  const runner = new Runner({
    vendor: paths,
    dataDir,
    secrets,
    emit: (e) => {
      events.push(e);
      listener(e);
    },
    fingerprint,
  });
  runners.push(runner);
  const all = <K extends AppEvent["type"]>(type: K) =>
    events.filter((e): e is Of<K> => e.type === type);
  const last = <K extends AppEvent["type"]>(type: K) => all(type).at(-1)!;
  /** Calls `f` with each event as it is emitted, before the runner goes on. */
  const onEvent = (f: (e: AppEvent) => void) => (listener = f);
  return { runner, events, dataDir, all, last, onEvent };
}

function fixture(name: "clean" | "broken" | "maven" | "edge" | "tags" | "env" | "history"): string {
  const d = copyFixture(name);
  temps.push(d);
  return d;
}

const printed = (events: AppEvent[]) =>
  events.flatMap((e) => (e.type === "console" ? e.lines : []));

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

/** `<dataDir>/projects/<hash>`, where the Runner keeps a project's runs and dry run. */
const projectDir = (dataDir: string, root: string) =>
  path.join(dataDir, "projects", createHash("sha1").update(root).digest("hex").slice(0, 12));

/** The `-T` a Karate run was given, as its first SUITE_ENTER records it. */
const suiteThreads = (file: string): number =>
  readAllEvents(file).find((e) => e.type === "SUITE_ENTER")!.data.threads;

/** Command lines of the running java.exe processes of the project at `root`. */
const javaCommandLines = (root: string) =>
  execFileSync(
    "powershell",
    [
      "-NoProfile",
      "-NonInteractive",
      "-Command",
      "(Get-CimInstance Win32_Process -Filter \"Name='java.exe'\").CommandLine",
    ],
    { encoding: "utf8" },
  )
    .split(/\r?\n/)
    .filter((l) => l.includes(root));

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
      { type: "threads", count: 1 },
      last("environment"),
      last("history"),
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
      threads: 1,
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
      { type: "threads", count: 1 },
      second.last("environment"),
      second.last("history"),
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

describe("Runner environment", () => {
  const names = (tree: TestNode | null) =>
    tree ? matchingLeaves(tree, NO_FILTER).map((n) => n.name) : [];
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
    expect(typesAfter(events, 0)).toEqual([
      "project",
      "filter",
      "threads",
      "environment",
      "history",
    ]);
  });

  it("setEnv rediscovers", T, async () => {
    const { runner, events, last, dataDir } = setup();
    const root = fixture("env");
    await runner.open(root);
    expect(names(last("project").tree)).not.toContain("qa only");
    expect(names(last("project").tree)).toContain("not prod");
    const from = events.length;
    await runner.setEnv("qa");
    expect(typesAfter(events, from)).toEqual([
      "project",
      "filter",
      "threads",
      "environment",
      "history",
    ]);
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
      { type: "threads", count: 1 },
      second.last("environment"),
      second.last("history"),
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
    expect(typesAfter(events, from)).toEqual([
      "project",
      "filter",
      "threads",
      "environment",
      "history",
      "notice",
    ]);
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

  it("a change refused during a discovery re-sends the environment in force", T, async () => {
    const { runner, events, all, last } = setup();
    await runner.open(fixture("env"));
    const switching = runner.setEnv("qa");
    await until(() => all("busy").at(-1)?.busy === true);
    const from = events.length;
    await runner.setEnv("prod");
    // the window must not show the environment still being discovered
    expect(typesAfter(events, from)).toEqual(["notice", "environment"]);
    expect(last("notice").message).toBe("A run is already in progress");
    expect(last("environment").view.env).toBeNull();
    const replayed = runner.replay().find((e): e is Of<"environment"> => e.type === "environment");
    expect(replayed?.view.env).toBeNull();
    await switching;
    expect(last("environment").view.env).toBe("qa");
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
    // the window still shows the name it sent until main re-sends the environment in force
    expect(typesAfter(events, from)).toEqual(["environment"]);
    expect(last("environment").view.env).toBeNull();
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

describe("Runner properties", () => {
  const SECRET = "s3cret-token-123";
  const plain = (name: string, value: string): RowEdit => ({ name, value, secret: false });
  const secret = (name: string, value: string): RowEdit => ({ name, value, secret: true });
  const keep = (name: string): RowEdit => ({ name, secret: true, keep: true });
  const argFile = (dataDir: string, root: string) =>
    path.join(
      dataDir,
      "projects",
      createHash("sha1").update(root).digest("hex").slice(0, 12),
      "run-args.txt",
    );

  it("shared and env properties, env wins", T, async () => {
    const { runner, events, last } = setup();
    await runner.open(fixture("env"));
    runner.saveProperties({
      shared: [plain("timeout", "5000")],
      byEnv: { qa: [plain("timeout", "9000")] },
    });
    await runner.setEnv("qa");
    await runner.run(byName(last("project").tree!, "always").key);
    expect(printed(events).some((l) => l.includes("timeout = 9000"))).toBe(true);
    const from = events.length;
    await runner.setEnv(null);
    await runner.run(byName(last("project").tree!, "always").key);
    expect(printed(events.slice(from)).some((l) => l.includes("timeout = 5000"))).toBe(true);
  });

  it("a secret reaches the run and is masked", T, async () => {
    const { runner, events, all, last } = setup();
    const root = fixture("env");
    // a failing match puts the value in the scenario's error too
    writeFileSync(
      path.join(root, "tests", "leak.feature"),
      [
        "Feature: leak",
        "",
        "Scenario: leak",
        "  * print 'api.token =', karate.properties['api.token']",
        "  * match karate.properties['api.token'] == 'other'",
        "",
      ].join("\n"),
    );
    await runner.open(root);
    await runner.setEnv("qa");
    runner.saveProperties({ shared: [], byEnv: { qa: [secret("api.token", SECRET)] } });
    expect(last("environment").view.properties.byEnv.qa).toEqual([
      { name: "api.token", secret: true, state: "set" },
    ]);
    await runner.run("f:tests/leak.feature");
    expect(last("runFinished").summary).toMatchObject({ scenariosFailed: 1 });
    expect(printed(events).some((l) => l.includes("api.token = ****"))).toBe(true);
    const failed = all("scenarios")
      .flatMap((e) => e.updates)
      .find((u) => u.status === "failed");
    expect(failed?.error).toContain("****");
    const logs = all("steps").flatMap((e) => e.steps.map((s) => s.log ?? ""));
    expect(logs.some((l) => l.includes("api.token = ****"))).toBe(true);
    expect(JSON.stringify(events)).not.toContain(SECRET);
  });

  it("settings never hold the plain secret", T, async () => {
    const { runner, dataDir } = setup();
    const root = fixture("env");
    await runner.open(root);
    runner.saveProperties({ shared: [secret("api.token", SECRET)], byEnv: {} });
    expect(readFileSync(path.join(dataDir, "settings.json"), "utf8")).not.toContain(SECRET);
    expect(loadProjectEnv(dataDir, root).properties.shared).toEqual([
      { name: "api.token", secret: fakeSecretBox().encrypt(SECRET) },
    ]);
  });

  it("properties it cannot save are applied with a notice", T, async () => {
    const { runner, events, last, dataDir } = setup();
    await runner.open(fixture("env"));
    mkdirSync(path.join(dataDir, "settings.json")); // writing a file there fails
    expect(runner.saveProperties({ shared: [plain("timeout", "5000")], byEnv: {} })).toBe(true); // applied, though not saved
    expect(last("notice").message).toContain("The properties could not be saved");
    expect(last("environment").view.properties.shared).toEqual([
      { name: "timeout", value: "5000" },
    ]);
    await runner.run(byName(last("project").tree!, "always").key);
    expect(printed(events).some((l) => l.includes("timeout = 5000"))).toBe(true);
  });

  it("the secret is not on the command line", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const root = fixture("env");
    await runner.open(root);
    runner.saveProperties({ shared: [secret("api.token", SECRET)], byEnv: {} });
    const saved = loadProjectEnv(dataDir, root);
    const run = runner.run(byName(last("project").tree!, "slow").key);
    await until(() => all("runStarted").length > 0);
    const lines = javaCommandLines(root);
    expect(lines).toHaveLength(1);
    expect(lines[0]).toContain(`@${argFile(dataDir, root)}`);
    expect(lines[0]).not.toContain(SECRET);
    // a save while busy is refused
    expect(runner.saveProperties({ shared: [], byEnv: {} })).toBe(false);
    expect(last("notice").message).toBe("A run is already in progress");
    expect(loadProjectEnv(dataDir, root)).toEqual(saved);
    await run;
    expect(last("runFinished").outcome).toBe("finished");
  });

  it("no argument file is left after a run, a cancel, or a failed spawn", T, async () => {
    const paths = { ...vendor };
    const { runner, events, all, last, dataDir } = setup(undefined, undefined, paths);
    const root = fixture("env");
    const file = argFile(dataDir, root);
    const key = (name: string) => byName(last("project").tree!, name).key;
    await runner.open(root);
    runner.saveProperties({ shared: [plain("timeout", "5000")], byEnv: {} });

    await runner.run(key("always"));
    expect(last("runFinished").outcome).toBe("finished");
    expect(existsSync(file)).toBe(false);

    const slow = runner.run(key("slow"));
    await until(() => all("runStarted").length === 2);
    expect(readFileSync(file, "latin1")).toBe('"-Dtimeout=5000"\r\n');
    await runner.cancel();
    await slow;
    expect(last("runFinished").outcome).toBe("cancelled");
    expect(existsSync(file)).toBe(false);

    const pidFile = path.join(dataDir, "running.pid");
    mkdirSync(pidFile); // writing a file there fails
    try {
      await runner.run(key("always"));
      expect(last("notice").message).toContain("EISDIR");
      expect(existsSync(file)).toBe(false);
    } finally {
      rmSync(pidFile, { recursive: true, force: true });
    }

    // the pre-run dry run is spawned before run() first awaits: only the run gets the missing Java
    const from = events.length;
    const failing = runner.run(key("always"));
    paths.java = path.join(dataDir, "missing", "java.exe");
    await failing;
    expect(last("runFinished").outcome).toBe("failed");
    expect(printed(events.slice(from)).some((l) => l.includes("ENOENT"))).toBe(true);
    expect(existsSync(file)).toBe(false);
  });

  it("an argument file another process holds is a notice, not an error", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const root = fixture("env");
    await runner.open(root);
    runner.saveProperties({ shared: [plain("timeout", "5000")], byEnv: {} });
    const run = runner.run(byName(last("project").tree!, "slow").key);
    await until(() => all("runStarted").length > 0);
    // without delete sharing, as an antivirus or an indexer can, until its input closes
    const holder = spawn("powershell", [
      "-NoProfile",
      "-NonInteractive",
      "-Command",
      `$f = [IO.File]::Open('${argFile(dataDir, root)}', 'Open', 'Read', 'ReadWrite'); 'held'; [Console]::In.ReadLine(); $f.Close()`,
    ]);
    const exited = new Promise((r) => holder.on("exit", r));
    try {
      await new Promise<void>((resolve, reject) => {
        holder.stdout.on("data", (d) => String(d).includes("held") && resolve());
        void exited.then(() => reject(new Error("the holder exited")));
      });
      await run;
      expect(last("runFinished").outcome).toBe("finished");
      expect(last("notice").message).toContain("could not be deleted");
    } finally {
      holder.stdin.end();
      await exited;
    }
  });

  it("an unreadable secret blocks the run", T, async () => {
    const first = setup();
    const root = fixture("env");
    await first.runner.open(root);
    first.runner.saveProperties({ shared: [secret("api.token", SECRET)], byEnv: {} });
    await first.runner.dispose();
    const settings = path.join(first.dataDir, "settings.json");
    const cipher = fakeSecretBox().encrypt(SECRET);
    writeFileSync(settings, readFileSync(settings, "utf8").replace(cipher, "Zm9v"));

    const { runner, all, last } = setup(first.dataDir);
    await runner.open(root);
    expect(last("environment").view.properties.shared).toEqual([
      { name: "api.token", secret: true, state: "unreadable" },
    ]);
    await runner.run("d:");
    expect(last("notice").message).toBe(
      "Secret api.token (All environments) can't be read. Replace it in Properties.",
    );
    expect(all("runStarted")).toHaveLength(0);
    // refused before the pre-run dry run: no JVM started; discovery still works
    expect(all("project")).toHaveLength(1);
    expect(last("project").tree).not.toBeNull();

    // with secret storage unavailable, a secret of the environment cannot be used either
    saveProjectEnv(first.dataDir, root, {
      env: "qa",
      properties: { shared: [], byEnv: { qa: [{ name: "api.token", secret: cipher }] } },
    });
    const off = setup(first.dataDir, fakeSecretBox(false));
    await off.runner.open(root);
    await off.runner.run("d:");
    expect(off.last("notice").message).toBe(
      "Secret api.token (qa) can't be read. Replace it in Properties.",
    );
    expect(off.all("runStarted")).toHaveLength(0);
    expect(off.all("project")).toHaveLength(1);
  });

  it("keep copies the stored ciphertext, a rename needs a new value", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const root = fixture("env");
    await runner.open(root);
    runner.saveProperties({
      shared: [secret("api.token", SECRET), plain("timeout", "5000")],
      byEnv: {},
    });
    const cipher = fakeSecretBox().encrypt(SECRET);
    const applied = runner.saveProperties({
      shared: [keep("api.token"), plain("timeout", "6000")],
      byEnv: { qa: [] },
    });
    expect(applied).toBe(true);
    expect(all("notice")).toHaveLength(0);
    expect(loadProjectEnv(dataDir, root).properties).toEqual({
      shared: [
        { name: "api.token", secret: cipher },
        { name: "timeout", value: "6000" },
      ],
      byEnv: { qa: [] },
    });
    expect(last("environment").view.properties).toEqual({
      shared: [
        { name: "api.token", secret: true, state: "set" },
        { name: "timeout", value: "6000" },
      ],
      byEnv: { qa: [] },
    });

    const saved = loadProjectEnv(dataDir, root);
    const views = all("environment").length;
    const cases: [PropertiesEdit, string][] = [
      [{ shared: [keep("api.key")], byEnv: {} }, "All environments: api.key"],
      [{ shared: [], byEnv: { qa: [keep("api.token")] } }, "qa: api.token"],
      [{ shared: [keep("timeout")], byEnv: {} }, "All environments: timeout"],
    ];
    for (const [edit, row] of cases) {
      expect(runner.saveProperties(edit)).toBe(false);
      expect(last("notice").message).toBe(`Properties not saved: ${row}: No saved secret to keep`);
    }
    expect(loadProjectEnv(dataDir, root)).toEqual(saved);
    expect(all("environment")).toHaveLength(views);
  });

  it("reserved names and bad values are refused", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const root = fixture("env");
    expect(runner.saveProperties({ shared: [], byEnv: {} })).toBe(false); // no project
    await runner.open(root);
    const views = all("environment").length;
    const cases: [PropertiesEdit, string][] = [
      [{ shared: [plain("karate.options", "-t @x")], byEnv: {} }, "karate.options: Reserved name"],
      [{ shared: [plain("JAVA.home", "x")], byEnv: {} }, "JAVA.home: Reserved name"],
      [{ shared: [plain("a", "x\ny")], byEnv: {} }, "a: No line breaks"],
      [
        { shared: [secret("a", "é")], byEnv: {} },
        "a: This value cannot be passed to Java exactly.",
      ],
      [{ shared: [plain("a", "1"), plain("a", "2")], byEnv: {} }, "a: Duplicate name"],
    ];
    for (const [edit, error] of cases) {
      expect(runner.saveProperties(edit)).toBe(false);
      expect(last("notice").message).toBe(`Properties not saved: All environments: ${error}`);
    }
    expect(all("environment")).toHaveLength(views);
    expect(existsSync(path.join(dataDir, "settings.json"))).toBe(false);

    const off = setup(undefined, fakeSecretBox(false));
    await off.runner.open(root);
    expect(off.runner.saveProperties({ shared: [secret("api.token", SECRET)], byEnv: {} })).toBe(
      false,
    );
    expect(off.last("notice").message).toBe("Secret storage is not available on this system");
    expect(existsSync(path.join(off.dataDir, "settings.json"))).toBe(false);
    // plain rows still save
    expect(off.runner.saveProperties({ shared: [plain("timeout", "1")], byEnv: {} })).toBe(true);
    expect(loadProjectEnv(off.dataDir, root).properties.shared).toEqual([
      { name: "timeout", value: "1" },
    ]);
  });

  it("saved rows today's rules reject are named in one notice on open", T, async () => {
    const root = fixture("env");
    const dataDir = mkdtempSync(path.join(tmpdir(), "kk-data-"));
    const properties = {
      shared: [
        { name: "timeout", value: "5000" },
        { name: "java.home", value: "C:/jdk" },
      ],
      byEnv: {
        qa: [{ name: "greeting", secret: "Zm9v", value: "h\u00e9llo" }, { secret: "Zm9v" }],
      },
    };
    writeFileSync(
      path.join(dataDir, "settings.json"),
      JSON.stringify({ projects: { [root]: { properties } } }),
    );
    const { runner, all, last } = setup(dataDir);
    await runner.open(root);
    await runner.refresh();
    expect(all("notice").map((n) => n.message)).toEqual([
      "Some saved properties were not loaded and are kept in settings.json: All environments: java.home, qa: greeting, qa: (no name)",
    ]);
    expect(last("environment").view.properties).toEqual({
      shared: [{ name: "timeout", value: "5000" }],
      byEnv: { qa: [] },
    });
  });

  it("no properties, no @ argument", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const root = fixture("env");
    await runner.open(root);
    // rows of another environment are not passed
    runner.saveProperties({ shared: [], byEnv: { qa: [plain("timeout", "9000")] } });
    const run = runner.run(byName(last("project").tree!, "slow").key);
    await until(() => all("runStarted").length > 0);
    const lines = javaCommandLines(root);
    expect(lines).toHaveLength(1);
    expect(lines[0]).not.toContain("run-args.txt");
    expect(lines[0]).not.toMatch(/\s"?@/);
    expect(existsSync(argFile(dataDir, root))).toBe(false);
    await runner.cancel();
    await run;
  });
});

describe("Runner history", () => {
  const SECRET = "s3cret-token-123";
  /** The run folder names of the project, oldest first. */
  const folders = (dataDir: string, root: string) => {
    try {
      return readdirSync(path.join(projectDir(dataDir, root), "runs")).sort();
    } catch {
      return [];
    }
  };
  const recordFile = (dataDir: string, root: string, id: string) =>
    path.join(projectDir(dataDir, root), "runs", id, "run.json");
  const record = (dataDir: string, root: string, id: string): RunRecord =>
    JSON.parse(readFileSync(recordFile(dataDir, root, id), "utf8"));
  /** Run All records, oldest first, with `over` applied to each. */
  const seed = (dataDir: string, root: string, count: number, over: Partial<RunRecord> = {}) =>
    Array.from({ length: count }, (_, i) => {
      const at = new Date(2020, 0, 1, 0, 0, i);
      const { id, dir } = createRunDir(projectDir(dataDir, root), at);
      writeRunRecord(dir, {
        version: 1,
        startedAt: at.getTime(),
        durationMillis: 1000,
        env: null,
        threads: 1,
        fingerprint: null,
        filter: NO_FILTER,
        selection: { kind: "folder", file: "", name: path.basename(root), line: null, key: "d:" },
        failedOf: null,
        outcome: "finished",
        exitCode: 0,
        tests: [],
        ...over,
      });
      return id;
    });
  const stored = (n: TestNode, status: StoredTest["status"]): StoredTest => ({
    kind: n.kind as StoredTest["kind"],
    file: n.file,
    refId: n.refId!,
    name: n.name,
    line: n.line,
    key: n.key,
    status,
  });
  const smoke = { include: ["smoke"], exclude: [], hideNonMatching: false };
  const GONE = "This run is no longer in the history";

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

  it("a run of an unchanged project stores its fingerprint", T, async () => {
    const { runner, last, dataDir } = setup();
    const root = fixture("clean");
    await runner.open(root);
    await runner.run(byName(last("project").tree!, "passing hello").key);
    const [id] = folders(dataDir, root);
    const { fingerprint } = record(dataDir, root, id);
    expect(fingerprint).not.toBeNull();
    expect(fingerprint).toBe(await projectFingerprint(root));
  });

  it("a file saved during a run stores no fingerprint", T, async () => {
    const { runner, all, dataDir } = setup();
    const root = fixture("clean");
    await runner.open(root);
    const run = runner.run("f:tests/slow.feature");
    await until(() => all("runStarted").length > 0);
    writeFileSync(path.join(root, "note.txt"), "saved during the run");
    await run;
    const [id] = folders(dataDir, root);
    expect(record(dataDir, root, id).fingerprint).toBeNull();
  });

  it("different walks store no fingerprint", T, async () => {
    const walks = ["a".repeat(64), "b".repeat(64)];
    const fake = async () => walks.shift()!;
    const { runner, last, dataDir } = setup(undefined, undefined, undefined, fake);
    const root = fixture("clean");
    await runner.open(root);
    await runner.run(byName(last("project").tree!, "passing hello").key);
    expect(walks).toEqual([]);
    const [id] = folders(dataDir, root);
    expect(record(dataDir, root, id).fingerprint).toBeNull();
  });

  it("a cancel during the first walk starts no Java", T, async () => {
    let runner: Runner | undefined;
    const fake = async () => {
      await runner!.cancel();
      return "a".repeat(64);
    };
    const s = setup(undefined, undefined, undefined, fake);
    runner = s.runner;
    const root = fixture("clean");
    await runner.open(root);
    await runner.run(byName(s.last("project").tree!, "passing hello").key);
    expect(s.all("runStarted")).toHaveLength(0);
    expect(s.last("notice").message).toBe("Run cancelled");
    expect(folders(s.dataDir, root)).toEqual([]);
  });

  it("dispose skips the second walk", T, async () => {
    let walks = 0;
    const fake = async () => {
      walks++;
      return "a".repeat(64);
    };
    const { runner, all, dataDir } = setup(undefined, undefined, undefined, fake);
    const root = fixture("clean");
    await runner.open(root);
    const run = runner.run("f:tests/slow.feature");
    await until(() => all("runStarted").length > 0);
    await runner.dispose();
    expect(walks).toBe(1);
    const [id] = folders(dataDir, root);
    expect(record(dataDir, root, id)).toMatchObject({ outcome: "cancelled", fingerprint: null });
    await run;
  });

  it("runFinished comes before the run's history", T, async () => {
    const { runner, events, last } = setup();
    const root = fixture("clean");
    await runner.open(root);
    await runner.run(byName(last("project").tree!, "passing hello").key);
    const types = events.map((e) => e.type);
    expect(types.lastIndexOf("runFinished")).toBeLessThan(types.lastIndexOf("history"));
  });
});
