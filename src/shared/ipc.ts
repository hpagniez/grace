// Shared by main, preload and renderer: keep this file free of Node imports.
import type { ImportResult, PropertiesEdit } from "./env";
import type { AppEvent, DebugCommand, FileStep } from "./events";
import type { FilterState } from "./tags";
import type { Theme } from "./theme";
import type { Channel } from "./updates";

export const CH = {
  ready: "app:ready",
  open: "project:open",
  refresh: "project:refresh",
  run: "run:start",
  debug: "run:debug",
  debugCommand: "debug:command",
  breakpoint: "debug:breakpoint",
  steps: "steps:get",
  openInEditor: "editor:open",
  editorMenu: "editor:menu",
  cancel: "run:cancel",
  report: "report:open",
  filter: "filter:set",
  threads: "threads:set",
  watch: "watch:set",
  env: "env:set",
  properties: "properties:save",
  importProperties: "properties:import",
  rerun: "history:rerun",
  rerunFailed: "history:rerunFailed",
  runReport: "history:report",
  exportReport: "history:export",
  clearHistory: "history:clear",
  openRecent: "recent:open",
  openSubfolder: "project:openSubfolder",
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
  /** A run on one thread with the step debugger (docs/specs/2026-11-02-step-debugger.md). */
  debug(key: string): Promise<void>;
  debugCommand(c: DebugCommand): Promise<void>;
  /** Adds the breakpoint, or removes it if set; main sends the new set as a `breakpoints` event. */
  toggleBreakpoint(file: string, line: number): Promise<void>;
  /** A scenario's or row's steps as its feature file has them; null for any other key. */
  steps(key: string): Promise<FileStep[] | null>;
  /** Opens a tree feature path or an absolute path at the line in VS Code; main refuses any file outside the open project (docs/specs/2026-11-03-open-in-editor.md). */
  openInEditor(file: string, line: number): Promise<void>;
  /** Shows Windows' menu with "Open in VS Code" at the point, in window pixels; choosing it does what `openInEditor` does (docs/specs/2026-11-03-open-in-editor-from-tree.md). */
  editorMenu(file: string, line: number, x: number, y: number): Promise<void>;
  cancel(): Promise<void>;
  openReport(): Promise<void>;
  setFilter(f: FilterState): Promise<void>;
  setThreads(n: number): Promise<void>;
  /** Turns watch mode on or off for the open project; main sends the choice in force as a `watch` event (docs/specs/2026-11-03-watch-mode.md). */
  setWatch(on: boolean): Promise<void>;
  setEnv(name: string | null): Promise<void>;
  /** True once main applied the edit; false if it refused it (a notice says why). */
  saveProperties(edit: PropertiesEdit): Promise<boolean>;
  /** Main asks for a `.properties` file and reads it once (docs/specs/2026-11-04-env-var-properties.md R4). */
  importProperties(): Promise<ImportResult>;
  /** The history actions send only the run's folder id, never a path. */
  rerun(id: string): Promise<void>;
  rerunFailed(id: string): Promise<void>;
  openRunReport(id: string): Promise<void>;
  /** Main asks where to save the run's report as a zip (docs/specs/2026-11-03-export-reports.md). */
  exportReport(id: string): Promise<void>;
  clearHistory(): Promise<void>;
  /** Both send a folder as saved in the recent list; main refuses any other. */
  openRecent(root: string): Promise<void>;
  /** Sends a folder as the last `project` event offered it; main refuses any other. */
  openSubfolder(folder: string): Promise<void>;
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
