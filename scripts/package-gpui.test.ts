import { mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { APP_FILES as UPDATER_APP_FILES, zipName as updaterZipName } from "../src/shared/updates";
import { APP_FILES, LICENSES, layout, zipName } from "./package-gpui.mjs";

const repo = path.resolve(import.meta.dirname, "..");
let dir = "";
let vendor = "";

function file(at: string, text = "x") {
  mkdirSync(path.dirname(at), { recursive: true });
  writeFileSync(at, text);
}

beforeEach(() => {
  dir = mkdtempSync(path.join(tmpdir(), "grace-package-gpui-"));
  vendor = path.join(dir, "vendor");
  file(path.join(vendor, "jre", "bin", "java.exe"));
  file(path.join(vendor, "karate.jar"));
  file(path.join(vendor, "listener.jar"));
  file(path.join(vendor, "licenses", "karate-LICENSE.txt"));
  file(path.join(dir, "Grace-built.exe"), "the app");
  file(path.join(dir, "lucide"), "ISC");
});
afterEach(() => rmSync(dir, { recursive: true, force: true }));

const opts = () => ({
  exe: path.join(dir, "Grace-built.exe"),
  vendor,
  licenses: { "lucide-LICENSE.txt": path.join(dir, "lucide") },
  out: path.join(dir, "out"),
});

describe("layout", () => {
  it("lays out the folder as Electron Grace's zip: Grace.exe at the root, the rest in resources", () => {
    layout(opts());
    const out = path.join(dir, "out");
    expect(readdirSync(out).sort()).toEqual(["Grace.exe", "resources"]);
    expect(readFileSync(path.join(out, "Grace.exe"), "utf8")).toBe("the app");
    expect(readdirSync(path.join(out, "resources")).sort()).toEqual(
      [APP_FILES, "jre", "karate.jar", "licenses", "listener.jar"].sort(),
    );
    expect(readdirSync(path.join(out, "resources", "jre", "bin"))).toEqual(["java.exe"]);
    expect(readdirSync(path.join(out, "resources", "licenses")).sort()).toEqual([
      "karate-LICENSE.txt",
      "lucide-LICENSE.txt",
    ]);
  });

  it("lists the folder's top-level names in app-files.json, which the updater checks", () => {
    layout(opts());
    const listed = JSON.parse(readFileSync(path.join(dir, "out", "resources", APP_FILES), "utf8"));
    expect(listed.sort()).toEqual(["Grace.exe", "resources"]);
  });

  it("empties a folder laid out before", () => {
    file(path.join(dir, "out", "old.dll"));
    layout(opts());
    expect(readdirSync(path.join(dir, "out")).sort()).toEqual(["Grace.exe", "resources"]);
  });

  it("refuses without the vendored files, and says how to get them", () => {
    rmSync(path.join(vendor, "listener.jar"));
    expect(() => layout(opts())).toThrow(/listener\.jar is missing: run npm run vendor/);
  });
});

describe("the updater's contract (ADR 0019)", () => {
  it("names the list and the zip as the updater does", () => {
    expect(APP_FILES).toBe(UPDATER_APP_FILES);
    expect(zipName("1.0.0-rc.10")).toBe(updaterZipName("1.0.0-rc.10"));
  });

  it("ships the licences Electron Grace ships", () => {
    const forge = readFileSync(path.join(repo, "forge.config.mts"), "utf8");
    const block = /NPM_LICENSES = \{([^}]*)\}/.exec(forge)?.[1] ?? "";
    const named = [...block.matchAll(/"([^"]+)": "([^"]+)"/g)].map((m) => [m[1], m[2]]);
    expect(named.length).toBeGreaterThan(0);
    expect(Object.entries(LICENSES)).toEqual(named);
  });
});
