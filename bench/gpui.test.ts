import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { isGpui, parseProbe, readiness, seedWatch } from "./gpui.mjs";

describe("parseProbe", () => {
  it("reads the line crates/grace/src/probe.rs prints", () => {
    expect(parseProbe("PROBE first_frame 1762640000123")).toEqual({
      name: "first_frame",
      ms: 1762640000123,
    });
    expect(parseProbe("PROBE first_frame 1762640000123\r")).toEqual({
      name: "first_frame",
      ms: 1762640000123,
    });
  });

  it("ignores any other line", () => {
    for (const line of [
      "",
      "PROBE",
      "PROBE first_frame",
      "PROBE first_frame soon",
      "[WARN] PROBE x 1",
    ])
      expect(parseProbe(line), line).toBeNull();
  });
});

describe("isGpui", () => {
  let dir = "";
  afterEach(() => rmSync(dir, { recursive: true, force: true }));

  it("tells a GPUI build from an Electron one by its resources/app.asar", () => {
    dir = mkdtempSync(path.join(tmpdir(), "grace-gpui-test-"));
    const exe = path.join(dir, "Grace.exe");
    writeFileSync(exe, "");
    expect(isGpui(exe)).toBe(true);
    mkdirSync(path.join(dir, "resources"));
    writeFileSync(path.join(dir, "resources", "app.asar"), "");
    expect(isGpui(exe)).toBe(false);
  });
});

describe("a start with watch mode on", () => {
  let dir = "";
  afterEach(() => rmSync(dir, { recursive: true, force: true }));

  it("seeds the data folder with the project open and watch on, as grace-core keys it", () => {
    dir = mkdtempSync(path.join(tmpdir(), "grace-gpui-test-"));
    const project = path.join(dir, "tests-2000");
    seedWatch(dir, project);
    expect(JSON.parse(readFileSync(path.join(dir, "settings.json"), "utf8"))).toEqual({
      lastProject: project,
      projects: { [project]: { watch: true } },
    });
  });

  it("is ready only once the first frame is drawn and watch mode is on", () => {
    const ready = readiness(true);
    expect(ready({ name: "first_frame", ms: 5 })).toBeNull();
    expect(ready({ name: "watch_on", ms: 9 })).toBe(5);
  });

  it("without watch mode is ready at the first frame", () => {
    const ready = readiness(false);
    expect(ready({ name: "other", ms: 1 })).toBeNull();
    expect(ready({ name: "first_frame", ms: 5 })).toBe(5);
  });
});
