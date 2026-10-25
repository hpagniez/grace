// Shared by main, preload and renderer: keep this file free of Node imports.
import type { PropertiesEdit } from "./env";
import type { AppEvent } from "./events";
import type { FilterState } from "./tags";

export const CH = {
  ready: "app:ready",
  open: "project:open",
  refresh: "project:refresh",
  run: "run:start",
  cancel: "run:cancel",
  report: "report:open",
  filter: "filter:set",
  env: "env:set",
  properties: "properties:save",
  rerun: "history:rerun",
  rerunFailed: "history:rerunFailed",
  runReport: "history:report",
  clearHistory: "history:clear",
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
  setEnv(name: string | null): Promise<void>;
  /** True once main applied the edit; false if it refused it (a notice says why). */
  saveProperties(edit: PropertiesEdit): Promise<boolean>;
  /** The history actions send only the run's folder id, never a path. */
  rerun(id: string): Promise<void>;
  rerunFailed(id: string): Promise<void>;
  openRunReport(id: string): Promise<void>;
  clearHistory(): Promise<void>;
  onEvent(cb: (e: AppEvent) => void): () => void;
}
