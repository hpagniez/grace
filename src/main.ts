import {
  app,
  BrowserWindow,
  dialog,
  ipcMain,
  Menu,
  nativeTheme,
  net,
  screen,
  shell,
  type OpenDialogOptions,
} from "electron";
import path from "node:path";
import { parsePropertiesEdit } from "./shared/env";
import type { AppEvent } from "./shared/events";
import { RUN_ID } from "./shared/history";
import { CH } from "./shared/ipc";
import { parseFilterState } from "./shared/tags";
import type { Channel } from "./shared/updates";
import { parseVersion } from "./shared/version";
import { removeLeftoverRuns } from "./main/history";
import { removeLeftoverArgFiles } from "./main/karate/argfile";
import { killOrphan } from "./main/karate/process";
import { checkRecent } from "./main/recent";
import { Runner } from "./main/runner";
import { safeStorageBox } from "./main/secrets";
import {
  loadLastProject,
  loadRecentProjects,
  loadWindowState,
  saveWindowState,
} from "./main/settings";
import { vendorPaths } from "./main/vendor";
import { updateFeed } from "./main/updates/check";
import { markStarted, removeLeftovers, switchInProgress, takeResult } from "./main/updates/startup";
import { Updater } from "./main/updates/updater";
import { placeWindow } from "./main/window";

// End-to-end tests give each app instance its own settings and run folders.
if (process.env.GRACE_USER_DATA) app.setPath("userData", process.env.GRACE_USER_DATA);

let win: BrowserWindow | null = null;
let startMaximized = false;
/** The folder Grace runs from, which an update replaces (R8.1). */
const installDir = path.dirname(process.execPath);

/** Shows the window the first time, maximized if it was saved so. A minimized window was shown already. */
const reveal = () => {
  if (!win || win.isDestroyed() || win.isVisible() || win.isMinimized()) return;
  if (startMaximized) {
    win.maximize(); // shows a hidden window, but does not focus it
    win.focus();
  } else win.show();
  // R10.6: the switch script keeps this version only once it has shown its window
  if (app.isPackaged) markStarted(installDir, app.getVersion());
};

// R10.9: a switch script is replacing this install, and this is not the version it installs
// (R10.6). Decided before the single-instance lock: a Grace holding it would make the version the
// script starts quit, and the script would switch back.
const finishing = app.isPackaged && switchInProgress(installDir, app.getVersion());
// One instance per userData folder: a second one would kill the first one's run as an orphan
// and share its run folders. It hands over to the first window instead.
const primary = !finishing && app.requestSingleInstanceLock();
if (finishing) {
  // never blocking, no window: quits when dismissed or after 5 s, before the script's swap
  void app.whenReady().then(async () => {
    await dialog
      .showMessageBox({
        message: "Grace is finishing an update and starts on its own in a moment.",
        signal: AbortSignal.timeout(5000),
      })
      .catch(() => {});
    app.quit();
  });
} else if (!primary) app.quit();
app.on("second-instance", () => {
  if (!win || win.isDestroyed()) return;
  if (win.isMinimized()) win.restore();
  reveal(); // still hidden if it has not painted yet
  win.focus();
});

const emit = (e: AppEvent) => {
  if (win && !win.isDestroyed() && !win.webContents.isDestroyed()) {
    win.webContents.send(CH.event, e);
  }
};

