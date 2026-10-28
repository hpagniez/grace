import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AppEvent } from "../../shared/events";
import { downloadPrefix, zipName, type Release } from "../../shared/updates";
import { loadUpdateSettings, saveUpdateSettings } from "../settings";
import type { fetchReleases as FetchReleases } from "./check";
import type { download as Download, unpack as Unpack, verify as Verify } from "./fetchZip";
import type { preflight as Preflight } from "./preflight";
import type { SwitchResult } from "./startup";
import type { startSwitch as StartSwitch } from "./switch";
import { Updater, type UpdaterDeps } from "./updater";

const RUNNING = "1.0.0-rc.5";
const SIZE = 1000;
const rel = (version: string): Release => ({
  version,
  prerelease: true,
  publishedAt: "2026-10-28T10:00:00Z",
  url: `${downloadPrefix(version)}${zipName(version)}`,
  size: SIZE,
  sha256: "ab".repeat(32),
});
const RELEASES = [rel("1.0.0-rc.6"), rel(RUNNING), rel("1.0.0-rc.4")];
const DAMAGED = "The download was damaged. Try again, or get it from the release page.";
/** What the real download leaves: the staging folder with the zip in it. */
const writeZip = (dest: string) => {
  mkdirSync(path.dirname(dest), { recursive: true });
  writeFileSync(dest, "zip");
};

let root: string;
let dataDir: string;
let installDir: string;
let staging: string;
let events: AppEvent[];
let busy: boolean;
let fetchReleases: ReturnType<typeof vi.fn<typeof FetchReleases>>;
let preflight: ReturnType<typeof vi.fn<typeof Preflight>>;
let download: ReturnType<typeof vi.fn<typeof Download>>;
let verify: ReturnType<typeof vi.fn<typeof Verify>>;
let unpack: ReturnType<typeof vi.fn<typeof Unpack>>;
let startSwitch: ReturnType<typeof vi.fn<typeof StartSwitch>>;
let quit: ReturnType<typeof vi.fn<() => void>>;
let openExternal: ReturnType<typeof vi.fn<(url: string) => Promise<void>>>;

beforeEach(() => {
  root = mkdtempSync(path.join(tmpdir(), "grace-updater-"));
  dataDir = path.join(root, "data");
  installDir = path.join(root, "Grace");
  staging = `${installDir}.update`;
  mkdirSync(installDir);
  events = [];
  busy = false;
  fetchReleases = vi.fn<typeof FetchReleases>(async () => RELEASES);
  preflight = vi.fn<typeof Preflight>(async () => null);
  download = vi.fn<typeof Download>(async (_url, dest, onProgress) => {
    writeZip(dest);
    onProgress(SIZE);
  });
  verify = vi.fn<typeof Verify>(async () => true);
  unpack = vi.fn<typeof Unpack>(async () => {});
  startSwitch = vi.fn<typeof StartSwitch>();
  quit = vi.fn<() => void>();
  openExternal = vi.fn<(url: string) => Promise<void>>(async () => {});
});
afterEach(() => rmSync(root, { recursive: true, force: true }));

const make = (over: Partial<UpdaterDeps> = {}) =>
  new Updater({
    dataDir,
    installDir,
    running: RUNNING,
    packaged: true,
    feed: "https://example.test/releases",
    isBusy: () => busy,
    emit: (e) => events.push(e),
    quit,
    openExternal,
    fetchReleases,
    preflight,
    download,
    verify,
    unpack,
    startSwitch,
    ...over,
  });

/** An Updater that has checked, so its releases are known. */
async function checked(over: Partial<UpdaterDeps> = {}) {
  const u = make(over);
  await u.check();
  events = [];
  return u;
}

const ofType = <T extends AppEvent["type"]>(t: T) =>
  events.filter((e): e is Extract<AppEvent, { type: T }> => e.type === t);
const notices = () => ofType("notice").map((e) => e.message);
const saveSettings = (patch: object) =>
  saveUpdateSettings(dataDir, { ...loadUpdateSettings(dataDir, RUNNING), ...patch });
const backup = (version: string) => {
  const dir = path.join(dataDir, "backups", version);
  mkdirSync(dir, { recursive: true });
  writeFileSync(
    path.join(dir, "backup.json"),
    JSON.stringify({ version, savedAt: "2026-10-27T10:00:00Z" }),
  );
};

