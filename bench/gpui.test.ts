import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { isGpui, parseProbe } from "./gpui.mjs";

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
