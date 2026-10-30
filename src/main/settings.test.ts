import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import { NO_FILTER, type FilterState } from "../shared/tags";
import { defaultUpdateSettings } from "../shared/updates";
import {
  emptyEnvSettings,
  forgetLastProject,
  loadAppearance,
  loadLastProject,
  loadProjectEnv,
  loadProjectFilter,
  loadProjectThreads,
  loadRecentProjects,
  loadUpdateSettings,
  loadWindowState,
  RECENT_LIMIT,
  removeRecentProject,
  saveAppearance,
  saveOpenedProject,
  saveProjectEnv,
  saveProjectFilter,
  saveProjectThreads,
  saveUpdateSettings,
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
    saveOpenedProject(data, project);
    expect(loadLastProject(data)).toBe(project);
  });

  it("loadLastProject ignores a missing folder", () => {
    const data = tmp();
    const project = path.join(tmp(), "gone");
    mkdirSync(project);
    saveOpenedProject(data, project);
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

  describe("recent projects", () => {
    const writeSettings = (data: string, settings: unknown) =>
      writeFileSync(
        path.join(data, "settings.json"),
        typeof settings === "string" ? settings : JSON.stringify(settings),
      );
    const readSettings = (data: string) =>
      JSON.parse(readFileSync(path.join(data, "settings.json"), "utf8"));

    it("saveOpenedProject puts the folder first and saves lastProject", () => {
      const data = tmp();
      const a = path.join(tmp(), "a");
      const b = path.join(tmp(), "b");
      mkdirSync(a);
      mkdirSync(b);
      saveOpenedProject(data, a);
      saveOpenedProject(data, b);
      saveOpenedProject(data, a);
      expect(loadRecentProjects(data)).toEqual([a, b]);
      expect(loadLastProject(data)).toBe(a);
    });

    it("keeps the 10 newest folders", () => {
      const data = tmp();
      for (let i = 0; i <= 10; i++) saveOpenedProject(data, `D:\\p${i}`);
      expect(RECENT_LIMIT).toBe(10);
      expect(loadRecentProjects(data)).toEqual(
        Array.from({ length: 10 }, (_, i) => `D:\\p${10 - i}`),
      );
    });

    it("same folder with a trailing separator or another case is one entry", () => {
      const data = tmp();
      writeSettings(data, { recentProjects: ["D:\\work\\api", "d:\\WORK\\API\\"] });
      expect(loadRecentProjects(data)).toEqual(["D:\\work\\api"]);
      saveOpenedProject(data, "d:\\WORK\\api");
      expect(loadRecentProjects(data)).toEqual(["d:\\WORK\\api"]);
    });

    it("a broken list reads as what can be kept", () => {
      const data = tmp();
      const v = Array.from({ length: 12 }, (_, i) => `D:\\v${i}`);
      writeSettings(data, { recentProjects: [42, "rel\\path", "D:\\a", "d:\\A", ...v] });
      expect(loadRecentProjects(data)).toEqual(["D:\\a", ...v.slice(0, 9)]);
      writeSettings(data, { recentProjects: "x" });
      expect(loadRecentProjects(data)).toEqual([]);
      writeSettings(data, { lastProject: "D:\\a" });
      expect(loadRecentProjects(data)).toEqual([]);
      writeSettings(data, "{not json");
      expect(loadRecentProjects(data)).toEqual([]);
      expect(loadRecentProjects(tmp())).toEqual([]);
      readFails.code = "EBUSY";
      try {
        expect(loadRecentProjects(data)).toEqual([]);
      } finally {
        readFails.code = null;
      }
    });

    it("removeRecentProject drops the row and a matching lastProject", () => {
      const data = tmp();
      writeSettings(data, {
        lastProject: "D:\\gone",
        recentProjects: ["D:\\gone", "D:\\b"],
        projects: { "D:\\gone": { threads: 2 } },
      });
      removeRecentProject(data, "D:\\gone");
      const saved = readSettings(data);
      expect(Object.hasOwn(saved, "lastProject")).toBe(false);
      expect(saved.recentProjects).toEqual(["D:\\b"]);
      expect(saved.projects).toEqual({ "D:\\gone": { threads: 2 } });
    });

    it("removeRecentProject keeps a lastProject that is another folder", () => {
      const data = tmp();
      writeSettings(data, { lastProject: "D:\\b", recentProjects: ["D:\\gone", "D:\\b"] });
      removeRecentProject(data, "D:\\gone");
      expect(readSettings(data)).toEqual({ lastProject: "D:\\b", recentProjects: ["D:\\b"] });
    });

    it("forgetLastProject keeps the other keys", () => {
      const data = tmp();
      const saved = {
        recentProjects: ["D:\\a"],
        window: { x: 1, y: 2, width: 700, height: 500, maximized: false },
        projects: { "D:\\a": { threads: 2 } },
      };
      writeSettings(data, { lastProject: "D:\\a", ...saved });
      forgetLastProject(data);
      expect(readSettings(data)).toEqual(saved);
    });
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

    it("saveOpenedProject keeps saved filters", () => {
      const data = tmp();
      const project = path.join(tmp(), "proj");
      mkdirSync(project);
      saveProjectFilter(data, project, a);
      saveOpenedProject(data, project);
      expect(loadProjectFilter(data, project)).toEqual(a);
      expect(loadLastProject(data)).toBe(project);
    });

    it("saveProjectFilter keeps lastProject", () => {
      const data = tmp();
      const project = path.join(tmp(), "proj");
      mkdirSync(project);
      saveOpenedProject(data, project);
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
        expect(() => saveOpenedProject(data, "q")).toThrow("EBUSY");
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

  describe("thread count", () => {
    it("save then load returns the count", () => {
      const data = tmp();
      saveProjectThreads(data, "x", 8);
      expect(loadProjectThreads(data, "x")).toBe(8);
    });

    it("each project has its own count", () => {
      const data = tmp();
      saveProjectThreads(data, "a", 3);
      expect(loadProjectThreads(data, "a")).toBe(3);
      expect(loadProjectThreads(data, "b")).toBe(1);
    });

    it("saving the count keeps the other settings", () => {
      const data = tmp();
      const project = path.join(tmp(), "proj");
      mkdirSync(project);
      const f: FilterState = { include: ["smoke"], exclude: [], hideNonMatching: false };
      const w: WindowState = { x: 1, y: 2, width: 700, height: 500, maximized: false };
      const e: EnvSettings = {
        env: "qa",
        properties: { shared: [{ name: "timeout", value: "5000" }], byEnv: {} },
      };
      saveOpenedProject(data, project);
      saveWindowState(data, w);
      saveProjectFilter(data, project, f);
      saveProjectEnv(data, project, e);
      saveProjectThreads(data, project, 4);
      expect(loadLastProject(data)).toBe(project);
      expect(loadWindowState(data)).toEqual(w);
      expect(loadProjectFilter(data, project)).toEqual(f);
      expect(loadProjectEnv(data, project)).toEqual(e);
      expect(loadProjectThreads(data, project)).toBe(4);
    });

    it.each(["8", 0, 2.5, 64, null])("a malformed saved count %j loads as 1", (threads) => {
      const data = tmp();
      writeFileSync(
        path.join(data, "settings.json"),
        JSON.stringify({ projects: { x: { threads } } }),
      );
      expect(loadProjectThreads(data, "x")).toBe(1);
    });

    it("an unreadable file loads as 1", () => {
      const data = tmp();
      saveProjectThreads(data, "x", 8);
      readFails.code = "EBUSY";
      try {
        expect(loadProjectThreads(data, "x")).toBe(1);
      } finally {
        readFails.code = null;
      }
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
      saveOpenedProject(data, project);
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
      saveOpenedProject(data, project);
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
      saveOpenedProject(data, project);
      writeFileSync(path.join(data, "settings.json.tmp"), "{junk");
      expect(loadLastProject(data)).toBe(project);
      saveWindowState(data, ok);
      expect(loadLastProject(data)).toBe(project);
      expect(loadWindowState(data)).toEqual(ok);
    });
  });

  describe("update settings", () => {
    const write = (data: string, settings: unknown) =>
      writeFileSync(path.join(data, "settings.json"), JSON.stringify(settings));
    const readJson = (data: string) =>
      JSON.parse(readFileSync(path.join(data, "settings.json"), "utf8"));

    it("reads missing or malformed update settings as defaults (R12)", () => {
      const dir = tmp();
      write(dir, {
        updates: {
          channel: "beta",
          checkAtStartup: "yes",
          autoUpdate: true,
          skipped: "v1",
          failed: 3,
          lastChecked: 5,
        },
      });
      expect(loadUpdateSettings(dir)).toEqual({
        channel: null,
        checkAtStartup: true,
        autoUpdate: true,
        skipped: null,
        failed: null,
        lastChecked: null,
      });
    });

    it("reads no file, a corrupt file or a non-object key as the defaults", () => {
      expect(loadUpdateSettings(tmp())).toEqual(defaultUpdateSettings());
      const corrupt = tmp();
      writeFileSync(path.join(corrupt, "settings.json"), "{not json");
      expect(loadUpdateSettings(corrupt)).toEqual(defaultUpdateSettings());
      for (const updates of [null, "x", 7, ["stable"]]) {
        const dir = tmp();
        write(dir, { updates });
        expect(loadUpdateSettings(dir), JSON.stringify(updates)).toEqual(defaultUpdateSettings());
      }
    });

    it("reads an unreadable file as the defaults and never throws", () => {
      const dir = tmp();
      write(dir, { updates: { autoUpdate: true } });
      readFails.code = "EBUSY";
      try {
        expect(loadUpdateSettings(dir)).toEqual(defaultUpdateSettings());
      } finally {
        readFails.code = null;
      }
    });

    it("keeps valid fields and defaults only the bad ones", () => {
      const dir = tmp();
      write(dir, {
        updates: {
          channel: "stable",
          checkAtStartup: false,
          autoUpdate: "no",
          skipped: "1.0.1",
          failed: "1.0.2-rc.1",
          lastChecked: "2026-10-28T10:00:00Z",
        },
      });
      expect(loadUpdateSettings(dir)).toEqual({
        channel: "stable",
        checkAtStartup: false,
        autoUpdate: false,
        skipped: "1.0.1",
        failed: "1.0.2-rc.1",
        lastChecked: "2026-10-28T10:00:00Z",
      });
    });

    it("reads a skipped or failed version that is not valid as null, so compareVersions never throws", () => {
      for (const bad of ["v1.0.1", "1.0", "1.0.0-beta.1", "01.0.0", "", " 1.0.0"]) {
        const dir = tmp();
        write(dir, { updates: { skipped: bad, failed: bad } });
        const s = loadUpdateSettings(dir);
        expect(s.skipped, bad).toBeNull();
        expect(s.failed, bad).toBeNull();
      }
    });

    it("reads a lastChecked that is not a date as null", () => {
      const dir = tmp();
      write(dir, { updates: { lastChecked: "yesterday" } });
      expect(loadUpdateSettings(dir).lastChecked).toBeNull();
    });

    it("reads no saved channel as not picked, and saves one only once picked (R1.3)", () => {
      const dir = tmp();
      expect(loadUpdateSettings(dir).channel).toBeNull();
      saveUpdateSettings(dir, { ...defaultUpdateSettings(), lastChecked: "2026-10-28T10:00:00Z" });
      expect(readJson(dir).updates).not.toHaveProperty("channel");
      expect(loadUpdateSettings(dir).channel).toBeNull();
      saveUpdateSettings(dir, { ...defaultUpdateSettings(), channel: "stable" });
      expect(readJson(dir).updates.channel).toBe("stable");
      expect(loadUpdateSettings(dir).channel).toBe("stable");
    });

    it("saves update settings and keeps the other keys", () => {
      const dir = tmp();
      write(dir, { lastProject: "D:\\p" });
      saveUpdateSettings(dir, { ...defaultUpdateSettings(), skipped: "1.0.1" });
      expect(readJson(dir).lastProject).toBe("D:\\p");
      expect(loadUpdateSettings(dir).skipped).toBe("1.0.1");
    });

    it("saving other settings keeps the update settings", () => {
      const dir = tmp();
      const s = { ...defaultUpdateSettings(), autoUpdate: true, failed: "1.0.3" };
      saveUpdateSettings(dir, s);
      saveWindowState(dir, { x: 0, y: 0, width: 700, height: 500, maximized: false });
      expect(loadUpdateSettings(dir)).toEqual(s);
    });
  });

  describe("appearance (spec R8)", () => {
    const write = (data: string, settings: unknown) =>
      writeFileSync(path.join(data, "settings.json"), JSON.stringify(settings));
    const readJson = (data: string) =>
      JSON.parse(readFileSync(path.join(data, "settings.json"), "utf8"));

    it("reads no file, a corrupt file or an unreadable one as Follow Windows", () => {
      expect(loadAppearance(tmp())).toBe("system");
      const corrupt = tmp();
      writeFileSync(path.join(corrupt, "settings.json"), "{not json");
      expect(loadAppearance(corrupt)).toBe("system");
      const busy = tmp();
      write(busy, { appearance: { theme: "dark" } });
      readFails.code = "EBUSY";
      try {
        expect(loadAppearance(busy)).toBe("system");
      } finally {
        readFails.code = null;
      }
    });

    it("reads a theme that is not one of the three, or a key that is not an object, as Follow Windows", () => {
      for (const appearance of [{ theme: "purple" }, { theme: 3 }, {}, null, "dark", ["dark"]]) {
        const dir = tmp();
        write(dir, { appearance });
        expect(loadAppearance(dir), JSON.stringify(appearance)).toBe("system");
      }
    });

    it("saves a theme, keeps the other keys and loads it back", () => {
      const dir = tmp();
      write(dir, { lastProject: "D:\\p" });
      saveAppearance(dir, "dark");
      expect(readJson(dir)).toEqual({ lastProject: "D:\\p", appearance: { theme: "dark" } });
      expect(loadAppearance(dir)).toBe("dark");
      saveAppearance(dir, "light");
      expect(loadAppearance(dir)).toBe("light");
      saveWindowState(dir, { x: 0, y: 0, width: 700, height: 500, maximized: false });
      expect(loadAppearance(dir)).toBe("light");
    });
  });
});
