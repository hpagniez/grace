import { mkdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { cleanUp, T, setup, fixture, until } from "../../test/runner";
import type { AppEvent } from "../shared/events";
import {
  loadLastProject,
  loadProjectThreads,
  loadRecentProjects,
  saveOpenedProject,
} from "./settings";

afterEach(cleanUp);

describe("Runner home screen", () => {
  const BUSY = "A run is already in progress";
  const around = (events: AppEvent[]) =>
    events.filter((e) => e.type === "busy" || e.type === "opening" || e.type === "project");

  it("open emits opening around the discovery", T, async () => {
    const { runner, events, last } = setup();
    const root = fixture("clean");
    await runner.open(root);
    expect(around(events)).toEqual([
      { type: "busy", busy: true },
      { type: "opening", root: path.resolve(root) },
      last("project"),
      { type: "opening", root: null },
      { type: "busy", busy: false },
    ]);
  });

  it("a successful open saves the folder first and as lastProject", T, async () => {
    const { runner, all, dataDir } = setup();
    const clean = fixture("clean");
    const tags = fixture("tags");
    await runner.open(clean);
    await runner.open(tags);
    expect(loadRecentProjects(dataDir)).toEqual([tags, clean]);
    expect(loadLastProject(dataDir)).toBe(tags);
    await until(() => all("recent").some((e) => e.entries.length === 2));
    expect(all("recent").at(-1)!.entries).toEqual([
      { path: tags, exists: true },
      { path: clean, exists: true },
    ]);
  });

  it("a failed open from the home screen changes nothing", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const opening = runner.open(fixture("clean"));
    await runner.cancel();
    expect(await opening).toBe(false);
    expect(last("project")).toEqual({ type: "project", project: null, tree: null, error: null });
    expect(all("notice").map((n) => n.message)).toEqual(["Cancelled"]);
    expect(last("opening")).toEqual({ type: "opening", root: null });
    expect(loadRecentProjects(dataDir)).toEqual([]);
    expect(loadLastProject(dataDir)).toBeNull();
    const seen = all("opening").length;
    await runner.open(path.join(tmpdir(), "kk-does-not-exist"));
    expect(all("opening")).toHaveLength(seen);
  });

  it("a failed list save keeps the project open", T, async () => {
    const { runner, all, last, dataDir } = setup();
    mkdirSync(path.join(dataDir, "settings.json"));
    const root = fixture("clean");
    expect(await runner.open(root)).toBe(true);
    expect(last("project").project?.root).toBe(root);
    expect(
      all("notice").some((n) => n.message.startsWith("The recent list could not be saved: ")),
    ).toBe(true);
  });

  it("openRecent opens only a saved folder", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const clean = fixture("clean");
    const tags = fixture("tags");
    saveOpenedProject(dataDir, clean);
    await runner.openRecent(tags);
    expect(all("notice").map((n) => n.message)).toEqual(["Invalid request"]);
    expect(all("busy")).toEqual([]);
    await runner.openRecent(clean);
    expect(last("project").project?.root).toBe(clean);
  });

  it("a second openRecent of the folder being opened is ignored", T, async () => {
    const { runner, all, dataDir } = setup();
    const clean = fixture("clean");
    saveOpenedProject(dataDir, clean);
    const first = runner.openRecent(clean);
    await until(() => all("opening").length > 0);
    await runner.openRecent(clean);
    await first;
    expect(all("notice")).toEqual([]);
  });

  it("a second openRecent is ignored whichever way the entry is spelled", T, async () => {
    const { runner, all, dataDir } = setup();
    const listed = fixture("clean") + path.sep; // open resolves it, a listed entry may not be
    saveOpenedProject(dataDir, listed);
    const first = runner.openRecent(listed);
    await until(() => all("opening").length > 0);
    await runner.openRecent(listed);
    await first;
    expect(all("notice")).toEqual([]);
  });

  it("removeRecent says so when the list cannot be saved", T, async () => {
    const { runner, all, dataDir } = setup();
    const clean = fixture("clean");
    saveOpenedProject(dataDir, clean);
    mkdirSync(path.join(dataDir, "settings.json.tmp")); // the list stays readable, its new copy cannot be written
    runner.removeRecent(clean);
    expect(all("notice").map((n) => n.message)).toEqual([
      expect.stringMatching(/^The recent list could not be saved: /),
    ]);
    expect(loadRecentProjects(dataDir)).toEqual([clean]);
  });

  it("removeRecent drops a saved folder only", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const clean = fixture("clean");
    const tags = fixture("tags");
    saveOpenedProject(dataDir, tags);
    saveOpenedProject(dataDir, clean);
    runner.removeRecent(tags);
    expect(loadRecentProjects(dataDir)).toEqual([clean]);
    await until(() => all("recent").length > 0);
    expect(last("recent").entries).toEqual([{ path: clean, exists: true }]);
    runner.removeRecent("D:\\not-saved");
    expect(all("notice").map((n) => n.message)).toEqual(["Invalid request"]);
  });

  it("close forgets the project and lastProject, keeps the rest", T, async () => {
    const { runner, last, dataDir } = setup();
    const clean = fixture("clean");
    await runner.open(clean);
    runner.setThreads(2);
    runner.close();
    expect(last("project")).toEqual({ type: "project", project: null, tree: null, error: null });
    expect(runner.current()).toBeNull();
    expect(loadLastProject(dataDir)).toBeNull();
    expect(loadRecentProjects(dataDir)).toEqual([clean]);
    expect(loadProjectThreads(dataDir, clean)).toBe(2);
    expect(runner.replay()).toEqual([
      { type: "opening", root: null },
      { type: "busy", busy: false },
    ]);
    await runner.open(clean);
    expect(last("threads")).toEqual({ type: "threads", count: 2 });
  });

  it("close shows the notice after the project event, which the window clears", T, async () => {
    const { runner, events, last, dataDir } = setup();
    await runner.open(fixture("clean"));
    rmSync(path.join(dataDir, "settings.json")); // open saved it: replace it with a folder
    mkdirSync(path.join(dataDir, "settings.json")); // reading it as a file fails
    runner.close();
    expect(last("notice").message).toMatch(/^The last project could not be forgotten: /);
    expect(events.lastIndexOf(last("notice"))).toBeGreaterThan(events.lastIndexOf(last("project")));
  });

  it("close is refused while busy", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const clean = fixture("clean");
    const opening = runner.open(clean);
    await until(() => all("busy").length > 0);
    runner.close();
    await opening;
    expect(all("notice").map((n) => n.message)).toEqual([BUSY]);
    expect(all("project")).toHaveLength(1);
    expect(last("project").project?.root).toBe(clean);
    expect(loadLastProject(dataDir)).toBe(clean);
  });

  it("replay during an open carries the folder being opened", T, async () => {
    const { runner } = setup();
    const clean = fixture("clean");
    const opening = runner.open(clean);
    expect(runner.replay()).toEqual([
      { type: "opening", root: path.resolve(clean) },
      { type: "busy", busy: true },
    ]);
    await opening;
  });
});
