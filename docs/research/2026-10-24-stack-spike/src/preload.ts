import { contextBridge, ipcRenderer, type IpcRendererEvent } from 'electron';
import { IPC, type Api } from './ipc';

const api: Api = {
  getVersions: () => ipcRenderer.invoke(IPC.getVersions),
  onTick: (cb) => {
    const listener = (_e: IpcRendererEvent, count: number) => cb(count);
    ipcRenderer.on(IPC.tick, listener);
    return () => ipcRenderer.off(IPC.tick, listener);
  },
};

contextBridge.exposeInMainWorld('api', api);
