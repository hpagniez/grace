import type { AppEvent, ScenarioUpdate, Summary } from "../../shared/events";
import type { Status, TestNode } from "../../shared/model";
import type { KarateEvent } from "./events";
import { extractSteps } from "./steps";
import { leafIndex, leafKeys } from "./tree";

/** Turns Karate events into status updates for the scenarios and rows of one selection. */
export class RunState {
  readonly keys: string[];
  private readonly index: Map<string, TestNode>;
  private readonly status = new Map<string, Status>();
  private summary: Summary | null = null;
  private suiteExit = false;

  constructor(tree: TestNode, selection: TestNode) {
    this.keys = leafKeys(selection);
    this.index = leafIndex(tree);
  }

  private keyOf(file: string, refId: string): string | null {
    const key = this.index.get(`${file}|${refId}`)?.key;
    return key !== undefined && this.keys.includes(key) ? key : null;
  }

  /** At most one `scenarios` event (last status per key wins), then one `steps` event per scenario. */
  apply(events: KarateEvent[]): AppEvent[] {
    const updates = new Map<string, ScenarioUpdate>();
    const steps: AppEvent[] = [];
    for (const { type, data: d } of events) {
      if (type === "SUITE_EXIT") {
        this.suiteExit = true;
        const s = d.summary;
        this.summary = s
          ? {
              scenarioCount: s.scenarioCount,
              scenariosPassed: s.scenariosPassed,
              scenariosFailed: s.scenariosFailed,
              durationMillis: s.durationMillis,
            }
          : null;
      } else if (d.callDepth !== 0) {
        continue; // called features
      } else if (type === "SCENARIO_ENTER" || type === "SCENARIO_EXIT") {
        const key = this.keyOf(d.feature, d.refId);
        if (!key) continue;
        const update: ScenarioUpdate = { key, status: "running" };
        if (type === "SCENARIO_EXIT") {
          update.status = d.skipped ? "notRun" : d.passed ? "passed" : "failed";
          update.durationMillis = d.durationMillis;
          if (update.status === "failed" && d.error) update.error = d.error;
        }
        this.status.set(key, update.status);
        updates.set(key, update);
      } else if (type === "FEATURE_EXIT") {
        for (const sc of extractSteps(d)) {
          const key = this.keyOf(d.relativePath, sc.refId);
          if (key) steps.push({ type: "steps", key, steps: sc.steps });
        }
      }
    }
    return [
      ...(updates.size ? [{ type: "scenarios" as const, updates: [...updates.values()] }] : []),
      ...steps,
    ];
  }

  finish(exitCode: number | null, cancelled: boolean, reportAvailable: boolean): AppEvent[] {
    const outcome = cancelled
      ? "cancelled"
      : this.suiteExit && (exitCode === 0 || exitCode === 1)
        ? "finished"
        : "failed";
    const out: AppEvent[] = [];
    // A finished suite ran everything it was going to: a selected test with no SCENARIO_EXIT did not run.
    const unended =
      outcome === "finished"
        ? this.keys.filter((k) => (this.status.get(k) ?? "running") === "running")
        : [...this.status].filter(([, s]) => s === "running").map(([k]) => k);
    const status = outcome === "finished" ? "notRun" : "interrupted";
    if (unended.length)
      out.push({ type: "scenarios", updates: unended.map((key) => ({ key, status })) });
    out.push({ type: "runFinished", outcome, exitCode, summary: this.summary, reportAvailable });
    if (outcome === "finished" && unended.length) {
      const ran = this.keys.length - unended.length;
      out.push({
        type: "notice",
        message: `Karate ran ${ran} of ${this.keys.length} selected tests`,
      });
    }
    return out;
  }
}
