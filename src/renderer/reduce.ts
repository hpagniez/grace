import type { EnvView } from "../shared/env";
import type { AppEvent, Project, RunOutcome, Step, Summary } from "../shared/events";
import type { HistoryEntry } from "../shared/history";
import type { Status, TestNode } from "../shared/model";
import { NO_FILTER, type FilterState } from "../shared/tags";

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
  /** The run folder's name, the key of its entry in the history. */
  runId: string | null;
  /** The environment the run in progress, or the last run, used. */
  env: string | null;
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
  /** The open project's tag filter, as main holds it; changes only when main says so. */
  filter: FilterState;
  /** The open project's environment and run properties, as main holds them; null with no project. */
  env: EnvView | null;
  /** The Runner holds its one-at-a-time slot: discovering, or running. */
  busy: boolean;
  /** The open project's listed runs, newest first, as main sends them. */
  history: HistoryEntry[];
  /** The tab shown in the bottom panel. */
  panel: "console" | "history";
}

export const CONSOLE_LIMIT = 10_000;

// A fresh copy: a store draft must never write through to the shared constant.
const noFilter = (): FilterState => ({ ...NO_FILTER, include: [], exclude: [] });

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
  runId: null,
  env: null,
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
    filter: noFilter(),
    env: null,
    busy: false,
    history: [],
    panel: "console",
  };
}

function byKey(n: TestNode | null, out = new Map<string, TestNode>()): Map<string, TestNode> {
  if (!n) return out;
  out.set(n.key, n);
  for (const c of n.children) byKey(c, out);
  return out;
}

/** Applies one event to a store draft (or a plain state object). */
export function reduce(s: AppState, e: AppEvent): void {
  switch (e.type) {
    case "project": {
      if (!e.project || e.project.root !== s.project?.root) {
        Object.assign(s, {
          statuses: {},
          results: {},
          steps: {},
          selected: null,
          run: idleRun(),
          filter: noFilter(), // main's `filter` event follows
          env: null, // and its `environment` event
          history: [], // main's `history` follows a project that opens
        });
      }
      // A key only says where a test sits: inserting a scenario gives the next one its key.
      const before = byKey(s.tree);
      const after = byKey(e.tree);
      const same = (key: string) => {
        const a = before.get(key);
        const b = after.get(key);
        return !!a && !!b && a.kind === b.kind && a.name === b.name;
      };
      if (s.selected && !same(s.selected)) s.selected = null;
      for (const map of [s.statuses, s.results, s.steps] as Record<string, unknown>[])
        for (const key of Object.keys(map)) if (!same(key)) delete map[key];
      s.project = e.project;
      s.tree = e.tree;
      s.error = e.error;
      s.notice = null;
      break;
    }
    case "runStarted":
      s.run = {
        ...idleRun(),
        active: true,
        total: e.total,
        startedAt: e.startedAt,
        runId: e.runId,
        env: e.env,
      };
      s.panel = "console";
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
      s.busy = false; // nothing follows a run; avoids "Preparing…" until the busy event arrives
      break;
    case "filter":
      s.filter = e.filter;
      break;
    case "history":
      s.history = e.runs;
      // the toolbar Report opens a run's report: it needs the run to still be listed with one
      if (!e.runs.some((r) => r.id === s.run.runId && r.reportAvailable))
        s.run.reportAvailable = false;
      break;
    case "environment":
      if (!s.project) break; // main can still hold the previous project's view
      // a result from another environment must not pass for the state of this one
      if (s.env && s.env.env !== e.view.env)
        Object.assign(s, { statuses: {}, results: {}, steps: {}, selected: null, run: idleRun() });
      s.env = e.view;
      break;
    case "busy":
      s.busy = e.busy;
      break;
    case "notice":
      s.notice = e.message;
      break;
  }
}
