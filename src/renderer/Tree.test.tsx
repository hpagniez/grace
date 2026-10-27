import { render, screen, cleanup, fireEvent, within } from "@solidjs/testing-library";
import { flush } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AppEvent } from "../shared/events";
import { FLAKY_TITLE, type FlakyEnv } from "../shared/flaky";
import type { Api } from "../shared/ipc";
import type { TestNode } from "../shared/model";
import { App } from "./App";

const node = (
  kind: TestNode["kind"],
  key: string,
  name: string,
  extra: Partial<TestNode> = {},
): TestNode => ({
  key,
  kind,
  name,
  file: "tests/x.feature",
  line: null,
  refId: null,
  tags: [],
  children: [],
  ...extra,
});

const row = (n: number, name: string) =>
  node("row", `s:tests/outline.feature#1.${n}`, name, { line: 5 + n });
const tree = (): TestNode =>
  node("folder", "d:", "clean", {
    children: [
      node("folder", "d:tests", "tests", {
        children: [
          node("feature", "f:tests/basic.feature", "basic API checks", {
            file: "tests/basic.feature",
            children: [
              node("scenario", "s:tests/basic.feature#1", "passing hello", {
                line: 6,
                tags: ["smoke"],
              }),
              node("scenario", "s:tests/basic.feature#2", "failing match", { line: 14 }),
            ],
          }),
          node("feature", "f:tests/outline.feature", "outline with examples", {
            file: "tests/outline.feature",
            children: [
              node("outline", "o:tests/outline.feature#1", "add <a> and <b>", {
                children: [row(1, "add 1 and 2"), row(2, "add 2 and 3")],
              }),
            ],
          }),
        ],
      }),
    ],
  });

function setup() {
  let emit: (e: AppEvent) => void = () => {};
  const api = {
    ready: vi.fn(async () => {}),
    openProject: vi.fn(async () => {}),
    refresh: vi.fn(async () => {}),
    run: vi.fn(async () => {}),
    cancel: vi.fn(async () => {}),
    openReport: vi.fn(async () => {}),
    setFilter: vi.fn(async () => {}),
    setEnv: vi.fn(async () => {}),
    saveProperties: vi.fn(async () => true),
    onEvent: (cb: (e: AppEvent) => void) => ((emit = cb), () => {}),
  };
  render(() => <App api={api as unknown as Api} />);
  const send = (e: AppEvent) => {
    emit(e);
    flush();
  };
  const load = () =>
    send({
      type: "project",
      project: { root: "D:/p", layout: "standalone" },
      tree: tree(),
      error: null,
    });
  return { api, send, load };
}

const button = (id: string) => screen.getByTestId(id) as HTMLButtonElement;

afterEach(cleanup);

