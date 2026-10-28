// Checks, the startup modal, auto-update and installs, run in main
// (docs/specs/2026-10-28-app-updates.md R2, R4, R5, R8, R9.3).
import { rm } from "node:fs/promises";
import path from "node:path";
import { setTimeout as sleep } from "node:timers/promises";
import type { AppEvent } from "../../shared/events";
import {
  autoUpdateTo,
  promptFor,
  releasePage,
  zipName,
  type Channel,
  type Release,
  type UpdateProgress,
  type UpdateSettings,
  type UpdateView,
} from "../../shared/updates";
import { compareVersions, parseVersion } from "../../shared/version";
import { loadUpdateSettings, saveUpdateSettings } from "../settings";
import { fetchReleases } from "./check";
import { download, unpack, verify } from "./fetchZip";
import { preflight } from "./preflight";
import {
  listBackups,
  resultNotice,
  settingsAfter,
  switchRunning,
  type SwitchResult,
} from "./startup";
import { startSwitch } from "./switch";

export interface UpdaterDeps {
  dataDir: string;
  /** The folder of `process.execPath`. */
  installDir: string;
  running: string;
  packaged: boolean;
  feed: string;
  isBusy(): boolean;
  emit(e: AppEvent): void;
  quit(): void;
  openExternal(url: string): Promise<void>;
  /** Electron's `net.fetch` in the app, which honours the system proxy; Node's fetch by default. */
  fetch?: typeof fetch;
  fetchReleases?: typeof fetchReleases;
  preflight?: typeof preflight;
  download?: typeof download;
  verify?: typeof verify;
  unpack?: typeof unpack;
  startSwitch?: typeof startSwitch;
  /** How often, and how long, auto-update waits for the Runner to be idle. 500 ms and 120 s by default. */
  idlePollMs?: number;
  idleWaitMs?: number;
}

const DAMAGED = "The download was damaged. Try again, or get it from the release page.";
const BUSY = "Finish or cancel the run first";
const UNKNOWN = "Grace can't install that version. Check for updates and try again.";
/** Download progress goes to the window at most this often. */
const PROGRESS_MS = 100;

/** Rethrows with a message that names the step (R8.6), unless the step's own message already does. */
const named =
  (step: string) =>
  (e: unknown): never => {
    const m = e instanceof Error ? e.message : String(e);
    throw new Error(m.startsWith(step) ? m : `${step} failed: ${m}`);
  };

export class Updater {
  private readonly d: Required<Omit<UpdaterDeps, "fetch">> & Pick<UpdaterDeps, "fetch">;
  private settings: UpdateSettings;
  /** The installable releases of the last successful check in this session (R2.7). */
  private releases: Release[] | null = null;
  private error: string | null = null;
  private checking: Promise<void> | null = null;
  /** The install under way, until its staging folder is settled: `cancel` aborts it. */
  private job: AbortController | null = null;
  private shown: UpdateProgress | null = null;

  constructor(deps: UpdaterDeps) {
    this.d = {
      fetchReleases,
      preflight,
      download,
      verify,
      unpack,
      startSwitch,
      idlePollMs: 500,
      idleWaitMs: 120_000,
      ...deps,
    };
    this.settings = loadUpdateSettings(deps.dataDir, deps.running);
  }

  view(): UpdateView {
    const { running, packaged, dataDir } = this.d;
    return {
      running,
      packaged,
      settings: this.settings,
      releases: this.releases,
      checking: this.checking !== null,
      error: this.error,
      backups: listBackups(dataDir),
    };
  }

  progress(): UpdateProgress | null {
    return this.shown;
  }

  /**
   * The last switch's result (R10.7), then the startup check (R2.2), then auto-update (R5.2) or the
   * modal (R4.1). Never rejects.
   */
  async startup(result: SwitchResult | null): Promise<void> {
    try {
      if (result) {
        this.store(settingsAfter(result, this.settings));
        this.d.emit({ type: "notice", message: resultNotice(result, this.d.running) });
        this.send();
      }
      if (!this.settings.checkAtStartup) return;
      await this.check();
      const rs = this.releases;
      if (!rs) return;
      const { running } = this.d;
      // R5.3: never right after a failed switch, so a file held open cannot loop downloads and restarts
      const auto = () =>
        result?.ok === false || !this.settings.checkAtStartup
          ? null
          : autoUpdateTo(rs, this.settings, running);
      let release = auto();
      // a build that is not packaged never installs, and a Runner still busy after the wait (the
      // reopen's discovery, a run) is not killed: the person then decides through the modal
      if (release && this.d.packaged) {
        const idle = await this.idle();
        release = auto(); // the settings may have changed during the wait
        if (release && idle) return await this.install(release.version, false);
      }
      release ??= promptFor(rs, this.settings, running);
      if (release) this.d.emit({ type: "updatePrompt", release });
    } catch {
      // a startup that cannot check or prompt still starts
    }
  }

  /** R2.4: one check at a time; a call during a check waits for it. Never rejects. */
  check(): Promise<void> {
    if (!this.checking) {
      this.checking = this.load().finally(() => {
        this.checking = null;
        this.send();
      });
      this.send();
    }
    return this.checking;
  }

