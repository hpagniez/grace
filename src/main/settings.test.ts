import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import { NO_FILTER, type FilterState } from "../shared/tags";
import {
  emptyEnvSettings,
  loadLastProject,
  loadProjectEnv,
  loadProjectFilter,
  loadWindowState,
  saveLastProject,
  saveProjectEnv,
  saveProjectFilter,
  saveWindowState,
  skippedProperties,
  type EnvSettings,
  type WindowState,
} from "./settings";

// Set to an error code to make every readFileSync (or renameSync) fail with it, like a file another program holds.
const readFails = vi.hoisted(() => ({ code: null as string | null }));
const renameFails = vi.hoisted(() => ({ code: null as string | null }));
vi.mock("node:fs", async (importOriginal) => {
  const fs = await importOriginal<typeof import("node:fs")>();
  const readFileSync = ((...args: Parameters<typeof fs.readFileSync>) => {
    if (readFails.code) throw Object.assign(new Error(readFails.code), { code: readFails.code });
    return fs.readFileSync(...args);
  }) as typeof fs.readFileSync;
  const renameSync = ((...args: Parameters<typeof fs.renameSync>) => {
    if (renameFails.code)
      throw Object.assign(new Error(renameFails.code), { code: renameFails.code });
    return fs.renameSync(...args);
  }) as typeof fs.renameSync;
  return { ...fs, readFileSync, renameSync };
});

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

    it("a file that cannot be read fails the save and stays as it was", () => {
      const data = tmp();
      const settings = path.join(data, "settings.json");
      const before = JSON.stringify({ lastProject: "p", projects: { x: a } });
      writeFileSync(settings, before);
      readFails.code = "EBUSY";
      try {
        expect(() => saveProjectFilter(data, "y", b)).toThrow("EBUSY");
        expect(() => saveLastProject(data, "q")).toThrow("EBUSY");
        expect(loadProjectFilter(data, "x")).toEqual(NO_FILTER);
        expect(loadLastProject(data)).toBeNull();
      } finally {
        readFails.code = null;
      }
      expect(readFileSync(settings, "utf8")).toBe(before);
    });

    it("the returned no-filter is a copy", () => {
      const f = loadProjectFilter(tmp(), "x");
      f.include.push("smoke");
      expect(NO_FILTER.include).toEqual([]);
      expect(loadProjectFilter(tmp(), "x").include).toEqual([]);
    });
  });

  describe("environment and run properties", () => {
    const filter: FilterState = { include: ["smoke"], exclude: ["wip"], hideNonMatching: true };
    const env: EnvSettings = {
      env: "qa",
      properties: {
        shared: [{ name: "timeout", value: "5000" }],
        byEnv: {
          qa: [
            { name: "timeout", value: "9000" },
            { name: "api.token", secret: "djEwYmFzZTY0" },
          ],
        },
      },
    };
    const writeEntry = (data: string, entry: unknown, root = "x") =>
      writeFileSync(
        path.join(data, "settings.json"),
        JSON.stringify({ projects: { [root]: entry } }),
      );

    it("env settings round-trip", () => {
      const data = tmp();
      saveProjectEnv(data, "proj-one", env);
      expect(loadProjectEnv(data, "proj-one")).toEqual(env);
      expect(loadProjectEnv(data, "proj-two")).toEqual(emptyEnvSettings());
    });

    it("saving the filter keeps env and properties", () => {
      const data = tmp();
      saveProjectEnv(data, "x", env);
      saveProjectFilter(data, "x", filter);
      expect(loadProjectEnv(data, "x")).toEqual(env);
      expect(loadProjectFilter(data, "x")).toEqual(filter);
    });

    it("saving env keeps the filter", () => {
      const data = tmp();
      saveProjectFilter(data, "x", filter);
      saveProjectEnv(data, "x", env);
      expect(loadProjectFilter(data, "x")).toEqual(filter);
      expect(loadProjectEnv(data, "x")).toEqual(env);
    });

    it("an entry from the tag-filter version loads as no environment", () => {
      const data = tmp();
      writeEntry(data, filter);
      expect(loadProjectEnv(data, "x")).toEqual(emptyEnvSettings());
    });

    it("malformed rows are dropped, good rows kept", () => {
      const data = tmp();
      const good = { name: "good", value: "1" };
      writeEntry(data, {
        env: "qa",
        properties: {
          shared: [
            good,
            { name: "both", value: "1", secret: "abc" },
            { name: "karate.env", value: "dev" },
            { name: "num", value: 5 },
            { name: "empty", secret: "" },
            { name: "multi", value: "a\nb" },
            { name: "wide", value: "café" },
            { value: "no name" },
            "text",
            null,
            { name: "secret", secret: "abc" },
          ],
          byEnv: {
            qa: [good],
            dev: "not an array",
            "bad name!": [good],
            nothing: [{ name: 1 }],
          },
        },
      });
      expect(loadProjectEnv(data, "x")).toEqual({
        env: "qa",
        properties: {
          shared: [good, { name: "secret", secret: "abc" }],
          byEnv: { qa: [good], nothing: [] },
        },
      });
      // only the rows of lists that load are named
      expect(skippedProperties(data, "x")).toEqual([
        "All environments: both",
        "All environments: karate.env",
        "All environments: num",
        "All environments: empty",
        "All environments: multi",
        "All environments: wide",
        "All environments: (no name)",
        "All environments: (no name)",
        "All environments: (no name)",
        "nothing: (no name)",
      ]);
    });

    it("rejected rows are named, and a save keeps them while their list exists", () => {
      const data = tmp();
      const settings = path.join(data, "settings.json");
      const good = { name: "good", value: "1" };
      const reserved = { name: "java.home", value: "C:/jdk" };
      const wide = { name: "greeting", secret: "abc", value: "h\u00e9llo" };
      writeEntry(data, {
        include: ["smoke"],
        properties: {
          shared: [good, reserved],
          byEnv: { qa: [wide, good], dev: [{ value: "x" }] },
        },
      });
      expect(loadProjectEnv(data, "x").properties).toEqual({
        shared: [good],
        byEnv: { qa: [good], dev: [] },
      });
      expect(skippedProperties(data, "x")).toEqual([
        "All environments: java.home",
        "qa: greeting",
        "dev: (no name)",
      ]);
      // after the new rows of each list still there; the rows of the deleted dev list go with it
      const fresh = { name: "fresh", value: "2" };
      saveProjectEnv(data, "x", {
        env: "qa",
        properties: { shared: [fresh], byEnv: { qa: [good], prod: [fresh] } },
      });
      const entry = JSON.parse(readFileSync(settings, "utf8")).projects.x;
      expect(entry).toEqual({
        include: ["smoke"],
        env: "qa",
        properties: {
          shared: [fresh, reserved],
          byEnv: { qa: [good, wide], prod: [fresh] },
        },
      });
      expect(skippedProperties(data, "x")).toEqual(["All environments: java.home", "qa: greeting"]);
      // a save never copies a valid row of the file
      saveProjectEnv(data, "x", emptyEnvSettings());
      expect(JSON.parse(readFileSync(settings, "utf8")).projects.x.properties).toEqual({
        shared: [reserved],
        byEnv: {},
      });
    });

    it("skippedProperties never throws", () => {
      const data = tmp();
      expect(skippedProperties(data, "x")).toEqual([]);
      writeEntry(data, { properties: { shared: [{ name: "java.home", value: "x" }] } });
      readFails.code = "EBUSY";
      try {
        expect(skippedProperties(data, "x")).toEqual([]);
      } finally {
        readFails.code = null;
      }
      writeFileSync(path.join(data, "settings.json"), "{not json");
      expect(skippedProperties(data, "x")).toEqual([]);
    });

    it("a bad env, bad lists and unsafe keys read as missing", () => {
      const data = tmp();
      const settings = path.join(data, "settings.json");
      writeEntry(data, { env: "not valid!", properties: { shared: "x", byEnv: [] } });
      expect(loadProjectEnv(data, "x")).toEqual(emptyEnvSettings());
      writeEntry(data, { env: 5, properties: "x" });
      expect(loadProjectEnv(data, "x")).toEqual(emptyEnvSettings());
      writeEntry(data, "text");
      expect(loadProjectEnv(data, "x")).toEqual(emptyEnvSettings());
      writeFileSync(
        settings,
        '{"projects":{"x":{"properties":{"byEnv":{"__proto__":[{"name":"a","value":"1"}]}}}}}',
      );
      const loaded = loadProjectEnv(data, "x");
      expect(Object.keys(loaded.properties.byEnv)).toEqual([]);
      expect(Object.getPrototypeOf(loaded.properties.byEnv)).toBe(Object.prototype);
      writeFileSync(settings, "{not json");
      expect(loadProjectEnv(data, "x")).toEqual(emptyEnvSettings());
    });

    it("a sparse row list loses only its holes", () => {
      const data = tmp();
      writeFileSync(
        path.join(data, "settings.json"),
        '{"projects":{"x":{"properties":{"shared":[{"name":"a","value":"1"},null,{"name":"b","value":"2"}]}}}}',
      );
      expect(loadProjectEnv(data, "x").properties.shared.map((r) => r.name)).toEqual(["a", "b"]);
    });

    it("env null is omitted from the file", () => {
      const data = tmp();
      saveProjectEnv(data, "x", env);
      saveProjectEnv(data, "x", { ...env, env: null });
      const entry = JSON.parse(readFileSync(path.join(data, "settings.json"), "utf8")).projects.x;
      expect(Object.hasOwn(entry, "env")).toBe(false);
      expect(entry.properties).toEqual(env.properties);
      expect(loadProjectEnv(data, "x").env).toBeNull();
    });

    it("loadProjectEnv does not throw when the file cannot be read", () => {
      const data = tmp();
      saveProjectEnv(data, "x", env);
      readFails.code = "EBUSY";
      try {
        expect(loadProjectEnv(data, "x")).toEqual(emptyEnvSettings());
        expect(() => saveProjectEnv(data, "x", env)).toThrow("EBUSY");
      } finally {
        readFails.code = null;
      }
    });

    it("the empty settings are a fresh object each call", () => {
      const e = emptyEnvSettings();
      e.properties.shared.push({ name: "a", value: "1" });
      e.properties.byEnv.qa = [];
      expect(emptyEnvSettings()).toEqual({ env: null, properties: { shared: [], byEnv: {} } });
    });
  });

  describe("window state", () => {
    const ok: WindowState = { x: -1900, y: 20, width: 700, height: 500, maximized: true };
    const writeSettings = (data: string, text: string) =>
      writeFileSync(path.join(data, "settings.json"), text);

    it("save then load returns the window state", () => {
      const data = tmp();
      saveWindowState(data, ok);
      expect(loadWindowState(data)).toEqual(ok);
    });

    it("loadWindowState reads a broken state as nothing saved", () => {
      expect(loadWindowState(tmp())).toBeNull();
      const { maximized: _, ...noMaximized } = ok;
      for (const window of [
        "abc",
        { ...ok, width: -5 },
        { ...ok, width: 0 },
        { ...ok, x: "a" },
        noMaximized,
      ]) {
        const data = tmp();
        writeSettings(data, JSON.stringify({ window }));
        expect(loadWindowState(data), JSON.stringify(window)).toBeNull();
      }
      const data = tmp();
      writeSettings(
        data,
        '{"window":{"x":1e999,"y":0,"width":700,"height":500,"maximized":false}}',
      );
      expect(loadWindowState(data)).toBeNull();
    });

    it("saving the window keeps the other settings", () => {
      const data = tmp();
      const project = path.join(tmp(), "proj");
      mkdirSync(project);
      const f: FilterState = { include: ["smoke"], exclude: [], hideNonMatching: false };
      saveLastProject(data, project);
      saveProjectFilter(data, project, f);
      saveWindowState(data, ok);
      expect(loadLastProject(data)).toBe(project);
      expect(loadProjectFilter(data, project)).toEqual(f);
      expect(loadWindowState(data)).toEqual(ok);
    });

    it("a failed rename leaves the file as it was", () => {
      const data = tmp();
      const project = path.join(tmp(), "proj");
      mkdirSync(project);
      saveLastProject(data, project);
      const settings = path.join(data, "settings.json");
      const before = readFileSync(settings, "utf8");
      renameFails.code = "EPERM";
      try {
        expect(() => saveWindowState(data, ok)).toThrow("EPERM");
      } finally {
        renameFails.code = null;
      }
      expect(readFileSync(settings, "utf8")).toBe(before);
      saveWindowState(data, ok);
      expect(loadLastProject(data)).toBe(project);
      expect(loadWindowState(data)).toEqual(ok);
    });

    it("a leftover temp file is ignored", () => {
      const data = tmp();
      const project = path.join(tmp(), "proj");
      mkdirSync(project);
      saveLastProject(data, project);
      writeFileSync(path.join(data, "settings.json.tmp"), "{junk");
      expect(loadLastProject(data)).toBe(project);
      saveWindowState(data, ok);
      expect(loadLastProject(data)).toBe(project);
      expect(loadWindowState(data)).toEqual(ok);
    });
  });
});