describe("startup", () => {
  it("applies the result, then checks, then prompts (R2.2, R4.1)", async () => {
    const result: SwitchResult = {
      from: "1.0.0-rc.4",
      to: RUNNING,
      rollback: false,
      ok: true,
      reverted: false,
      error: null,
      updates: { ...loadUpdateSettings(dataDir, RUNNING), channel: "prerelease" },
    };
    const u = make();
    await u.startup(result);
    expect(events.map((e) => e.type)).toEqual([
      "notice",
      "updates",
      "updates",
      "updates",
      "updatePrompt",
    ]);
    expect(notices()).toEqual(["Updated to 1.0.0-rc.5."]);
    expect(ofType("updatePrompt")[0].release.version).toBe("1.0.0-rc.6");
    expect(u.view().releases).toEqual(RELEASES);
    expect(download).not.toHaveBeenCalled();
  });

  it("no check and no request when checkAtStartup is off (R2.3)", async () => {
    saveSettings({ checkAtStartup: false });
    await make().startup(null);
    expect(fetchReleases).not.toHaveBeenCalled();
    expect(ofType("updatePrompt")).toEqual([]);
  });

  it("auto-update installs instead of prompting (R5.2)", async () => {
    saveSettings({ autoUpdate: true, skipped: "1.0.0-rc.6" });
    await make().startup(null);
    expect(startSwitch).toHaveBeenCalledWith(expect.objectContaining({ to: "1.0.0-rc.6" }));
    expect(quit).toHaveBeenCalled();
    expect(ofType("updatePrompt")).toEqual([]);
  });

  it("auto-update waits for the Runner to be idle, then installs", async () => {
    saveSettings({ autoUpdate: true });
    let polls = 0;
    await make({ isBusy: () => ++polls < 3, idlePollMs: 1 }).startup(null);
    expect(polls).toBeGreaterThanOrEqual(3);
    expect(startSwitch).toHaveBeenCalledWith(expect.objectContaining({ to: "1.0.0-rc.6" }));
    expect(notices()).toEqual([]);
  });

  it("auto-update prompts instead when the Runner stays busy", async () => {
    saveSettings({ autoUpdate: true });
    busy = true;
    await make({ idlePollMs: 1, idleWaitMs: 20 }).startup(null);
    expect(ofType("updatePrompt").map((e) => e.release.version)).toEqual(["1.0.0-rc.6"]);
    expect(download).not.toHaveBeenCalled();
    expect(notices()).toEqual([]);
  });

  it("auto-update prompts instead in a build that is not packaged (R13.1)", async () => {
    saveSettings({ autoUpdate: true });
    await make({ packaged: false }).startup(null);
    expect(ofType("updatePrompt").map((e) => e.release.version)).toEqual(["1.0.0-rc.6"]);
    expect(preflight).not.toHaveBeenCalled();
    expect(notices()).toEqual([]);
  });

  it("no auto-update right after a failed switch (R5.3)", async () => {
    saveSettings({ autoUpdate: true });
    const result: SwitchResult = {
      from: RUNNING,
      to: "1.0.0-rc.6",
      rollback: false,
      ok: false,
      reverted: false,
      error: "Grace did not exit within 60 s",
      updates: null,
    };
    await make().startup(result);
    expect(fetchReleases).toHaveBeenCalledOnce();
    expect(download).not.toHaveBeenCalled();
    expect(ofType("updatePrompt")).toEqual([]);
    expect(notices()).toEqual([
      "Couldn't switch to 1.0.0-rc.6: Grace did not exit within 60 s. You are still on 1.0.0-rc.5.",
    ]);
  });

  it("saves the result's settings: a rollback holds (R10.7)", async () => {
    saveSettings({ checkAtStartup: false, autoUpdate: true });
    const result: SwitchResult = {
      from: "1.0.0-rc.6",
      to: RUNNING,
      rollback: true,
      ok: true,
      reverted: false,
      error: null,
      updates: loadUpdateSettings(dataDir, RUNNING),
    };
    await make().startup(result);
    expect(loadUpdateSettings(dataDir, RUNNING)).toMatchObject({
      autoUpdate: false,
      skipped: "1.0.0-rc.6",
    });
  });
});

