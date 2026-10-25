import { app, BrowserWindow, dialog, ipcMain, Menu, shell } from "electron";
import path from "node:path";
import { parsePropertiesEdit } from "./shared/env";
import type { AppEvent } from "./shared/events";
import { CH } from "./shared/ipc";
import { parseFilterState } from "./shared/tags";
import { removeLeftoverArgFiles } from "./main/karate/argfile";
import { killOrphan } from "./main/karate/process";
import { Runner } from "./main/runner";
import { safeStorageBox } from "./main/secrets";
import { loadLastProject, saveLastProject } from "./main/settings";
import { vendorPaths } from "./main/vendor";

// End-to-end tests give each app instance its own settings and run folders.
if (process.env.KARATE_KID_USER_DATA) app.setPath("userData", process.env.KARATE_KID_USER_DATA);

let win: BrowserWindow | null = null;

// One instance per userData folder: a second one would kill the first one's run as an orphan
// and share its run folders. It hands over to the first window instead.
const primary = app.requestSingleInstanceLock();
if (!primary) app.quit();
app.on("second-instance", () => {
  if (!win || win.isDestroyed()) return;
  if (win.isMinimized()) win.restore();
  win.focus();
});

const emit = (e: AppEvent) => {
  if (win && !win.isDestroyed() && !win.webContents.isDestroyed()) {
    win.webContents.send(CH.event, e);
  }
};

const createWindow = () => {
  win = new BrowserWindow({
    width: 1200,
    height: 800,
    webPreferences: {
      preload: path.join(__dirname, "preload.cjs"),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
    },
  });

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

/** Handlers never reject to the renderer: a failed Runner call becomes a notice. */
const handle = (channel: string, fn: (...args: unknown[]) => Promise<void> | void) =>
  ipcMain.handle(channel, async (_e, ...args) => {
    try {
      await fn(...args);
    } catch (err) {
      emit({ type: "notice", message: err instanceof Error ? err.message : String(err) });
    }
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

  handle(CH.ready, () => {
    for (const e of runner.replay()) emit(e);
  });
  handle(CH.open, async () => {
    const r = await dialog.showOpenDialog({ properties: ["openDirectory"] });
    if (r.canceled || !r.filePaths[0]) return;
    if (await runner.open(r.filePaths[0])) saveLastProject(dataDir, r.filePaths[0]);
  });
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
      return;
    }
    runner.saveProperties(edit);
  });
  handle(CH.cancel, () => runner.cancel());
  handle(CH.report, async () => {
    const report = runner.reportPath();
    if (report) await shell.openPath(report);
  });

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
  createWindow();
  try {
    const last = loadLastProject(dataDir);
    if (last) await runner.open(last);
  } catch (err) {
    emit({ type: "notice", message: err instanceof Error ? err.message : String(err) });
  }

  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow();
    }
  });
});

app.on("window-all-closed", () => {
  app.quit();
});
