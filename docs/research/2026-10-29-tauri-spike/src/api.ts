// Grace's typed Api (src/shared/ipc.ts) over Tauri 2 IPC, in place of src/preload.ts.
// Every method is one call of the single `api` command with its CH channel, like ipcRenderer.invoke.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AppEvent } from "../../../../src/shared/events";
import { CH, type Api } from "../../../../src/shared/ipc";

const call =
  <T = void>(channel: string) =>
  (...args: unknown[]) =>
    invoke<T>("api", { channel, args });

// listen() registers asynchronously. `ready` makes main replay its state, so it waits for every
// listener: otherwise the replay could reach the page before the listener exists and be lost.
let listening: Promise<unknown> = Promise.resolve();

export const api: Api = {
  ready: () => listening.then(() => invoke("api", { channel: CH.ready, args: [] })),
  openProject: call(CH.open),
  refresh: call(CH.refresh),
  run: call(CH.run),
  cancel: call(CH.cancel),
  openReport: call(CH.report),
  setFilter: call(CH.filter),
  setThreads: call(CH.threads),
  setEnv: call(CH.env),
  saveProperties: call<boolean>(CH.properties),
  rerun: call(CH.rerun),
  rerunFailed: call(CH.rerunFailed),
  openRunReport: call(CH.runReport),
  clearHistory: call(CH.clearHistory),
  openRecent: call(CH.openRecent),
  removeRecent: call(CH.removeRecent),
  closeProject: call(CH.close),
  onEvent: (cb) => {
    const off = listen<AppEvent>(CH.event, (e) => cb(e.payload));
    listening = Promise.all([listening, off]);
    return () => void off.then((f) => f());
  },
};