describe("main screen", () => {
  it("shows a failed scenario's status on its feature", () => {
    const { send, load } = setup();
    load();
    send({
      type: "runStarted",
      selection: "d:",
      runId: "20261025-120000-000",
      keys: ["s:tests/basic.feature#1", "s:tests/basic.feature#2"],
      total: 2,
      startedAt: Date.now(),
      env: null,
    });
    send({
      type: "scenarios",
      updates: [{ key: "s:tests/basic.feature#2", status: "failed", error: "boom" }],
    });
    const nodes = screen.getAllByTestId("tree-node");
    const byKey = (k: string) => nodes.find((n) => n.dataset.key === k)!;
    expect(nodes).toHaveLength(8);
    expect(byKey("s:tests/basic.feature#2").dataset.status).toBe("failed");
    expect(byKey("f:tests/basic.feature").dataset.status).toBe("failed");
    expect(byKey("s:tests/basic.feature#1").dataset.status).toBe("pending");
    expect(byKey("f:tests/outline.feature").dataset.status).toBe("notRun");
    expect(byKey("o:tests/outline.feature#1").dataset.kind).toBe("outline");
    expect(screen.getByText("smoke")).toBeTruthy();
  });

  it("runs a row from its run button", () => {
    const { api, load } = setup();
    load();
    fireEvent.click(screen.getByRole("button", { name: "Run add 2 and 3" }));
    expect(api.run).toHaveBeenCalledWith("s:tests/outline.feature#1.2");
  });

  it("gives every run button an accessible name", () => {
    const { load } = setup();
    load();
    const names = screen.getAllByTestId("run-node").map((b) => b.getAttribute("aria-label"));
    expect(names).toContain("Run failing match");
    expect(names).toContain("Run tests");
    expect(names.every((n) => n?.startsWith("Run "))).toBe(true);
  });

  it("labels a feature with its file name, then its title", () => {
    const { load } = setup();
    load();
    const feature = screen
      .getAllByTestId("tree-node")
      .find((n) => n.dataset.key === "f:tests/basic.feature")!;
    expect(feature.textContent).toContain("basic.feature");
    expect(feature.textContent!.indexOf("basic.feature")).toBeLessThan(
      feature.textContent!.indexOf("basic API checks"),
    );
    expect(screen.getByRole("button", { name: "Run basic.feature" })).toBeTruthy();
  });

  it("selects a node by clicking its label", () => {
    const { load } = setup();
    load();
    fireEvent.click(screen.getByText("failing match"));
    flush();
    const selected = screen.getByRole("treeitem", { selected: true });
    expect(selected.querySelector("[data-key]")!.getAttribute("data-key")).toBe(
      "s:tests/basic.feature#2",
    );
    expect(screen.getByTestId("details-location").textContent).toBe("tests/x.feature:14");
  });

  it("disables run buttons while a run is active and enables Cancel", () => {
    const { send, load } = setup();
    load();
    expect(button("run-all-button").disabled).toBe(false);
    expect(button("cancel-button").disabled).toBe(true);
    send({
      type: "runStarted",
      selection: "d:",
      runId: "20261025-120000-000",
      keys: [],
      total: 4,
      startedAt: Date.now(),
      env: null,
    });
    for (const id of ["run-all-button", "open-button", "refresh-button"])
      expect(button(id).disabled).toBe(true);
    for (const b of screen.getAllByTestId("run-node"))
      expect((b as HTMLButtonElement).disabled).toBe(true);
    expect(button("cancel-button").disabled).toBe(false);
    expect(screen.getByTestId("run-outcome").textContent).toBe("Running…");
  });

  it("shows discovery as busy: actions off, Cancel on, Preparing…", () => {
    const { send, load } = setup();
    load();
    send({ type: "busy", busy: true });
    for (const id of ["run-all-button", "open-button", "refresh-button"])
      expect(button(id).disabled).toBe(true);
    for (const b of screen.getAllByTestId("run-node"))
      expect((b as HTMLButtonElement).disabled).toBe(true);
    expect(button("cancel-button").disabled).toBe(false);
    expect(screen.getByTestId("run-outcome").textContent).toBe("Preparing…");
    send({ type: "busy", busy: false });
    expect(button("run-all-button").disabled).toBe(false);
    expect(button("cancel-button").disabled).toBe(true);
    expect(screen.getByTestId("run-outcome").textContent).toBe("");
  });

  it("labels an unnamed scenario by its line", () => {
    const { send } = setup();
    const unnamed = node("scenario", "s:tests/x.feature#1", null as unknown as string, {
      line: 3,
    });
    send({
      type: "project",
      project: { root: "D:/p", layout: "standalone" },
      tree: node("folder", "d:", "p", {
        children: [node("feature", "f:tests/x.feature", "x", { children: [unnamed] })],
      }),
      error: null,
    });
    expect(screen.getByRole("button", { name: "Run Scenario at line 3" })).toBeTruthy();
    fireEvent.click(screen.getByText("Scenario at line 3"));
    flush();
    expect(screen.getByRole("heading").textContent).toBe("Scenario at line 3");
  });

  it("disables Run all without a tree or without leaves, and says why", () => {
    const { send } = setup();
    expect(button("run-all-button").disabled).toBe(true);
    expect(screen.getByTestId("tree-message").textContent).toBe("Open a Karate project folder");
    send({
      type: "project",
      project: { root: "D:/p", layout: "maven" },
      tree: node("folder", "d:", "p"),
      error: null,
    });
    expect(button("run-all-button").disabled).toBe(true);
    expect(screen.getByTestId("tree-message").textContent).toBe("No feature files found");
    expect(screen.getByTestId("project-layout").textContent).toBe("Maven");
    send({
      type: "project",
      project: { root: "D:/p", layout: "maven" },
      tree: null,
      error: "bad syntax",
    });
    expect(screen.getByTestId("tree-message").textContent).toBe("bad syntax");
  });

  it("shows steps once they arrive, with their log on demand", () => {
    const { send, load } = setup();
    load();
    fireEvent.click(screen.getByText("passing hello"));
    flush();
    expect(screen.getByTestId("details").textContent).toContain(
      "Steps are available when the feature finishes",
    );
    send({
      type: "steps",
      key: "s:tests/basic.feature#1",
      steps: [
        {
          keyword: "Given",
          text: "path 'hello'",
          line: 7,
          status: "passed",
          millis: 3,
          background: false,
          log: null,
        },
        {
          keyword: "*",
          text: "print 'x'",
          line: 8,
          status: "passed",
          millis: 1,
          background: true,
          log: "x",
        },
      ],
    });
    const steps = screen.getAllByTestId("step");
    expect(steps.map((s) => s.dataset.status)).toEqual(["passed", "passed"]);
    expect(steps[0].textContent).toContain("Given path 'hello'");
    expect(screen.queryByTestId("step-log")).toBeNull();
    fireEvent.click(screen.getByTestId("step-toggle"));
    flush();
    expect(screen.getByTestId("step-log").textContent).toBe("x");
  });

  it("shows progress, outcome, elapsed and the notice", () => {
    const { send, load } = setup();
    load();
    send({
      type: "runStarted",
      selection: "d:",
      runId: "20261025-120000-000",
      keys: [],
      total: 4,
      startedAt: Date.now(),
      env: null,
    });
    send({
      type: "scenarios",
      updates: [
        { key: "a", status: "passed" },
        { key: "b", status: "failed" },
      ],
    });
    expect(screen.getByTestId("progress-text").textContent).toBe("2 / 4");
    expect(screen.getByTestId("progress-passed").textContent).toBe("1");
    expect(screen.getByTestId("progress-failed").textContent).toBe("1");
    send({
      type: "runFinished",
      outcome: "finished",
      exitCode: 1,
      summary: { scenarioCount: 4, scenariosPassed: 3, scenariosFailed: 1, durationMillis: 65_000 },
      reportAvailable: true,
    });
    expect(screen.getByTestId("run-outcome").textContent).toBe("Finished");
    expect(screen.getByTestId("progress-elapsed").textContent).toBe("01:05");
    expect(button("report-button").disabled).toBe(false);
    send({ type: "notice", message: "Nothing to run" });
    expect(screen.getByTestId("notice").textContent).toBe("Nothing to run");
  });
});

