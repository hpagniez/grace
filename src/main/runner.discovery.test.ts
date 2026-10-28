import { cpSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { cleanUp, temps, T, setup, fixture, leaves } from "../../test/runner";
import { NO_FILTER } from "../shared/tags";

afterEach(cleanUp);

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
    // the recent list check is asynchronous: it may or may not have arrived
    expect(events.filter((e) => e.type !== "recent")).toEqual([
      { type: "busy", busy: true },
      { type: "project", project: null, tree: null, error: `Folder not found: ${missing}` },
      { type: "flaky", tests: [] },
      { type: "busy", busy: false },
    ]);
  });

  it("replay restores a window that loads during discovery", T, async () => {
    const { runner, last } = setup();
    expect(runner.replay()).toEqual([
      { type: "opening", root: null },
      { type: "busy", busy: false },
    ]);
    const root = fixture("clean");
    const opening = runner.open(root);
    // The window may load before the first project event: it still learns the runner is busy.
    expect(runner.replay()).toEqual([
      { type: "opening", root },
      { type: "busy", busy: true },
    ]);
    await opening;
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

  it("cancel during open keeps the project open before", T, async () => {
    const { runner, all } = setup();
    expect(await runner.open(fixture("clean"))).toBe(true);
    const before = runner.current();
    const opening = runner.open(fixture("maven"));
    await runner.cancel();
    expect(await opening).toBe(false);
    expect(all("project").at(-1)).toEqual(before);
    expect(runner.current()).toBe(before);
    expect(all("notice").map((n) => n.message)).toEqual(["Cancelled"]);
    // the recent list is sent after its folder checks, so it may come after busy
    expect(all("busy").at(-1)).toEqual({ type: "busy", busy: false });
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
