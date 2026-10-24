import path from "node:path";
import { describe, expect, it } from "vitest";
import type { AppEvent, ScenarioUpdate } from "../../shared/events";
import { NO_FILTER } from "../../shared/tags";
import { readAllEvents } from "./events";
import { RunState } from "./run-state";
import { buildTree } from "./tree";

const events = (name: string) => readAllEvents(path.resolve("test/fixtures/events", name));
const tree = buildTree(events("dryrun-clean.jsonl"), "tests");
const newState = () => new RunState(tree, tree, NO_FILTER);
const smokeNotWip = { ...NO_FILTER, include: ["smoke"], exclude: ["wip"] };
const updatesOf = (out: AppEvent[]): ScenarioUpdate[] =>
  out.flatMap((e) => (e.type === "scenarios" ? e.updates : []));

describe("RunState", () => {
  it("marks running then passed or failed", () => {
    const state = newState();
    const final = new Map<string, ScenarioUpdate>();
    for (const u of updatesOf(state.apply(events("run-clean.jsonl")))) final.set(u.key, u);
    const statuses = [...final.values()].map((u) => u.status);
    expect(statuses.filter((s) => s === "passed")).toHaveLength(8);
    expect(statuses.filter((s) => s === "failed")).toHaveLength(1);
    const failed = [...final.values()].find((u) => u.status === "failed")!;
    expect(failed.key).toBe("s:tests/basic.feature#2");
    expect(failed.error).toContain("basic.feature:20");
  });

  it("reports running for an entered scenario", () => {
    const enter = events("run-clean.jsonl").find(
      (e) => e.type === "SCENARIO_ENTER" && e.data.callDepth === 0,
    )!;
    expect(updatesOf(newState().apply([enter]))).toEqual([
      { key: "s:tests/basic.feature#1", status: "running" },
    ]);
  });

  it("sends at most one scenarios event per call", () => {
    const out = newState().apply(events("run-clean.jsonl"));
    expect(out.filter((e) => e.type === "scenarios")).toHaveLength(1);
  });

  it("ignores called features", () => {
    const keys = updatesOf(newState().apply(events("run-clean.jsonl"))).map((u) => u.key);
    expect(keys.some((k) => k.includes("common/create-user") || k.includes("start-mock"))).toBe(
      false,
    );
  });

  it("emits steps when a feature ends", () => {
    const out = newState().apply(events("run-clean.jsonl"));
    const steps = out.filter((e) => e.type === "steps");
    const failing = steps.find((e) => e.key === "s:tests/basic.feature#2");
    expect(failing?.steps.some((s) => s.status === "failed")).toBe(true);
    expect(steps.filter((e) => e.key.startsWith("s:tests/basic.feature#"))).toHaveLength(4);
  });

  // The spike sample (taskkill during tests/slow.feature) has the same slow.feature refIds as the clean tree.
  it("finish after cancel interrupts running scenarios", () => {
    const state = newState();
    state.apply(events("4-after-taskkill-complete-file.jsonl"));
    const out = state.finish(1, true, false);
    expect(updatesOf(out)).toEqual([{ key: "s:tests/slow.feature#1", status: "interrupted" }]);
    expect(out.at(-1)).toMatchObject({
      type: "runFinished",
      outcome: "cancelled",
      exitCode: 1,
      summary: null,
    });
  });

  it("finish without SUITE_EXIT is a failed run", () => {
    const out = newState().finish(3, false, false);
    expect(out).toEqual([
      {
        type: "runFinished",
        outcome: "failed",
        exitCode: 3,
        summary: null,
        reportAvailable: false,
      },
    ]);
  });

  it("finish with SUITE_EXIT carries the summary", () => {
    const state = newState();
    state.apply(events("run-clean.jsonl"));
    const out = state.finish(1, false, true);
    expect(out).toHaveLength(1);
    expect(out[0]).toMatchObject({
      outcome: "finished",
      reportAvailable: true,
      summary: { scenarioCount: 9, scenariosPassed: 8, scenariosFailed: 1 },
    });
  });

  it("finish marks selected tests Karate never ended as not run", () => {
    const run = events("run-clean.jsonl");
    const enter = run.find((e) => e.type === "SCENARIO_ENTER" && e.data.callDepth === 0)!;
    const state = newState();
    state.apply([enter, run.find((e) => e.type === "SUITE_EXIT")!]);
    const out = state.finish(0, false, true);
    expect(updatesOf(out)).toEqual(state.keys.map((key) => ({ key, status: "notRun" })));
    expect(out.at(-1)).toEqual({ type: "notice", message: "Karate ran 0 of 9 selected tests" });
    expect(out.find((e) => e.type === "runFinished")).toMatchObject({ outcome: "finished" });
  });

  it("keys are the matching leaves", () => {
    expect(new RunState(tree, tree, smokeNotWip).keys).toHaveLength(2);
  });

  it("warns when Karate ran more than selected", () => {
    const state = new RunState(tree, tree, smokeNotWip);
    state.apply(events("run-clean.jsonl"));
    expect(state.finish(1, false, true).at(-1)).toEqual({
      type: "notice",
      message: "Karate ran 9 tests, more than the 2 selected",
    });
  });

  it("no over-run notice for a matching count", () => {
    const state = newState();
    state.apply(events("run-clean.jsonl"));
    expect(state.finish(1, false, true).some((e) => e.type === "notice")).toBe(false);
  });
});
