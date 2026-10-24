import type { AppEvent, Project, RunOutcome, Step, Summary } from "../shared/events";
import type { Status, TestNode } from "../shared/model";

export interface RunView {
  active: boolean;
  total: number;
  done: number;
  passed: number;
  failed: number;
  startedAt: number | null;
  outcome: RunOutcome | null;
  summary: Summary | null;
  reportAvailable: boolean;
}
export interface AppState {
  project: Project | null;
  tree: TestNode | null;
  error: string | null;
  statuses: Record<string, Status>;
  results: Record<string, { durationMillis?: number; error?: string }>;
  steps: Record<string, Step[]>;
  console: string[];
  run: RunView;
  notice: string | null;
  selected: string | null;
}

export const CONSOLE_LIMIT = 10_000;

const idleRun = (): RunView => ({
  active: false,
  total: 0,
  done: 0,
  passed: 0,
  failed: 0,
  startedAt: null,
  outcome: null,
  summary: null,
  reportAvailable: false,
});

export function initialState(): AppState {
  return {
    project: null,
    tree: null,
    error: null,
    statuses: {},
    results: {},
    steps: {},
    console: [],
    run: idleRun(),
    notice: null,
    selected: null,
  };
}

function collectKeys(n: TestNode, out: Set<string>): Set<string> {
  out.add(n.key);
  for (const c of n.children) collectKeys(c, out);
  return out;
}

/** Applies one event to a store draft (or a plain state object). */
export function reduce(s: AppState, e: AppEvent): void {
  switch (e.type) {
    case "project": {
      s.project = e.project;
      s.tree = e.tree;
      s.error = e.error;
      s.notice = null;
      const live = e.tree ? collectKeys(e.tree, new Set()) : new Set<string>();
      if (s.selected && !live.has(s.selected)) s.selected = null;
      for (const map of [s.statuses, s.results, s.steps] as Record<string, unknown>[])
        for (const key of Object.keys(map)) if (!live.has(key)) delete map[key];
      break;
    }
    case "runStarted":
      s.run = { ...idleRun(), active: true, total: e.total, startedAt: e.startedAt };
      for (const key of e.keys) {
        s.statuses[key] = "pending";
        delete s.results[key];
        delete s.steps[key];
      }
      s.console = [];
      break;
    case "scenarios":
      for (const u of e.updates) {
        const before = s.statuses[u.key];
        const final = before === "passed" || before === "failed";
        s.statuses[u.key] = u.status;
        const r = s.results[u.key] ?? {};
        if (u.durationMillis !== undefined) r.durationMillis = u.durationMillis;
        if (u.error !== undefined) r.error = u.error;
        s.results[u.key] = r;
        if (!final && (u.status === "passed" || u.status === "failed")) {
          s.run.done++;
          if (u.status === "passed") s.run.passed++;
          else s.run.failed++;
        }
      }
      break;
    case "steps":
      s.steps[e.key] = e.steps;
      break;
    case "console":
      s.console = s.console.concat(e.lines).slice(-CONSOLE_LIMIT);
      break;
    case "runFinished":
      s.run.active = false;
      s.run.outcome = e.outcome;
      s.run.summary = e.summary;
      s.run.reportAvailable = e.reportAvailable;
      break;
    case "notice":
      s.notice = e.message;
      break;
  }
}