const createWindow = (dataDir: string) => {
  const saved = loadWindowState(dataDir);
  const placed =
    saved &&
    placeWindow(
      saved,
      screen.getAllDisplays().map((d) => d.workArea),
    );
  startMaximized = saved?.maximized ?? false;
  win = new BrowserWindow({
    ...(placed ?? { width: 1200, height: 800 }),
    minHeight: 480, // room for the bars, 120 px of tests and 96 px of bottom panel
    show: false, // shown once painted, so it never flashes white
    backgroundColor: nativeTheme.shouldUseDarkColors ? "#18181b" : "#ffffff", // the app root's two backgrounds
    // packaged, the window takes Grace.exe's icon; from source it would get electron.exe's
    ...(app.isPackaged ? {} : { icon: path.join(app.getAppPath(), "assets", "icon.ico") }),
    webPreferences: {
      preload: path.join(__dirname, "preload.cjs"),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
    },
  });
  // The constructor fits the size to the main screen's work area; setBounds keeps it.
  if (placed) win.setBounds(placed);

  // A window that never shows would leave the app unreachable: show it anyway if the page fails or is slow.
  const w = win;
  w.once("ready-to-show", reveal);
  w.webContents.once("did-fail-load", reveal);
  setTimeout(reveal, 5000);

  // Saved at the end of each move or resize by the person too, so a crash or a shutdown keeps it.
  const save = () => {
    try {
      const maximized = w.isMaximized() && !w.isMinimized();
      const bounds = maximized || w.isMinimized() ? w.getNormalBounds() : w.getBounds();
      saveWindowState(dataDir, { ...bounds, maximized });
    } catch (err) {
      console.error(err); // the last saved state stays; never block the close
    }
  };
  w.on("close", save)
    .on("moved", save)
    .on("resized", save)
    .on("maximize", save)
    .on("unmaximize", save);

  // The window only ever shows the app's own page.
  const contents = win.webContents;
  contents.setWindowOpenHandler(() => ({ action: "deny" }));
  contents.on("will-navigate", (e) => {
    if (e.url !== contents.getURL()) e.preventDefault();
  });

  if (MAIN_WINDOW_VITE_DEV_SERVER_URL) {
    win.loadURL(MAIN_WINDOW_VITE_DEV_SERVER_URL);
  } else {
    win.loadFile(path.join(__dirname, `../renderer/${MAIN_WINDOW_VITE_NAME}/index.html`));
  }
};

/**
 * Handlers never reject to the renderer: a failed Runner call becomes a notice and resolves
 * undefined. Otherwise the handler's return value is the answer.
 */
const handle = (channel: string, fn: (...args: unknown[]) => unknown) =>
  ipcMain.handle(channel, async (_e, ...args) => {
    try {
      return await fn(...args);
    } catch (err) {
      emit({ type: "notice", message: err instanceof Error ? err.message : String(err) });
    }
  });

/** History requests carry a run's folder id only: anything else is refused before the Runner sees it. */
const handleRunId = (channel: string, fn: (id: string) => unknown) =>
  handle(channel, (id) => {
    if (typeof id === "string" && RUN_ID.test(id)) return fn(id);
    emit({ type: "notice", message: "Invalid request" });
  });

/** Update requests carry a version only: anything else is refused before the Updater sees it. */
const handleVersion = (channel: string, fn: (version: string) => unknown) =>
  handle(channel, (v) => {
    if (typeof v === "string" && parseVersion(v)) return fn(v);
    emit({ type: "notice", message: "Invalid request" });
  });

/** A settings change from the window: a plain object with only these keys, each of its type. */
const isUpdatePatch = (
  p: unknown,
): p is { channel?: Channel; checkAtStartup?: boolean; autoUpdate?: boolean } =>
  typeof p === "object" &&
  p !== null &&
  Object.getPrototypeOf(p) === Object.prototype &&
  Object.entries(p).every(([k, v]) =>
    k === "channel"
      ? v === "stable" || v === "prerelease"
      : (k === "checkAtStartup" || k === "autoUpdate") && typeof v === "boolean",
  );

/** Recent-list requests carry a folder path only: anything else is refused before the Runner sees it. */
const handlePath = (channel: string, fn: (root: string) => unknown) =>
  handle(channel, (root) => {
    if (typeof root === "string" && root.length <= 1000) return fn(root);
    emit({ type: "notice", message: "Invalid request" });
  });

