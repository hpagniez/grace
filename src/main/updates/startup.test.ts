import { spawn } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, rmSync, utimesSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { defaultUpdateSettings, type UpdateSettings } from "../../shared/updates";
import {
  listBackups,
  markStarted,
  removeLeftovers,
  resultNotice,
  settingsAfter,
  switchInProgress,
  takeResult,
  type SwitchResult,
} from "./startup";

const BOM = String.fromCharCode(0xfeff);
const carried: UpdateSettings = {
  channel: "prerelease",
  checkAtStartup: false,
  autoUpdate: true,
  skipped: "1.0.0-rc.4",
  failed: null,
  lastChecked: "2026-10-28T10:00:00.000Z",
};
const restored: UpdateSettings = {
  channel: "stable",
  checkAtStartup: true,
  autoUpdate: false,
  skipped: null,
  failed: null,
  lastChecked: null,
};
const ok = (from: string, to: string): SwitchResult => ({
  from,
  to,
  rollback: false,
  ok: true,
  reverted: false,
  error: null,
  updates: carried,
});
const failed = (from: string, to: string, error: string): SwitchResult => ({
  ...ok(from, to),
  ok: false,
  error,
});

/** The id of a process that has exited. */
async function deadPid(): Promise<number> {
  const child = spawn(process.execPath, ["-e", ""], { stdio: "ignore" });
  await new Promise((r) => child.once("exit", r));
  return child.pid!;
}

describe("results", () => {
  it("words the notices (R10.7)", () => {
    expect(resultNotice(ok("1.0.0-rc.5", "1.0.0-rc.6"), "1.0.0-rc.6")).toBe(
      "Updated to 1.0.0-rc.6.",
    );
    expect(resultNotice({ ...ok("1.0.0-rc.6", "1.0.0-rc.5"), rollback: true }, "1.0.0-rc.5")).toBe(
      "Rolled back to 1.0.0-rc.5.",
    );
    expect(resultNotice(failed("1.0.0-rc.5", "1.0.0-rc.6", "a file is in use"), "1.0.0-rc.5")).toBe(
      "Couldn't switch to 1.0.0-rc.6: a file is in use. You are still on 1.0.0-rc.5.",
    );
  });

  it("does not claim the old version when the new one stayed installed", () => {
    expect(resultNotice(failed("1.0.0-rc.5", "1.0.0-rc.6", "x"), "1.0.0-rc.6")).toBe(
      "Couldn't switch to 1.0.0-rc.6 cleanly: x. You are on 1.0.0-rc.6.",
    );
  });

  it("holds a rollback: carried settings, auto-update off, from skipped (R10.7)", () => {
    const s = settingsAfter(
      { ...ok("1.0.0-rc.6", "1.0.0-rc.5"), rollback: true, updates: carried },
      restored,
    );
    expect(s).toEqual({ ...carried, autoUpdate: false, skipped: "1.0.0-rc.6" });
  });

  it("carries the settings over an update that restored saved data", () => {
    expect(settingsAfter(ok("1.0.0-rc.5", "1.0.0-rc.6"), restored)).toEqual(carried);
  });

  it("keeps the current settings after a failure, and remembers a version that did not start (R10.6)", () => {
    const f = failed("1.0.0-rc.5", "1.0.0-rc.6", "x");
    expect(settingsAfter(f, restored)).toEqual(restored);
    expect(settingsAfter({ ...f, reverted: true }, restored)).toEqual({
      ...restored,
      failed: "1.0.0-rc.6",
    });
  });

  describe("takeResult", () => {
    let dir: string;
    const file = () => path.join(dir, "update-result.json");
    beforeEach(() => {
      dir = mkdtempSync(path.join(tmpdir(), "grace-result-"));
    });
    afterEach(() => rmSync(dir, { recursive: true, force: true }));

    it("reads a result with a BOM and deletes it", () => {
      writeFileSync(file(), BOM + JSON.stringify(ok("1.0.0-rc.5", "1.0.0-rc.6")));
      expect(takeResult(dir)).toEqual(ok("1.0.0-rc.5", "1.0.0-rc.6"));
      expect(existsSync(file())).toBe(false);
    });

    it("is null without a file", () => {
      expect(takeResult(dir)).toBeNull();
    });

    it.each([
      ["not JSON", "{oops"],
      ["not an object", "[1]"],
      ["a bad version", JSON.stringify({ ...ok("1.0.0-rc.5", "1.0.0-rc.6"), to: "1.0" })],
      ["a string for a boolean", JSON.stringify({ ...ok("1.0.0-rc.5", "1.0.0-rc.6"), ok: "yes" })],
      [
        "a missing field",
        JSON.stringify({ ...ok("1.0.0-rc.5", "1.0.0-rc.6"), reverted: undefined }),
      ],
      [
        "an error that is not text",
        JSON.stringify({ ...ok("1.0.0-rc.5", "1.0.0-rc.6"), error: 3 }),
      ],
    ])("is null and deleted when malformed: %s", (_name, text) => {
      writeFileSync(file(), text);
      expect(takeResult(dir)).toBeNull();
      expect(existsSync(file())).toBe(false);
    });

    it("reads the carried update settings with the shared rules, and none as null (R12)", () => {
      const r = ok("1.0.0-rc.5", "1.0.0-rc.6");
      writeFileSync(
        file(),
        JSON.stringify({ ...r, updates: { channel: "nightly", skipped: "soon" } }),
      );
      expect(takeResult(dir)?.updates).toEqual({
        ...defaultUpdateSettings("1.0.0-rc.5"), // the default channel follows `from`
        channel: "prerelease",
      });
      writeFileSync(file(), JSON.stringify({ ...r, updates: carried }));
      expect(takeResult(dir)?.updates).toEqual(carried);
      writeFileSync(file(), JSON.stringify({ ...r, updates: null }));
      expect(takeResult(dir)?.updates).toBeNull();
    });
  });
});

