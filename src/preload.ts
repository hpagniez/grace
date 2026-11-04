import { contextBridge, ipcRenderer } from "electron";
import type { AppEvent } from "./shared/events";
import { CH, type Api } from "./shared/ipc";

const api: Api = {
  ready: () => ipcRenderer.invoke(CH.ready),
  openProject: () => ipcRenderer.invoke(CH.open),
  refresh: () => ipcRenderer.invoke(CH.refresh),
  run: (key) => ipcRenderer.invoke(CH.run, key),
  debug: (key) => ipcRenderer.invoke(CH.debug, key),
  debugCommand: (c) => ipcRenderer.invoke(CH.debugCommand, c),
  toggleBreakpoint: (file, line) => ipcRenderer.invoke(CH.breakpoint, file, line),
  steps: (key) => ipcRenderer.invoke(CH.steps, key),
  openInEditor: (file, line) => ipcRenderer.invoke(CH.openInEditor, file, line),
  editorMenu: (file, line, x, y) => ipcRenderer.invoke(CH.editorMenu, file, line, x, y),
  cancel: () => ipcRenderer.invoke(CH.cancel),
  openReport: () => ipcRenderer.invoke(CH.report),
  setFilter: (f) => ipcRenderer.invoke(CH.filter, f),
  setThreads: (n) => ipcRenderer.invoke(CH.threads, n),
  setWatch: (on) => ipcRenderer.invoke(CH.watch, on),
  setEnv: (name) => ipcRenderer.invoke(CH.env, name),
  saveProperties: (edit) => ipcRenderer.invoke(CH.properties, edit),
  importProperties: () => ipcRenderer.invoke(CH.importProperties),
  rerun: (id) => ipcRenderer.invoke(CH.rerun, id),
  rerunFailed: (id) => ipcRenderer.invoke(CH.rerunFailed, id),
  openRunReport: (id) => ipcRenderer.invoke(CH.runReport, id),
  exportReport: (id) => ipcRenderer.invoke(CH.exportReport, id),
  clearHistory: () => ipcRenderer.invoke(CH.clearHistory),
  openRecent: (root) => ipcRenderer.invoke(CH.openRecent, root),
  removeRecent: (root) => ipcRenderer.invoke(CH.removeRecent, root),
  closeProject: () => ipcRenderer.invoke(CH.close),
  checkUpdates: () => ipcRenderer.invoke(CH.checkUpdates),
  installVersion: (version, useSavedData) =>
    ipcRenderer.invoke(CH.installVersion, version, useSavedData),
  cancelUpdate: () => ipcRenderer.invoke(CH.cancelUpdate),
  setUpdateSettings: (patch) => ipcRenderer.invoke(CH.updateSettings, patch),
  skipVersion: (version) => ipcRenderer.invoke(CH.skipVersion, version),
  openWhatsNew: (version) => ipcRenderer.invoke(CH.whatsNew, version),
  getAppearance: () => ipcRenderer.invoke(CH.getAppearance),
  setAppearance: (theme) => ipcRenderer.invoke(CH.setAppearance, theme),
  onEvent: (cb) => {
    // main sends one tick's events as one message; Solid applies the writes of one task together
    const listener = (_: unknown, events: AppEvent[]) => {
      for (const e of events) cb(e);
    };
    ipcRenderer.on(CH.event, listener);
    return () => ipcRenderer.removeListener(CH.event, listener);
  },
};

contextBridge.exposeInMainWorld("api", api);