describe("flaky marks", () => {
  const SCENARIO = "s:tests/x.feature#1";
  const STEADY = "s:tests/x.feature#2";
  const ROW = "s:tests/x.feature#3.1";
  // a feature holding a flaky scenario, a steady one, and an outline with a flaky row
  const flakyTree = (): TestNode =>
    node("folder", "d:", "p", {
      children: [
        node("folder", "d:tests", "tests", {
          children: [
            node("feature", "f:tests/x.feature", "x", {
              file: "tests/x.feature",
              children: [
                node("scenario", SCENARIO, "coin", { line: 3 }),
                node("scenario", STEADY, "steady", { line: 6 }),
                node("outline", "o:tests/x.feature#3", "sum <a>", {
                  children: [
                    node("row", ROW, "sum 1", { line: 10 }),
                    node("row", "s:tests/x.feature#3.2", "sum 2", { line: 11 }),
                  ],
                }),
              ],
            }),
          ],
        }),
      ],
    });
  const proof: FlakyEnv[] = [{ env: null, passed: 1, failed: 1, runs: [] }];
  const rowOf = (key: string) =>
    screen.getAllByTestId("tree-node").find((n) => n.dataset.key === key)!;
  const markOf = (key: string) => within(rowOf(key)).queryByTestId("flaky-mark");
  const countOf = (key: string) => within(rowOf(key)).queryByTestId("flaky-count");

  it("marks flaky tests and counts them above", () => {
    const { send } = setup();
    send({
      type: "project",
      project: { root: "D:/p", layout: "standalone" },
      tree: flakyTree(),
      error: null,
    });
    send({
      type: "flaky",
      tests: [
        { key: SCENARIO, envs: proof },
        { key: ROW, envs: proof },
      ],
    });
    for (const key of [SCENARIO, ROW]) {
      const mark = markOf(key)!;
      expect(mark.textContent).toBe("flaky");
      expect(mark.title).toBe(FLAKY_TITLE);
      expect(mark.getAttribute("aria-label")).toBe(FLAKY_TITLE);
      expect(countOf(key)).toBeNull();
    }
    expect(countOf("o:tests/x.feature#3")!.textContent).toBe("1 flaky");
    expect(markOf("o:tests/x.feature#3")).toBeNull();
    expect(countOf("f:tests/x.feature")!.textContent).toBe("2 flaky");
    expect(countOf("d:tests")!.textContent).toBe("2 flaky");
    for (const key of [STEADY, "s:tests/x.feature#3.2"]) {
      expect(markOf(key)).toBeNull();
      expect(countOf(key)).toBeNull();
    }
    // the mark sits after the tags and before the run button
    const order = [...rowOf(SCENARIO).children].map((c) => (c as HTMLElement).dataset.testid);
    expect(order.indexOf("flaky-mark")).toBe(order.indexOf("run-node") - 1);
  });

  it("counts a flaky test the tag filter hides", () => {
    const { send, load } = setup();
    load();
    send({
      type: "filter",
      filter: { include: ["smoke"], exclude: [], hideNonMatching: true },
    });
    send({ type: "flaky", tests: [{ key: "s:tests/basic.feature#2", envs: proof }] });
    // "failing match" has no tag: the filter hides it, its feature still counts it
    expect(screen.queryByText("failing match")).toBeNull();
    expect(within(rowOf("f:tests/basic.feature")).getByTestId("flaky-count").textContent).toBe(
      "1 flaky",
    );
  });

  it("keeps the mark of a dimmed test", () => {
    const { send, load } = setup();
    load();
    send({
      type: "filter",
      filter: { include: ["smoke"], exclude: [], hideNonMatching: false },
    });
    send({ type: "flaky", tests: [{ key: "s:tests/basic.feature#2", envs: proof }] });
    expect(rowOf("s:tests/basic.feature#2").dataset.match).toBe("false");
    expect(markOf("s:tests/basic.feature#2")).toBeTruthy();
  });

  it("clears the marks for a new project", () => {
    const { send, load } = setup();
    load();
    send({ type: "flaky", tests: [{ key: "s:tests/basic.feature#2", envs: proof }] });
    expect(screen.getAllByTestId("flaky-mark")).toHaveLength(1);
    send({
      type: "project",
      project: { root: "D:/other", layout: "standalone" },
      tree: tree(),
      error: null,
    });
    expect(screen.queryByTestId("flaky-mark")).toBeNull();
    expect(screen.queryByTestId("flaky-count")).toBeNull();
  });

  it("drops the mark of a key that now holds another test, keeps the rest", () => {
    const { send, load } = setup();
    load();
    send({
      type: "flaky",
      tests: [
        { key: "s:tests/basic.feature#1", envs: proof },
        { key: "s:tests/basic.feature#2", envs: proof },
      ],
    });
    // the same tree but for a scenario renamed in place: its key now names another test
    const renamed = tree();
    renamed.children[0].children[0].children[1].name = "another test";
    send({
      type: "project",
      project: { root: "D:/p", layout: "standalone" },
      tree: renamed,
      error: null,
    });
    expect(markOf("s:tests/basic.feature#1")).toBeTruthy();
    expect(markOf("s:tests/basic.feature#2")).toBeNull();
    expect(countOf("f:tests/basic.feature")!.textContent).toBe("1 flaky");
  });
});
