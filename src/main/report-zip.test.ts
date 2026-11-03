import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { zipReport } from "./report-zip";
import { TAR } from "./updates/fetchZip";

const dirs: string[] = [];
const temp = () => {
  const d = mkdtempSync(path.join(tmpdir(), "kk-zip-"));
  dirs.push(d);
  return d;
};
afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true });
});

/** A run folder: Karate's report, its JUnit XML and event file, and Grace's run.json. */
function runFolder(): string {
  const dir = temp();
  const files = {
    "karate-summary.html": "<html>",
    "feature-html/café.html": "<html>",
    "junit-xml/a.xml": "<testsuite/>",
    "karate-json/karate-events.jsonl": "{}",
    "run.json": "{}",
  };
  for (const [f, text] of Object.entries(files)) {
    mkdirSync(path.dirname(path.join(dir, f)), { recursive: true });
    writeFileSync(path.join(dir, f), text);
  }
  return dir;
}

/** The zip's files, unpacked by Windows' tar, as sorted paths with `/`. */
function unpacked(zip: string): string[] {
  const dir = temp();
  execFileSync(TAR, ["-xf", zip, "-C", dir], { windowsHide: true });
  return readdirSync(dir, { recursive: true, withFileTypes: true })
    .filter((e) => e.isFile())
    .map((e) => path.relative(dir, path.join(e.parentPath, e.name)).replaceAll("\\", "/"))
    .sort();
}

const REPORT = ["feature-html/café.html", "junit-xml/a.xml", "karate-summary.html"];

describe("zipReport", () => {
  it("holds the report and leaves out run.json and karate-json", async () => {
    const out = path.join(temp(), "out dir é");
    mkdirSync(out);
    const zip = path.join(out, "r é.zip");
    await zipReport(runFolder(), zip);
    expect(unpacked(zip)).toEqual(REPORT);
  });

  it("writes a zip whatever the name", async () => {
    const zip = path.join(temp(), "r.bin");
    await zipReport(runFolder(), zip);
    expect(unpacked(zip)).toEqual(REPORT);
  });

  it("replaces an existing file", async () => {
    const zip = path.join(temp(), "r.zip");
    writeFileSync(zip, "junk");
    await zipReport(runFolder(), zip);
    expect(unpacked(zip)).toEqual(REPORT);
  });

  it("leaves nothing when the folder is gone", async () => {
    const zip = path.join(temp(), "r.zip");
    await expect(zipReport(path.join(temp(), "missing"), zip)).rejects.toThrow(/\S/);
    expect(existsSync(zip)).toBe(false);
  });
});