describe("a switch on disk", () => {
  let root: string;
  let install: string;
  let staging: string;
  const switching = () => path.join(staging, "switching");
  const plan = (to: string) =>
    writeFileSync(path.join(staging, "plan.json"), JSON.stringify({ to }));
  beforeEach(() => {
    root = mkdtempSync(path.join(tmpdir(), "grace-startup-"));
    install = path.join(root, "Grace");
    staging = `${install}.update`;
    mkdirSync(staging);
  });
  afterEach(() => rmSync(root, { recursive: true, force: true }));

  describe("switchInProgress", () => {
    it("sees a live switch and ignores a dead one (R10.9)", async () => {
      plan("1.0.0-rc.6");
      writeFileSync(switching(), String(process.pid));
      expect(switchInProgress(install, "1.0.0-rc.5")).toBe(true);
      writeFileSync(switching(), String(await deadPid()));
      expect(switchInProgress(install, "1.0.0-rc.5")).toBe(false);
    });

    it("never stops the version the plan installs", () => {
      plan("1.0.0-rc.6");
      writeFileSync(switching(), String(process.pid));
      expect(switchInProgress(install, "1.0.0-rc.6")).toBe(false);
    });

    it("counts a live switch whose plan cannot be read", () => {
      writeFileSync(switching(), String(process.pid));
      expect(switchInProgress(install, "1.0.0-rc.6")).toBe(true);
    });

    it("is false without a switching file or a staging folder", () => {
      plan("1.0.0-rc.6");
      expect(switchInProgress(install, "1.0.0-rc.5")).toBe(false);
      expect(switchInProgress(path.join(root, "Other"), "1.0.0-rc.5")).toBe(false);
    });

    it("trusts an empty file for 30 seconds: the script writes it non-atomically", () => {
      plan("1.0.0-rc.6");
      writeFileSync(switching(), "");
      expect(switchInProgress(install, "1.0.0-rc.5")).toBe(true);
      const old = new Date(Date.now() - 60_000);
      utimesSync(switching(), old, old);
      expect(switchInProgress(install, "1.0.0-rc.5")).toBe(false);
      writeFileSync(switching(), "not a pid");
      utimesSync(switching(), old, old);
      expect(switchInProgress(install, "1.0.0-rc.5")).toBe(false);
    });
  });

  describe("markStarted", () => {
    it("marks the start only for the version the plan installs", () => {
      plan("1.0.0-rc.6");
      markStarted(install, "1.0.0-rc.5");
      expect(existsSync(path.join(staging, "started"))).toBe(false);
      markStarted(install, "1.0.0-rc.6");
      expect(existsSync(path.join(staging, "started"))).toBe(true);
    });

    it("does nothing, and does not throw, without a plan or a staging folder", () => {
      markStarted(install, "1.0.0-rc.6");
      markStarted(path.join(root, "Other"), "1.0.0-rc.6");
      expect(existsSync(path.join(staging, "started"))).toBe(false);
    });
  });

  describe("removeLeftovers", () => {
    const leftovers = () => {
      for (const d of [`${install}.old`, `${install}.failed`]) mkdirSync(d);
      writeFileSync(path.join(staging, "Grace-win32-x64-1.0.0-rc.6.zip"), "partial");
    };
    const fast = { pollMs: 10, timeoutMs: 200 };

    it("removes a partial download left by a killed Grace (R10.8)", async () => {
      leftovers();
      await removeLeftovers(install);
      for (const d of [`${install}.old`, `${install}.failed`, staging]) {
        expect(existsSync(d)).toBe(false);
      }
    });

    it("waits for a live script to end, then removes", async () => {
      leftovers();
      writeFileSync(switching(), String(process.pid));
      const done = removeLeftovers(install, { pollMs: 10, timeoutMs: 5_000 });
      await new Promise((r) => setTimeout(r, 100));
      expect(existsSync(staging)).toBe(true);
      rmSync(switching());
      await done;
      expect(existsSync(staging)).toBe(false);
    });

    it("removes nothing when the script outlives the wait", async () => {
      leftovers();
      writeFileSync(switching(), String(process.pid));
      await removeLeftovers(install, fast);
      expect(existsSync(`${install}.old`)).toBe(true);
      expect(existsSync(staging)).toBe(true);
    });

    it("is fine when there is nothing to remove", async () => {
      rmSync(staging, { recursive: true });
      await expect(removeLeftovers(install, fast)).resolves.toBeUndefined();
    });
  });
});

