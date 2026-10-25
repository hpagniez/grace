// Shared by main, preload and renderer: keep this file free of Node imports.
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
  onEvent(cb: (e: AppEvent) => void): () => void;
}
