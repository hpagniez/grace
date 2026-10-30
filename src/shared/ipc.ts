// Shared by main, preload and renderer: keep this file free of Node imports.
import type { PropertiesEdit } from "./env";
import type { AppEvent } from "./events";
import type { FilterState } from "./tags";
import type { Theme } from "./theme";
import type { Channel } from "./updates";

export const CH = {
  ready: "app:ready",
  open: "project:open",
  refresh: "project:refresh",
  run: "run:start",
  cancel: "run:cancel",
  report: "report:open",
  filter: "filter:set",
  threads: "threads:set",
  env: "env:set",
  properties: "properties:save",
  rerun: "history:rerun",
  rerunFailed: "history:rerunFailed",
  runReport: "history:report",
  clearHistory: "history:clear",
  openRecent: "recent:open",
  removeRecent: "recent:remove",
  close: "project:close",
  checkUpdates: "updates:check",
  installVersion: "updates:install",
  cancelUpdate: "updates:cancel",
  updateSettings: "updates:settings",
  skipVersion: "updates:skip",
  whatsNew: "updates:whatsNew",
  getAppearance: "appearance:get",
  setAppearance: "appearance:set",
  event: "app:event",
} as const;

export interface Api {
  ready(): Promise<void>;
  openProject(): Promise<void>;
  refresh(): Promise<void>;
  run(key: string): Promise<void>;
  cancel(): Promise<void>;
  openReport(): Promise<void>;
  setFilter(f: FilterState): Promise<void>;
  setThreads(n: number): Promise<void>;
  setEnv(name: string | null): Promise<void>;
  /** True once main applied the edit; false if it refused it (a notice says why). */
  saveProperties(edit: PropertiesEdit): Promise<boolean>;
  /** The history actions send only the run's folder id, never a path. */
  rerun(id: string): Promise<void>;
  rerunFailed(id: string): Promise<void>;
  openRunReport(id: string): Promise<void>;
  clearHistory(): Promise<void>;
  /** Both send a folder as saved in the recent list; main refuses any other. */
  openRecent(root: string): Promise<void>;
  removeRecent(root: string): Promise<void>;
  /** Closes the open project and goes back to the home screen. */
  closeProject(): Promise<void>;
  /** The update calls send versions and settings only; main checks each before acting (R2, R4, R6, R8). */
  checkUpdates(): Promise<void>;
  installVersion(version: string, useSavedData: boolean): Promise<void>;
  /** Stops the install under way, or closes a failed one's message. */
  cancelUpdate(): Promise<void>;
  setUpdateSettings(patch: {
    channel?: Channel;
    checkAtStartup?: boolean;
    autoUpdate?: boolean;
  }): Promise<void>;
  skipVersion(version: string): Promise<void>;
  /** Opens the version's release page in the default browser. */
  openWhatsNew(version: string): Promise<void>;
  /** The saved theme (spec R8); main applies it to the window. */
  getAppearance(): Promise<Theme>;
  /** Main refuses anything but a theme, and saves and applies one. */
  setAppearance(theme: Theme): Promise<void>;
  onEvent(cb: (e: AppEvent) => void): () => void;
}