describe("listBackups", () => {
  let dir: string;
  const backup = (name: string, meta: unknown) => {
    mkdirSync(path.join(dir, "backups", name), { recursive: true });
    if (meta !== undefined) {
      writeFileSync(
        path.join(dir, "backups", name, "backup.json"),
        typeof meta === "string" ? meta : JSON.stringify(meta),
      );
    }
  };
  beforeEach(() => {
    dir = mkdtempSync(path.join(tmpdir(), "grace-backups-"));
  });
  afterEach(() => rmSync(dir, { recursive: true, force: true }));

  it("lists backups newest first", () => {
    backup("1.0.0-rc.4", { version: "1.0.0-rc.4", savedAt: "2026-10-01T09:00:00.000Z" });
    backup(
      "1.0.0-rc.6",
      BOM + JSON.stringify({ version: "1.0.0-rc.6", savedAt: "2026-10-27T09:00:00.000Z" }),
    );
    backup("1.0.0-rc.5", { version: "1.0.0-rc.5", savedAt: "2026-10-15T09:00:00.000Z" });
    expect(listBackups(dir)).toEqual([
      { version: "1.0.0-rc.6", savedAt: "2026-10-27T09:00:00.000Z" },
      { version: "1.0.0-rc.5", savedAt: "2026-10-15T09:00:00.000Z" },
      { version: "1.0.0-rc.4", savedAt: "2026-10-01T09:00:00.000Z" },
    ]);
  });

  it("skips folders without a valid backup.json", () => {
    backup("1.0.0-rc.4", undefined);
    backup("1.0.0-rc.5", "{oops");
    backup("1.0.0-rc.6", { version: "1.0.0-rc.4", savedAt: "2026-10-27T09:00:00.000Z" });
    backup("1.0.0", { version: "1.0.0", savedAt: "yesterday" });
    backup("nightly", { version: "nightly", savedAt: "2026-10-27T09:00:00.000Z" });
    backup("1.1.0", { version: "1.1.0", savedAt: "2026-10-27T09:00:00.000Z", extra: 1 });
    writeFileSync(path.join(dir, "backups", "stray.txt"), "x");
    expect(listBackups(dir)).toEqual([{ version: "1.1.0", savedAt: "2026-10-27T09:00:00.000Z" }]);
  });

  it("is empty without a backups folder", () => {
    expect(listBackups(dir)).toEqual([]);
  });
});
