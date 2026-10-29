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
  switchRunning,
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

    it("reads a result without reverted, which R10.5 does not list, as not reverted", () => {
      writeFileSync(
        file(),
        JSON.stringify({ ...failed("1.0.0-rc.5", "1.0.0-rc.6", "x"), reverted: undefined }),
      );
      expect(takeResult(dir)).toEqual(failed("1.0.0-rc.5", "1.0.0-rc.6", "x"));
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
        JSON.stringify({ ...ok("1.0.0-rc.5", "1.0.0-rc.6"), rollback: undefined }),
      ],
      [
        "a reverted that is not a boolean",
        JSON.stringify({ ...ok("1.0.0-rc.5", "1.0.0-rc.6"), reverted: "no" }),
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
      // a channel that is not valid, or null as a plan carries one not picked, reads as not picked
      expect(takeResult(dir)?.updates).toEqual(defaultUpdateSettings());
      writeFileSync(file(), JSON.stringify({ ...r, updates: { channel: null } }));
      expect(takeResult(dir)?.updates).toEqual(defaultUpdateSettings());
      writeFileSync(file(), JSON.stringify({ ...r, updates: carried }));
      expect(takeResult(dir)?.updates).toEqual(carried);
      for (const updates of [null, [], ["stable"]]) {
        writeFileSync(file(), JSON.stringify({ ...r, updates }));
        expect(takeResult(dir)?.updates, JSON.stringify(updates)).toBeNull();
      }
    });
  });
});

describe("a switch on disk", () => {
  let root: string;
  let install: string;
  let staging: string;
  const switching = () => path.join(staging, "switching");
  /** Sets the `switching` file's modified time `ms` back. */
  const age = (ms: number) => {
    const t = new Date(Date.now() - ms);
    utimesSync(switching(), t, t);
  };
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

    it("ignores a switching file older than 30 minutes, even one naming a live pid", () => {
      plan("1.0.0-rc.6");
      writeFileSync(switching(), String(process.pid));
      age(29 * 60_000);
      expect(switchInProgress(install, "1.0.0-rc.5")).toBe(true);
      age(31 * 60_000);
      expect(switchInProgress(install, "1.0.0-rc.5")).toBe(false);
    });
  });

  describe("switchRunning", () => {
    it("sees a live script whatever version runs, never a dead or stale one", async () => {
      plan("1.0.0-rc.6");
      writeFileSync(switching(), String(process.pid));
      expect(switchRunning(install)).toBe(true);
      age(31 * 60_000);
      expect(switchRunning(install)).toBe(false);
      writeFileSync(switching(), String(await deadPid()));
      expect(switchRunning(install)).toBe(false);
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
    const old = () => `${install}.old`;
    const aside = () => `${install}.failed`;
    /** A Grace install: its file list is inside. */
    const graceFolder = (dir: string) => {
      mkdirSync(path.join(dir, "resources"), { recursive: true });
      writeFileSync(path.join(dir, "resources", "app-files.json"), "[]");
    };
    /** What a switch of this install leaves: both folders, and its plan in the staging folder. */
    const leftovers = () => {
      graceFolder(old());
      graceFolder(aside());
      writeFileSync(
        path.join(staging, "plan.json"),
        JSON.stringify({ to: "1.0.0-rc.6", installDir: install }),
      );
      writeFileSync(path.join(staging, "Grace-win32-x64-1.0.0-rc.6.zip"), "partial");
    };
    const fast = { pollMs: 10, timeoutMs: 200 };

    it("removes what a switch of this install left (R10.8)", async () => {
      leftovers();
      // the plan names the folder in another case, with a trailing separator
      writeFileSync(
        path.join(staging, "plan.json"),
        JSON.stringify({ to: "1.0.0-rc.6", installDir: `${install.toUpperCase()}\\` }),
      );
      await removeLeftovers(install);
      for (const d of [old(), aside(), staging]) expect(existsSync(d), d).toBe(false);
    });

    it("removes a copy the script moved into staging but could not delete", async () => {
      leftovers();
      // an earlier .failed, moved in by the script, whose removal an antivirus cut short
      graceFolder(path.join(staging, "failed"));
      writeFileSync(path.join(staging, "failed", "left.txt"), "");
      await removeLeftovers(install);
      for (const d of [old(), aside(), staging]) expect(existsSync(d), d).toBe(false);
    });

    it("removes a partial download left by a killed Grace", async () => {
      writeFileSync(path.join(staging, "Grace-win32-x64-1.0.0-rc.6.zip"), "partial");
      mkdirSync(path.join(staging, "app"));
      await removeLeftovers(install);
      expect(existsSync(staging)).toBe(false);
    });

    it("never removes a .old or .failed that is not a Grace folder", async () => {
      leftovers();
      for (const d of [old(), aside()]) {
        rmSync(d, { recursive: true });
        mkdirSync(d);
        writeFileSync(path.join(d, "mine.txt"), "");
      }
      await removeLeftovers(install);
      for (const d of [old(), aside()]) expect(existsSync(path.join(d, "mine.txt")), d).toBe(true);
      expect(existsSync(staging)).toBe(false);
    });

    it("never removes a Grace folder that no switch of this install made", async () => {
      leftovers();
      // a plan of another install
      writeFileSync(
        path.join(staging, "plan.json"),
        JSON.stringify({ to: "1.0.0-rc.6", installDir: path.join(root, "Other") }),
      );
      await removeLeftovers(install);
      for (const d of [old(), aside()]) expect(existsSync(d), d).toBe(true);
      // no plan at all
      mkdirSync(staging);
      await removeLeftovers(install);
      for (const d of [old(), aside()]) expect(existsSync(d), d).toBe(true);
    });

    it("leaves a staging folder that holds anything else, and the folders its plan names", async () => {
      leftovers();
      writeFileSync(path.join(staging, "notes.txt"), "mine");
      await removeLeftovers(install);
      expect(existsSync(path.join(staging, "notes.txt"))).toBe(true);
      expect(existsSync(path.join(staging, "plan.json"))).toBe(true);
      for (const d of [old(), aside()]) expect(existsSync(d), d).toBe(true);
    });

    it("keeps the plan for the next start when a folder cannot be moved away", async () => {
      leftovers();
      // a process whose current folder is .old keeps Windows from moving it
      const holder = spawn(process.execPath, ["-e", "setTimeout(() => {}, 10000)"], { cwd: old() });
      try {
        await new Promise((r) => holder.once("spawn", r));
        await removeLeftovers(install);
        expect(existsSync(path.join(old(), "resources", "app-files.json"))).toBe(true);
        expect(existsSync(path.join(staging, "plan.json"))).toBe(true);
      } finally {
        holder.kill();
        await new Promise((r) => holder.once("exit", r));
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

    it("does not wait for a switching file older than 30 minutes", async () => {
      leftovers();
      writeFileSync(switching(), String(process.pid));
      age(31 * 60_000);
      await removeLeftovers(install, { pollMs: 10, timeoutMs: 5_000 });
      expect(existsSync(staging)).toBe(false);
      expect(existsSync(`${install}.old`)).toBe(false);
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