describe("check", () => {
  it("one at a time (R2.4)", async () => {
    let answer!: (rs: Release[]) => void;
    fetchReleases.mockImplementation(() => new Promise((ok) => (answer = ok)));
    const u = make();
    const first = u.check();
    const second = u.check();
    expect(u.view().checking).toBe(true);
    answer(RELEASES);
    await Promise.all([first, second]);
    expect(fetchReleases).toHaveBeenCalledOnce();
    expect(u.view()).toMatchObject({ checking: false, releases: RELEASES, error: null });
  });

  it("saves the time of a successful check (R2.6)", async () => {
    await make().check();
    expect(loadUpdateSettings(dataDir, RUNNING).lastChecked).not.toBeNull();
  });

  it("a failure keeps lastChecked and sets the error (R2.5)", async () => {
    saveSettings({ lastChecked: "2026-10-27T10:00:00.000Z" });
    fetchReleases.mockRejectedValue(new Error("GitHub answered 403"));
    const u = make();
    await u.check();
    expect(u.view().error).toBe("Couldn't check for updates: GitHub answered 403");
    expect(u.view().settings.lastChecked).toBe("2026-10-27T10:00:00.000Z");
    expect(loadUpdateSettings(dataDir, RUNNING).lastChecked).toBe("2026-10-27T10:00:00.000Z");
    expect(u.view().releases).toBeNull();
  });

  it("fetches through the fetch it is given", async () => {
    const f = vi.fn<typeof fetch>();
    await make({ fetch: f }).check();
    expect(fetchReleases).toHaveBeenCalledWith("https://example.test/releases", undefined, f);
  });
});

describe("settings", () => {
  it("setSettings saves, then emits updates", () => {
    const u = make();
    u.setSettings({ channel: "stable", autoUpdate: true });
    expect(loadUpdateSettings(dataDir, RUNNING)).toMatchObject({
      channel: "stable",
      autoUpdate: true,
      checkAtStartup: true,
    });
    expect(ofType("updates").at(-1)?.view.settings.channel).toBe("stable");
  });

  it("skip saves a valid version only", () => {
    const u = make();
    u.skip("nonsense");
    expect(loadUpdateSettings(dataDir, RUNNING).skipped).toBeNull();
    u.skip("1.0.0-rc.6");
    expect(loadUpdateSettings(dataDir, RUNNING).skipped).toBe("1.0.0-rc.6");
    expect(ofType("updates")).toHaveLength(1);
  });

  it("lists the saved copies of the data (R9)", () => {
    backup("1.0.0-rc.4");
    expect(make().view().backups).toEqual([
      { version: "1.0.0-rc.4", savedAt: "2026-10-27T10:00:00Z" },
    ]);
  });
});

