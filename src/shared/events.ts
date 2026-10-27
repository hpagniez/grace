// Shared by main and renderer: keep this file free of Node imports.
import type { EnvView } from "./env";
import type { HistoryEntry } from "./history";
import type { HttpCall } from "./http";
import type { Layout, Status, TestNode } from "./model";
import type { FilterState } from "./tags";

export interface Step {
  /** Gherkin prefix as written: Given, When, Then, And, or `*`. */
  keyword: string;
  /** Karate keyword and its argument, e.g. `match response.message == 'x'`. */
  text: string;
  line: number;
  /** Karate's `result.status` as-is. */
  status: string;
  millis: number;
  background: boolean;
  log: string | null;
}
export interface ScenarioUpdate {
  key: string;
  status: Status;
  durationMillis?: number;
  error?: string;
}
export interface Summary {
  scenarioCount: number;
  scenariosPassed: number;
  scenariosFailed: number;
  durationMillis: number;
}
export type RunOutcome = "finished" | "cancelled" | "failed";
export interface Project {
  root: string;
  layout: Layout;
}
/** A saved recent folder and whether it is still a folder on disk. */
export interface RecentEntry {
  path: string;
  exists: boolean;
}
export type AppEvent =
  | { type: "project"; project: Project | null; tree: TestNode | null; error: string | null }
  | {
      type: "runStarted";
      selection: string;
      /** The run's folder name under `runs/`. */
      runId: string;
      keys: string[];
      total: number;
      startedAt: number;
      env: string | null;
    }
  | { type: "scenarios"; updates: ScenarioUpdate[] }
  | { type: "steps"; key: string; steps: Step[] }
  | { type: "calls"; key: string; calls: HttpCall[] }
  | { type: "console"; lines: string[] }
  | {
      type: "runFinished";
      outcome: RunOutcome;
      exitCode: number | null;
      summary: Summary | null;
      reportAvailable: boolean;
    }
  | { type: "notice"; message: string }
  /** The open project's tag filter, as main holds and applies it. */
  | { type: "filter"; filter: FilterState }
  /** The open project's thread count, as main holds it and passes to every run. */
  | { type: "threads"; count: number }
  /** The open project's environment, suggestions and run properties, as main holds them. */
  | { type: "environment"; view: EnvView }
  /** The open project's listed runs, newest first. */
  | { type: "history"; runs: HistoryEntry[] }
  /** The Runner is discovering or running; only Cancel makes sense meanwhile. */
  | { type: "busy"; busy: boolean }
  /** The folder main is opening, or null. Sent around each open and in every replay. */
  | { type: "opening"; root: string | null }
  /** The saved recent folders, newest first, each checked for existence. */
  | { type: "recent"; entries: RecentEntry[] };
