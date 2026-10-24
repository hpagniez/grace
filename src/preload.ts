import { contextBridge, ipcRenderer } from "electron";
import type { AppEvent } from "./shared/events";
import { CH, type Api } from "./shared/ipc";

const api: Api = {
  ready: () => ipcRenderer.invoke(CH.ready),
  openProject: () => ipcRenderer.invoke(CH.open),
  refresh: () => ipcRenderer.invoke(CH.refresh),
  run: (key) => ipcRenderer.invoke(CH.run, key),
  cancel: () => ipcRenderer.invoke(CH.cancel),
  openReport: () => ipcRenderer.invoke(CH.report),
  onEvent: (cb) => {
    const listener = (_: unknown, e: AppEvent) => cb(e);
    ipcRenderer.on(CH.event, listener);
    return () => ipcRenderer.removeListener(CH.event, listener);
  },
};

contextBridge.exposeInMainWorld("api", api);
