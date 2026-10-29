import type { EnvView } from "../shared/env";
import type { AppEvent, Project, RecentEntry, RunOutcome, Step, Summary } from "../shared/events";
import type { FlakyEnv } from "../shared/flaky";
import type { HistoryEntry } from "../shared/history";
import type { HttpCall, HttpGroup } from "../shared/http";
import type { Status, TestNode } from "../shared/model";
import { NO_FILTER, type FilterState } from "../shared/tags";
import type { Release, UpdateProgress, UpdateView } from "../shared/updates";
import type { CallRef } from "./http";

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
  /** Each test's HTTP calls, in run order; cleared exactly as `steps` is. */
  calls: Record<string, HttpCall[]>;
  console: string[];
  run: RunView;
  notice: string | null;
  selected: string | null;
  /** The open project's tag filter, as main holds it; changes only when main says so. */
  filter: FilterState;
  /** The open project's environment and run properties, as main holds them; null with no project. */
  env: EnvView | null;
  /**
   * The open project's thread count, as main holds it. A fresh object per `threads` event, so the
   * box resets even when main re-sends the count it held. Null with no project.
   */
  threads: { count: number } | null;
  /** The Runner holds its one-at-a-time slot: discovering, or running. */
  busy: boolean;
  /** The open project's listed runs, newest first, as main sends them. */
  history: HistoryEntry[];
  /** Proof per flaky test key of the tree shown (F15). Main's `flaky` event follows every tree and every history. */
  flaky: Record<string, FlakyEnv[]>;
  /** The status groups switched on in the HTTP tab; none means All. Lasts for the session. */
  httpGroups: HttpGroup[];
  /** The call shown in the details pane in place of the node; null shows the node. */
  selectedCall: CallRef | null;
  /** The tab shown in the bottom panel. */
  panel: "console" | "history" | "http";
  /** The folder main is opening, or null. */
  opening: string | null;
  /** Main has said what is open or opening; until then the window cannot tell home from a folder opening. */
  loaded: boolean;
  /** The saved recent folders, newest first; null until main sends them. */
  recent: RecentEntry[] | null;
  /** What main holds about updates; null until main sends it. */
  updates: UpdateView | null;
  /** The release the startup modal offers, until it closes. */
  updatePrompt: Release | null;
  /** The install in progress or its failure. */
  updateProgress: UpdateProgress | null;
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
    calls: {},
    console: [],
    run: idleRun(),
    notice: null,
    selected: null,
    filter: noFilter(),
    env: null,
    threads: null,
    busy: false,
    history: [],
    flaky: {},
    httpGroups: [],
    selectedCall: null,
    panel: "console",
    opening: null,
    loaded: false,
    recent: null,
    updates: null,
    updatePrompt: null,
    updateProgress: null,
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
          calls: {},
          selected: null,
          selectedCall: null,
          run: idleRun(),
          filter: noFilter(), // main's `filter` event follows
          env: null, // and its `environment` event
          threads: null, // main's `threads` event follows
          history: [], // main's `history` follows a project that opens
          flaky: {}, // and its `flaky` follows the tree
        });
        if (!e.project) s.console = []; // a closed project leaves no console behind
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
      if (s.selectedCall && !same(s.selectedCall.key)) s.selectedCall = null;
      for (const map of [s.statuses, s.results, s.steps, s.calls, s.flaky] as Record<
        string,
        unknown
      >[])
        for (const key of Object.keys(map)) if (!same(key)) delete map[key];
      s.project = e.project;
      s.tree = e.tree;
      s.error = e.error;
      s.notice = null;
      s.loaded = true;
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
      // the bottom tab stays as it is, so the HTTP tab can be watched filling during the run
      if (s.selectedCall && e.keys.includes(s.selectedCall.key)) s.selectedCall = null;
      for (const key of e.keys) {
        s.statuses[key] = "pending";
        delete s.results[key];
        delete s.steps[key];
        delete s.calls[key];
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
    case "calls":
      s.calls[e.key] = e.calls;
      break;
    case "console":
      s.console = s.console.concat(e.lines).slice(-CONSOLE_LIMIT);
      break;
    case "runFinished":
      s.run.active = false;
      s.run.outcome = e.outcome;
      s.run.summary = e.summary;
      s.run.reportAvailable = e.reportAvailable;
      s.busy = false; // nothing follows a run; avoids "Finding tests…" until the busy event arrives
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
    case "flaky":
      s.flaky = Object.fromEntries(e.tests.map((t) => [t.key, t.envs]));
      break;
    case "environment":
      if (!s.project) break; // main can still hold the previous project's view
      // a result from another environment must not pass for the state of this one
      if (s.env && s.env.env !== e.view.env)
        Object.assign(s, {
          statuses: {},
          results: {},
          steps: {},
          calls: {},
          selected: null,
          selectedCall: null,
          run: idleRun(),
        });
      s.env = e.view;
      break;
    case "threads":
      if (!s.project) break; // main can still hold the previous project's count
      s.threads = { count: e.count };
      break;
    case "busy":
      s.busy = e.busy;
      break;
    case "notice":
      s.notice = e.message;
      break;
    case "opening":
      s.opening = e.root;
      s.loaded = true;
      if (e.root && !s.project) s.error = null; // the last open's "Folder not found" is not this one's
      break;
    case "recent":
      s.recent = e.entries;
      break;
    case "updates":
      s.updates = e.view;
      break;
    case "updatePrompt":
      s.updatePrompt = e.release;
      break;
    case "updateProgress":
      s.updateProgress = e.progress;
      break;
  }
}