  private async load(): Promise<void> {
    try {
      this.releases = await this.d.fetchReleases(this.d.feed, undefined, this.d.fetch);
      this.error = null;
      this.store({ ...this.settings, lastChecked: new Date().toISOString() });
    } catch (e) {
      this.error = `Couldn't check for updates: ${(e as Error).message}`;
    }
  }

  setSettings(patch: { channel?: Channel; checkAtStartup?: boolean; autoUpdate?: boolean }): void {
    const s = this.settings;
    const {
      channel = s.channel,
      checkAtStartup = s.checkAtStartup,
      autoUpdate = s.autoUpdate,
    } = patch;
    this.store({ ...s, channel, checkAtStartup, autoUpdate });
    this.send();
  }

  /** R4.3: the modal shows again only for a version above it. */
  skip(version: string): void {
    if (!parseVersion(version)) return;
    this.store({ ...this.settings, skipped: version });
    this.send();
  }

  /** R4.2: the URL is built from a valid version, never taken from the response. */
  async whatsNew(version: string): Promise<void> {
    if (parseVersion(version)) await this.d.openExternal(releasePage(version));
  }

  /**
   * R8: preflight, download, check, unpack, then hand over to the switch script and quit. A refusal
   * is a notice; a failure is a progress error, the staging folder removed and nothing else changed.
   */
  async install(version: string, useSavedData: boolean): Promise<void> {
    const { running, installDir, dataDir } = this.d;
    const release = this.releases?.find((r) => r.version === version);
    const refusal = this.job
      ? "Grace is already installing a version."
      : !this.d.packaged
        ? "Updates install only in the packaged app"
        : this.d.isBusy()
          ? BUSY
          : switchRunning(installDir)
            ? "Grace is still finishing the last update. Try again in a minute."
            : null;
    if (refusal || !release) {
      this.d.emit({ type: "notice", message: refusal ?? UNKNOWN });
      return;
    }

    const job = new AbortController();
    this.job = job;
    const rollback = compareVersions(version, running) < 0;
    const stagingDir = `${installDir}.update`;
    const zip = path.join(stagingDir, zipName(version));
    const step = (s: "download" | "check" | "unpack", received = 0) =>
      this.show({ version, rollback, step: s, received, total: release.size });
    const removeStaging = () =>
      rm(stagingDir, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
    try {
      step("download");
      const why = await this.d.preflight(installDir, release.size).catch(named("The folder check"));
      if (why) throw new Error(why);
      await removeStaging().catch(named("The download"));
      let sent = 0;
      const onProgress = (received: number) => {
        const now = Date.now();
        if (received !== release.size && now - sent < PROGRESS_MS) return;
        sent = now;
        step("download", received);
      };
      await this.d
        .download(release.url, zip, onProgress, job.signal, this.d.fetch)
        .catch(named("The download"));
      job.signal.throwIfAborted();
      step("check");
      const intact = await this.d.verify(zip, release.size, release.sha256).catch(() => false);
      if (!intact) throw new Error(DAMAGED);
      job.signal.throwIfAborted();
      step("unpack");
      await this.d.unpack(zip, path.join(stagingDir, "app")).catch(named("Unpacking"));
      job.signal.throwIfAborted();
      // quitting would kill a run started during the download
      if (this.d.isBusy()) throw new Error(BUSY);
      const saved = listBackups(dataDir).some((b) => b.version === version);
      try {
        this.d.startSwitch({
          pid: process.pid,
          from: running,
          to: version,
          rollback,
          // R7.6, R9.3: a rollback restores the target's copy; an update only when asked
          restore: saved && (rollback || useSavedData),
          installDir,
          stagingDir,
          dataDir,
          updates: this.settings,
          exitTimeoutSec: 60,
          startTimeoutSec: 120,
          keepBackups: 3,
        });
      } catch (e) {
        named("The switch script")(e);
      }
    } catch (e) {
      await removeStaging().catch(() => {});
      if (!job.signal.aborted) this.show({ version, rollback, error: (e as Error).message });
      return;
    } finally {
      this.job = null;
    }
    this.d.quit();
  }

  /** R8.6: stops the install under way, or closes a failed one's message. */
  cancel(): void {
    this.job?.abort();
    if (this.shown) this.show(null);
  }

  /** True once the Runner is idle; false if it is still busy after `idleWaitMs`. */
  private async idle(): Promise<boolean> {
    const deadline = Date.now() + this.d.idleWaitMs;
    while (this.d.isBusy()) {
      if (Date.now() >= deadline) return false;
      await sleep(this.d.idlePollMs);
    }
    return true;
  }

  private send(): void {
    this.d.emit({ type: "updates", view: this.view() });
  }

  private show(p: UpdateProgress | null): void {
    this.shown = p;
    this.d.emit({ type: "updateProgress", progress: p });
  }

  /** Applies `s` even if it cannot be saved: a disk error must not undo the change. */
  private store(s: UpdateSettings): void {
    this.settings = s;
    try {
      saveUpdateSettings(this.d.dataDir, s);
    } catch (err) {
      this.d.emit({
        type: "notice",
        message: `The update settings could not be saved: ${(err as Error).message}`,
      });
    }
  }
}
