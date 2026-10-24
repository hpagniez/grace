import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { NO_FILTER, type FilterState } from "../shared/tags";
import { loadLastProject, loadProjectFilter, saveLastProject, saveProjectFilter } from "./settings";

const temps: string[] = [];
const tmp = () => {
  const d = mkdtempSync(path.join(tmpdir(), "kk-settings-"));
  temps.push(d);
  return d;
};
afterEach(() => temps.splice(0).forEach((d) => rmSync(d, { recursive: true, force: true })));

describe("settings", () => {
  it("save then load returns the folder", () => {
    const data = tmp();
    const project = path.join(tmp(), "proj");
    mkdirSync(project);
    saveLastProject(data, project);
    expect(loadLastProject(data)).toBe(project);
  });

  it("loadLastProject ignores a missing folder", () => {
    const data = tmp();
    const project = path.join(tmp(), "gone");
    mkdirSync(project);
    saveLastProject(data, project);
    rmSync(project, { recursive: true });
    expect(loadLastProject(data)).toBeNull();
  });

  it("returns null when there is no settings file", () => {
    expect(loadLastProject(tmp())).toBeNull();
  });

  it("returns null for a corrupt settings.json", () => {
    const data = tmp();
    writeFileSync(path.join(data, "settings.json"), "{not json");
    expect(loadLastProject(data)).toBeNull();
  });

  describe("tag filter", () => {
    const a: FilterState = { include: ["smoke"], exclude: ["wip"], hideNonMatching: true };
    const b: FilterState = { include: [], exclude: ["slow"], hideNonMatching: false };

    it("filter round-trips per project", () => {
      const data = tmp();
      saveProjectFilter(data, "proj-one", a);
      saveProjectFilter(data, "proj-two", b);
      expect(loadProjectFilter(data, "proj-one")).toEqual(a);
      expect(loadProjectFilter(data, "proj-two")).toEqual(b);
      expect(loadProjectFilter(data, "proj-three")).toEqual(NO_FILTER);
    });

    it("saveLastProject keeps saved filters", () => {
      const data = tmp();
      const project = path.join(tmp(), "proj");
      mkdirSync(project);
      saveProjectFilter(data, project, a);
      saveLastProject(data, project);
      expect(loadProjectFilter(data, project)).toEqual(a);
      expect(loadLastProject(data)).toBe(project);
    });

    it("saveProjectFilter keeps lastProject", () => {
      const data = tmp();
      const project = path.join(tmp(), "proj");
      mkdirSync(project);
      saveLastProject(data, project);
      saveProjectFilter(data, project, a);
      expect(loadLastProject(data)).toBe(project);
    });

    it("a malformed entry reads as no filter", () => {
      const data = tmp();
      const settings = path.join(data, "settings.json");
      writeFileSync(settings, JSON.stringify({ projects: { x: { include: "smoke" } } }));
      expect(loadProjectFilter(data, "x")).toEqual(NO_FILTER);
      writeFileSync(settings, "{not json");
      expect(loadProjectFilter(data, "x")).toEqual(NO_FILTER);
    });

    it("saving over a corrupt file starts fresh", () => {
      const data = tmp();
      writeFileSync(path.join(data, "settings.json"), "{not json");
      saveProjectFilter(data, "x", a);
      expect(loadProjectFilter(data, "x")).toEqual(a);
    });

    it("the returned no-filter is a copy", () => {
      const f = loadProjectFilter(tmp(), "x");
      f.include.push("smoke");
      expect(NO_FILTER.include).toEqual([]);
      expect(loadProjectFilter(tmp(), "x").include).toEqual([]);
    });
  });
});
