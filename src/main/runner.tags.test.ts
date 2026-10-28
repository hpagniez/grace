import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { cleanUp, vendor, temps, T, setup, fixture, byName, until } from "../../test/runner";
import type { TestNode } from "../shared/model";
import { matchingLeaves, NO_FILTER, projectTags, type TagFilter } from "../shared/tags";
import { buildArgs, detectLayout, tagArgs } from "./karate/command";
import { readAllEvents } from "./karate/events";
import { spawnKarate } from "./karate/process";
import { buildTree } from "./karate/tree";
import { loadProjectFilter } from "./settings";

afterEach(cleanUp);

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
      second.last("flaky"),
      { type: "opening", root: null },
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
    rmSync(path.join(dataDir, "settings.json")); // open saved it: replace it with a folder
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