describe("install", () => {
  it("refused while busy or not packaged (R6.4)", async () => {
    const u = await checked();
    const dev = await checked({ packaged: false });
    busy = true;
    await u.install("1.0.0-rc.6", false);
    busy = false;
    await dev.install("1.0.0-rc.6", false);
    expect(notices()).toEqual([
      "Finish or cancel the run first",
      "Updates install only in the packaged app",
    ]);
    expect(preflight).not.toHaveBeenCalled();
    expect(download).not.toHaveBeenCalled();
    expect(ofType("updateProgress")).toEqual([]);
  });

  it("refuses a version the last check did not find, and before any check", async () => {
    const fresh = make();
    const u = await checked();
    await fresh.install("1.0.0-rc.6", false);
    await u.install("1.0.0-rc.9", false);
    expect(notices()).toEqual([
      "Grace can't install that version. Check for updates and try again.",
      "Grace can't install that version. Check for updates and try again.",
    ]);
    expect(download).not.toHaveBeenCalled();
  });

  it("refuses while a switch script still runs, and leaves its folder", async () => {
    mkdirSync(staging);
    writeFileSync(path.join(staging, "switching"), String(process.pid));
    await (await checked()).install("1.0.0-rc.6", false);
    expect(notices()).toEqual(["Grace is still finishing the last update. Try again in a minute."]);
    expect(existsSync(path.join(staging, "switching"))).toBe(true);
    expect(preflight).not.toHaveBeenCalled();
  });

  it("refuses a second install while one runs", async () => {
    let release!: () => void;
    preflight.mockImplementation(() => new Promise((ok) => (release = () => ok(null))));
    const u = await checked();
    const first = u.install("1.0.0-rc.6", false);
    await u.install("1.0.0-rc.6", false);
    expect(notices()).toEqual(["Grace is already installing a version."]);
    await vi.waitFor(() => expect(preflight).toHaveBeenCalledOnce());
    release();
    await first;
    expect(startSwitch).toHaveBeenCalledOnce();
  });

  it("a damaged download changes nothing (R8.3)", async () => {
    verify.mockResolvedValue(false);
    const u = await checked();
    await u.install("1.0.0-rc.6", false);
    expect(u.progress()).toEqual({ version: "1.0.0-rc.6", rollback: false, error: DAMAGED });
    expect(startSwitch).not.toHaveBeenCalled();
    expect(existsSync(staging)).toBe(false);
    expect(quit).not.toHaveBeenCalled();
  });

  it("a verify that rejects counts as a damaged download", async () => {
    verify.mockRejectedValue(Object.assign(new Error("ENOENT"), { code: "ENOENT" }));
    const u = await checked();
    await u.install("1.0.0-rc.6", false);
    expect(u.progress()).toEqual({ version: "1.0.0-rc.6", rollback: false, error: DAMAGED });
  });

  it("removes a leftover staging folder before the download (R8.2)", async () => {
    mkdirSync(staging);
    writeFileSync(path.join(staging, "old.zip"), "old");
    await (await checked()).install("1.0.0-rc.6", false);
    expect(existsSync(path.join(staging, "old.zip"))).toBe(false);
    expect(download).toHaveBeenCalledWith(
      rel("1.0.0-rc.6").url,
      path.join(staging, zipName("1.0.0-rc.6")),
      expect.any(Function),
      expect.any(AbortSignal),
      undefined,
    );
    expect(unpack).toHaveBeenCalledWith(
      path.join(staging, zipName("1.0.0-rc.6")),
      path.join(staging, "app"),
    );
  });

  it("shows each step, with download progress at most every 100 ms (R8)", async () => {
    download.mockImplementation(async (_url, dest, onProgress) => {
      writeZip(dest);
      for (let n = 1; n <= SIZE; n++) onProgress(n);
    });
    await (await checked()).install("1.0.0-rc.6", false);
    const steps = ofType("updateProgress").map((e) => e.progress);
    const downloads = steps.filter((p) => p && "step" in p && p.step === "download");
    expect(downloads.length).toBeLessThanOrEqual(4);
    expect(downloads.at(-1)).toMatchObject({ received: SIZE, total: SIZE });
    expect(
      steps.map((p) => p && "step" in p && p.step).filter((s, i, a) => s !== a[i - 1]),
    ).toEqual(["download", "check", "unpack"]);
  });

  it("each failing step ends in an error, never a throw, and removes the staging folder (R8.6)", async () => {
    const cases: [string, () => void, string][] = [
      [
        "preflight",
        () =>
          preflight.mockResolvedValue("Not enough free space on D: Grace needs 4 MB to update."),
        "Not enough free space on D: Grace needs 4 MB to update.",
      ],
      [
        "preflight rejects",
        () => preflight.mockRejectedValue(new Error("EPERM: scandir")),
        "The folder check failed: EPERM: scandir",
      ],
      [
        "download",
        () => download.mockRejectedValue(new TypeError("fetch failed")),
        "The download failed: fetch failed",
      ],
      [
        "download answer",
        () => download.mockRejectedValue(new Error("The download answered 404")),
        "The download answered 404",
      ],
      [
        "unpack",
        () => unpack.mockRejectedValue(new Error("The unpacked update has no Grace.exe")),
        "Unpacking failed: The unpacked update has no Grace.exe",
      ],
      [
        "switch",
        () =>
          startSwitch.mockImplementation(() => {
            throw new Error("The switch script did not start");
          }),
        "The switch script did not start",
      ],
    ];
    for (const [, arrange, error] of cases) {
      arrange();
      const u = await checked();
      await expect(u.install("1.0.0-rc.6", false)).resolves.toBeUndefined();
      expect(u.progress()).toEqual({ version: "1.0.0-rc.6", rollback: false, error });
      expect(existsSync(staging)).toBe(false);
      expect(quit).not.toHaveBeenCalled();
      // the next case starts from working fakes
      preflight.mockResolvedValue(null);
      download.mockImplementation(async (_url, dest) => writeZip(dest));
      unpack.mockResolvedValue();
      startSwitch.mockImplementation(() => {});
    }
  });

  it("does not hand over when a run started during the download (R6.4)", async () => {
    download.mockImplementation(async (_url, dest) => {
      writeZip(dest);
      busy = true;
    });
    const u = await checked();
    await u.install("1.0.0-rc.6", false);
    expect(u.progress()).toEqual({
      version: "1.0.0-rc.6",
      rollback: false,
      error: "Finish or cancel the run first",
    });
    expect(existsSync(staging)).toBe(false);
    expect(startSwitch).not.toHaveBeenCalled();
    expect(quit).not.toHaveBeenCalled();
  });

  it("hands over with the right plan, then quits (R8.5)", async () => {
    const u = await checked();
    await u.install("1.0.0-rc.6", false);
    expect(startSwitch).toHaveBeenCalledWith(
      expect.objectContaining({
        pid: process.pid,
        from: "1.0.0-rc.5",
        to: "1.0.0-rc.6",
        rollback: false,
        restore: false,
        installDir,
        stagingDir: staging,
        dataDir,
        updates: loadUpdateSettings(dataDir, RUNNING),
        exitTimeoutSec: 60,
        startTimeoutSec: 120,
        keepBackups: 3,
      }),
    );
    expect(quit).toHaveBeenCalled();
    expect(startSwitch.mock.invocationCallOrder[0]).toBeLessThan(quit.mock.invocationCallOrder[0]);
  });

  it("a rollback restores a saved copy; an update only when asked (R7.6, R9.3)", async () => {
    const plan = () => startSwitch.mock.lastCall![0];
    const u = await checked();
    await u.install("1.0.0-rc.4", false);
    expect(plan()).toMatchObject({ to: "1.0.0-rc.4", rollback: true, restore: false });
    backup("1.0.0-rc.4");
    backup("1.0.0-rc.6");
    await u.install("1.0.0-rc.4", false);
    expect(plan()).toMatchObject({ to: "1.0.0-rc.4", rollback: true, restore: true });
    await u.install("1.0.0-rc.6", false);
    expect(plan()).toMatchObject({ to: "1.0.0-rc.6", rollback: false, restore: false });
    await u.install("1.0.0-rc.6", true);
    expect(plan()).toMatchObject({ to: "1.0.0-rc.6", rollback: false, restore: true });
  });
});

