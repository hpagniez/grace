import path from "node:path";
import { describe, expect, it } from "vitest";
import type { AppEvent } from "../shared/events";
import type { TestNode } from "../shared/model";
import { readAllEvents } from "../main/karate/events";
import { RunState } from "../main/karate/run-state";
import { buildTree, leafKeys } from "../main/karate/tree";
import { CONSOLE_LIMIT, initialState, reduce } from "./reduce";

const fixture = (name: string) => readAllEvents(path.resolve("test/fixtures/events", name));
const tree: TestNode = buildTree(fixture("dryrun-clean.jsonl"), "clean");
const keys = leafKeys(tree);
const project = (t: TestNode | null = tree): AppEvent => ({
  type: "project",
  project: { root: "D:/p", layout: "standalone" },
  tree: t,
  error: null,
});
const other = { root: "D:/q", layout: "standalone" } as const;
const started = (ks: string[] = keys): AppEvent => ({
  type: "runStarted",
  selection: tree.key,
  keys: ks,
  total: ks.length,
  startedAt: 1000,
});
const apply = (s: ReturnType<typeof initialState>, ...es: AppEvent[]) =>
  es.forEach((e) => reduce(s, e));

describe("reduce", () => {
  it("runStarted sets selection to pending, keeps others, clears console, sets total", () => {
    const s = initialState();
    apply(s, project());
    s.statuses["other"] = "passed";
    s.console = ["old"];
    apply(s, started(keys.slice(0, 2)));
    expect(s.statuses[keys[0]]).toBe("pending");
    expect(s.statuses[keys[1]]).toBe("pending");
    expect(s.statuses["other"]).toBe("passed");
    expect(s.console).toEqual([]);
    expect(s.run).toMatchObject({ active: true, total: 2, done: 0, startedAt: 1000 });
  });

  it("replays a whole run: done 9, passed 8, failed 1, outcome finished", () => {
    const s = initialState();
    apply(s, project(), started());
    const rs = new RunState(tree, tree);
    apply(s, ...rs.apply(fixture("run-clean.jsonl")), ...rs.finish(1, false, true));
    expect(s.run).toMatchObject({
      active: false,
      done: 9,
      passed: 8,
      failed: 1,
      outcome: "finished",
    });
    expect(s.run.reportAvailable).toBe(true);
    expect(s.run.summary).not.toBeNull();
    expect(Object.keys(s.steps).length).toBeGreaterThan(0);
    expect(Object.values(s.results).some((r) => r.error)).toBe(true);
  });

  it("does not double count a repeated final status", () => {
    const s = initialState();
    apply(s, project(), started());
    const up: AppEvent = { type: "scenarios", updates: [{ key: keys[0], status: "passed" }] };
    apply(s, up, up);
    expect(s.run).toMatchObject({ done: 1, passed: 1 });
  });

  it("project event keeps statuses of existing keys and drops the others", () => {
    const s = initialState();
    apply(s, project());
    s.statuses[keys[0]] = "passed";
    s.statuses["gone"] = "failed";
    s.results["gone"] = { durationMillis: 1 };
    s.steps["gone"] = [];
    s.notice = "x";
    apply(s, project());
    expect(s.statuses[keys[0]]).toBe("passed");
    expect(s.statuses["gone"]).toBeUndefined();
    expect(s.results["gone"]).toBeUndefined();
    expect(s.steps["gone"]).toBeUndefined();
    expect(s.notice).toBeNull();
  });

  it("project event drops the status of a key that now names another test", () => {
    const s = initialState();
    apply(s, project());
    const [same, moved] = keys;
    for (const k of [same, moved]) {
      s.statuses[k] = "failed";
      s.results[k] = { error: "boom" };
      s.steps[k] = [];
    }
    s.selected = moved;
    // a scenario inserted above shifts section numbers: `moved` is now another scenario
    const shifted = structuredClone(tree);
    const rename = (n: TestNode) => {
      if (n.key === moved) n.name = "inserted";
      n.children.forEach(rename);
    };
    rename(shifted);
    apply(s, project(shifted));
    expect(s.statuses[same]).toBe("failed");
    expect(s.results[same]).toEqual({ error: "boom" });
    expect(s.steps[same]).toEqual([]);
    expect(s.statuses[moved]).toBeUndefined();
    expect(s.results[moved]).toBeUndefined();
    expect(s.steps[moved]).toBeUndefined();
    expect(s.selected).toBeNull();
  });

  it("project event for another folder resets the run view", () => {
    const s = initialState();
    apply(s, project(), started());
    const rs = new RunState(tree, tree);
    apply(s, ...rs.apply(fixture("run-clean.jsonl")), ...rs.finish(1, false, true));
    s.selected = keys[0];
    const runView = () => {
      const { statuses, results, steps, selected, run } = s;
      return { statuses, results, steps, selected, run };
    };
    const reset = { statuses: {}, results: {}, steps: {}, selected: null, run: initialState().run };
    apply(s, { ...(project() as Extract<AppEvent, { type: "project" }>), project: other });
    expect(runView()).toEqual(reset);
    apply(s, started(), { type: "project", project: null, tree: null, error: "Folder not found" });
    expect(runView()).toEqual(reset);
  });

  it("project event keeps a selection that still exists and clears one that is gone", () => {
    const s = initialState();
    apply(s, project());
    s.selected = keys[0];
    apply(s, project());
    expect(s.selected).toBe(keys[0]);
    s.selected = "gone";
    apply(s, project());
    expect(s.selected).toBeNull();
    s.selected = keys[0];
    apply(s, project(null));
    expect(s.selected).toBeNull();
  });

  it("console keeps the last 10000 lines", () => {
    const s = initialState();
    const chunk = (from: number, to: number): AppEvent => ({
      type: "console",
      lines: Array.from({ length: to - from }, (_, i) => `l${from + i}`),
    });
    apply(s, chunk(0, 4000), chunk(4000, 8000), chunk(8000, 10500));
    expect(s.console).toHaveLength(CONSOLE_LIMIT);
    expect(s.console.at(-1)).toBe("l10499");
    expect(s.console[0]).toBe("l500");
  });

  it("notice sets the notice", () => {
    const s = initialState();
    apply(s, { type: "notice", message: "hi" });
    expect(s.notice).toBe("hi");
  });
});