app.whenReady().then(async () => {
  if (!primary) return;
  // No menu: its Reload would drop the window's view of a live run.
  Menu.setApplicationMenu(null);
  const dataDir = app.getPath("userData");
  const vendor = vendorPaths(
    app.isPackaged ? process.resourcesPath : path.join(app.getAppPath(), "vendor"),
  );
  const runner = new Runner({ vendor, dataDir, secrets: safeStorageBox(), emit });
  // The window loses events sent before it subscribes: the one-shot notice and prompt wait for `ready`.
  let held: AppEvent[] | null = [];
  const updater = new Updater({
    dataDir,
    installDir,
    running: app.getVersion(),
    packaged: app.isPackaged,
    feed: updateFeed(process.env, app.isPackaged),
    isBusy: () => runner.isBusy(),
    emit: (e) => {
      if (held && (e.type === "notice" || e.type === "updatePrompt")) held.push(e);
      else emit(e);
    },
    quit: () => app.quit(),
    openExternal: (url) => shell.openExternal(url),
    // Electron's fetch honours the system proxy
    fetch: (input, init) => net.fetch(input as string, init),
  });

  handle(CH.ready, () => {
    emit({ type: "updates", view: updater.view() });
    emit({ type: "updateProgress", progress: updater.progress() });
    for (const e of runner.replay()) emit(e);
    void runner.sendRecent();
    // after the replay, whose project event clears the notice
    for (const e of held ?? []) emit(e);
    held = null;
  });
  handle(CH.open, async () => {
    // Start next to the newest recent folder that still exists, so its siblings are one click away.
    const newest = (await checkRecent(loadRecentProjects(dataDir))).find((r) => r.exists);
    const options: OpenDialogOptions = {
      properties: ["openDirectory"],
      ...(newest ? { defaultPath: path.dirname(newest.path) } : {}),
    };
    const r =
      win && !win.isDestroyed()
        ? await dialog.showOpenDialog(win, options)
        : await dialog.showOpenDialog(options);
    if (r.canceled || !r.filePaths[0]) return;
    await runner.open(r.filePaths[0]);
  });
  handlePath(CH.openRecent, (root) => runner.openRecent(root));
  handlePath(CH.removeRecent, (root) => runner.removeRecent(root));
  handle(CH.close, () => runner.close());
  handle(CH.refresh, () => runner.refresh());
  handle(CH.run, async (key) => {
    if (typeof key !== "string" || key.length > 1000 || !/^[dfos]:/.test(key)) {
      emit({ type: "notice", message: "Invalid request" });
      return;
    }
    await runner.run(key);
  });
  handle(CH.filter, (f) => {
    const filter = parseFilterState(f);
    if (!filter) {
      emit({ type: "notice", message: "Invalid request" });
      return;
    }
    runner.setFilter(filter);
  });
  handle(CH.threads, (n) => runner.setThreads(n));
  handle(CH.env, async (name) => {
    if (name !== null && (typeof name !== "string" || name.length > 64)) {
      emit({ type: "notice", message: "Invalid request" });
      return;
    }
    await runner.setEnv(name);
  });
  handle(CH.properties, (e) => {
    const edit = parsePropertiesEdit(e);
    if (!edit) {
      emit({ type: "notice", message: "Invalid request" });
      return false;
    }
    return runner.saveProperties(edit);
  });
  handle(CH.cancel, () => runner.cancel());
  handle(CH.report, async () => {
    const report = runner.reportPath();
    if (report) await shell.openPath(report);
  });
  handleRunId(CH.rerun, (id) => runner.rerun(id));
  handleRunId(CH.rerunFailed, (id) => runner.rerunFailed(id));
  handleRunId(CH.runReport, async (id) => {
    const report = runner.runReportPath(id);
    if (report) await shell.openPath(report);
  });
  handle(CH.clearHistory, () => runner.clearHistory());
  handle(CH.checkUpdates, () => updater.check());
  handle(CH.installVersion, async (version, useSavedData) => {
    if (
      typeof version !== "string" ||
      !parseVersion(version) ||
      typeof useSavedData !== "boolean"
    ) {
      emit({ type: "notice", message: "Invalid request" });
      return;
    }
    await updater.install(version, useSavedData);
  });
  handle(CH.cancelUpdate, () => updater.cancel());
  handle(CH.updateSettings, (patch) => {
    if (!isUpdatePatch(patch)) {
      emit({ type: "notice", message: "Invalid request" });
      return;
    }
    updater.setSettings(patch);
  });
  handleVersion(CH.skipVersion, (version) => updater.skip(version));
  handleVersion(CH.whatsNew, (version) => updater.whatsNew(version));

  let quitting = false;
  app.on("before-quit", (e) => {
    if (quitting) return;
    quitting = true;
    e.preventDefault();
    runner
      .dispose()
      .catch(() => {}) // quit anyway
      .then(() => app.quit());
  });

  try {
    await killOrphan(path.join(dataDir, "running.pid"), vendor.jreDir);
  } catch (err) {
    console.error(err);
  }
  removeLeftoverArgFiles(dataDir);
  removeLeftoverRuns(dataDir);
  createWindow(dataDir);
  if (app.isPackaged) void removeLeftovers(installDir);
  try {
    const last = loadLastProject(dataDir);
    if (last) await runner.open(last);
  } catch (err) {
    emit({ type: "notice", message: err instanceof Error ? err.message : String(err) });
  }
  // After the reopen, whose project event would clear the result's notice (R10.7), and never
  // awaited: the check delays neither the window nor the reopening (R2.2). The result file is
  // deleted only here, as it is applied, so a Grace that dies during the reopen reads it again.
  void updater.startup(takeResult(dataDir));

  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow(dataDir);
    }
  });
});

app.on("window-all-closed", () => {
  app.quit();
});