describe("cancel", () => {
  it("removes the staging folder and clears progress (R8.6)", async () => {
    download.mockImplementation(
      async (_url, dest, _onProgress, signal) =>
        new Promise((_ok, fail) => {
          writeZip(dest);
          signal.addEventListener("abort", () => fail(signal.reason));
        }),
    );
    const u = await checked();
    const install = u.install("1.0.0-rc.6", false);
    await vi.waitFor(() => expect(download).toHaveBeenCalled());
    u.cancel();
    expect(u.progress()).toBeNull();
    await install;
    expect(existsSync(staging)).toBe(false);
    expect(u.progress()).toBeNull();
    expect(ofType("updateProgress").at(-1)?.progress).toBeNull();
    expect(startSwitch).not.toHaveBeenCalled();
    expect(quit).not.toHaveBeenCalled();
  });

  it("closes a failed install's message", async () => {
    verify.mockResolvedValue(false);
    const u = await checked();
    await u.install("1.0.0-rc.6", false);
    u.cancel();
    expect(u.progress()).toBeNull();
    expect(ofType("updateProgress").at(-1)?.progress).toBeNull();
  });
});

it("whatsNew opens only the release page of a valid version (R4.2)", async () => {
  const u = make();
  await u.whatsNew("1.0.0-rc.6");
  expect(openExternal).toHaveBeenCalledWith(
    "https://github.com/hpagniez/grace/releases/tag/v1.0.0-rc.6",
  );
  await u.whatsNew('x" & calc');
  expect(openExternal).toHaveBeenCalledTimes(1);
});
