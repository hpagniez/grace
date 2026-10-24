import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { loadLastProject, saveLastProject } from "./settings";

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
});
