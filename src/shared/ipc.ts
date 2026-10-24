// Shared by main, preload and renderer: keep this file free of Node imports.
import type { AppEvent } from "./events";

export const CH = {
  ready: "app:ready",
  open: "project:open",
  refresh: "project:refresh",
  run: "run:start",
  cancel: "run:cancel",
  report: "report:open",
  event: "app:event",
} as const;

export interface Api {
  ready(): Promise<void>;
  openProject(): Promise<void>;
  refresh(): Promise<void>;
  run(key: string): Promise<void>;
  cancel(): Promise<void>;
  openReport(): Promise<void>;
  onEvent(cb: (e: AppEvent) => void): () => void;
}
