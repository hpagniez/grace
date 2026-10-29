import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { APP_FILES, preflight } from "./preflight";

const roomy = async () => 1e9;

let parent: string;
let install: string;

beforeEach(() => {
  parent = mkdtempSync(path.join(tmpdir(), "grace-preflight-"));
  install = path.join(parent, "Grace");
  mkdirSync(path.join(install, "resources"), { recursive: true });
  writeFileSync(path.join(install, "Grace.exe"), "");
  writeFileSync(
    path.join(install, "resources", APP_FILES),
    JSON.stringify(["Grace.exe", "resources", "locales"]),
  );
  mkdirSync(path.join(install, "locales"));
});

afterEach(() => {
  rmSync(parent, { recursive: true, force: true });
});

describe("preflight", () => {
  it("passes a folder that holds only Grace's files", async () => {
    expect(await preflight(install, 100, roomy)).toBeNull();
  });

  it("ignores desktop.ini and Thumbs.db", async () => {
    writeFileSync(path.join(install, "desktop.ini"), "");
    writeFileSync(path.join(install, "Thumbs.db"), "");
    expect(await preflight(install, 100, roomy)).toBeNull();
  });

  it("compares names without regard to case", async () => {
    writeFileSync(
      path.join(install, "resources", APP_FILES),
      JSON.stringify(["GRACE.EXE", "Resources", "LOCALES"]),
    );
    expect(await preflight(install, 100, roomy)).toBeNull();
  });

  it("refuses a folder shared with other files (R8.1)", async () => {
    writeFileSync(path.join(install, "taxes.xlsx"), "");
    expect(await preflight(install, 100, roomy)).toBe(
      "Grace's folder also holds other files (taxes.xlsx). Put Grace's files in a folder of their own to update it.",
    );
  });

  it("names at most 3 other files, sorted", async () => {
    for (const n of ["e.txt", "d.txt", "c.txt", "b.txt", "a.txt"]) {
      writeFileSync(path.join(install, n), "");
    }
    expect(await preflight(install, 100, roomy)).toBe(
      "Grace's folder also holds other files (a.txt, b.txt, c.txt). Put Grace's files in a folder of their own to update it.",
    );
  });

  it("refuses without a file list, naming the first 3 entries", async () => {
    rmSync(path.join(install, "resources", APP_FILES));
    expect(await preflight(install, 100, roomy)).toBe(
      "Grace's folder also holds other files (Grace.exe, locales, resources). Put Grace's files in a folder of their own to update it.",
    );
  });

  it("refuses an unreadable file list", async () => {
    writeFileSync(path.join(install, "resources", APP_FILES), "not json");
    expect(await preflight(install, 100, roomy)).toMatch(/^Grace's folder also holds other files/);
  });

  describe("folders next to the install that an update uses (R8.2, R10.3, R10.6)", () => {
    const inTheWay = (dir: string) =>
      `Grace can't update itself while ${dir} exists: it is not a folder Grace made. Move or rename it, then try again.`;

    it("refuses a .old or .failed that is not a Grace folder, so it is never deleted", async () => {
      for (const suffix of [".old", ".failed"]) {
        const dir = install + suffix;
        mkdirSync(dir);
        writeFileSync(path.join(dir, "mine.txt"), "");
        expect(await preflight(install, 100, roomy)).toBe(inTheWay(dir));
        rmSync(dir, { recursive: true });
        writeFileSync(dir, "a file of that name");
        expect(await preflight(install, 100, roomy)).toBe(inTheWay(dir));
        rmSync(dir);
      }
    });

    it("refuses a .update that holds anything Grace did not put there", async () => {
      const dir = `${install}.update`;
      mkdirSync(dir);
      writeFileSync(path.join(dir, "notes.txt"), "");
      expect(await preflight(install, 100, roomy)).toBe(inTheWay(dir));
    });

    it("accepts what a switch or a download leaves", async () => {
      for (const dir of [`${install}.old`, `${install}.failed`]) {
        mkdirSync(path.join(dir, "resources"), { recursive: true });
        writeFileSync(path.join(dir, "resources", APP_FILES), "[]");
      }
      const staging = `${install}.update`;
      // old and failed: moved in by the startup cleanup or the script before they are removed
      for (const d of ["app", "old", "failed"])
        mkdirSync(path.join(staging, d), { recursive: true });
      for (const f of [
        "plan.json",
        "switch.ps1",
        "switching",
        "started",
        "Grace-win32-x64-1.0.0-rc.6.zip",
      ])
        writeFileSync(path.join(staging, f), "");
      expect(await preflight(install, 100, roomy)).toBeNull();
    });
  });

  describe("when the parent is read-only", () => {
    const user = process.env.USERNAME ?? "";
    // `W` and `AD` deny adding files and folders; the install folder keeps its own rights
    const icacls = (...args: string[]) => execFileSync("icacls", args, { stdio: "ignore" });
    afterEach(() => icacls(parent, "/remove:d", user));

    it("refuses a parent it cannot write to", async () => {
      icacls(parent, "/deny", `${user}:(W,AD)`);
      const message = await preflight(install, 100, roomy);
      expect(message).toBe(
        `Grace can't update itself in ${parent}. Move the Grace folder somewhere you can write to, such as your user folder, or download the new version from the release page.`,
      );
    });
  });

  it("asks for 4 times the zip size", async () => {
    expect(await preflight(install, 100, async () => 399)).toBe(
      `Not enough free space on ${path.parse(install).root[0]}: Grace needs 1 MB to update.`,
    );
    expect(await preflight(install, 100, async () => 400)).toBeNull();
  });

  it("rounds the space up to whole MB", async () => {
    expect(await preflight(install, 218_000_000, async () => 0)).toMatch(
      /needs 872 MB to update\.$/,
    );
  });

  it("measures the real free space by default", async () => {
    expect(await preflight(install, 100)).toBeNull();
  });
});
