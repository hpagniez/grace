import { render, screen, cleanup, fireEvent, within } from "@solidjs/testing-library";
import {
  ChevronDown,
  ChevronRight,
  FileSearch,
  FileText,
  Folder,
  FolderOpen,
  Shuffle,
} from "lucide";
import { flush } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AppEvent } from "../shared/events";
import { FLAKY_TITLE, type FlakyEnv } from "../shared/flaky";
import type { Api } from "../shared/ipc";
import type { TestNode } from "../shared/model";
import { NO_FILTER } from "../shared/tags";
import { App } from "./App";
import { treeRows } from "./treeRows";
import { drawn } from "../../test/drawn";

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
const rowByKey = (key: string) =>
  screen.getAllByTestId("tree-node").find((n) => n.dataset.key === key)!;

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

  it("shows each row's status by shape, named for a screen reader (R4.4)", () => {
    const { send, load } = setup();
    load();
    const mark = (key: string) => within(rowByKey(key)).getByRole("img");
    for (const n of screen.getAllByTestId("tree-node"))
      expect(within(n).getByRole("img").getAttribute("aria-label")).toBe("Not run");
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
      updates: [
        { key: "s:tests/basic.feature#1", status: "passed" },
        { key: "s:tests/basic.feature#2", status: "failed", error: "boom" },
      ],
    });
    expect(mark("s:tests/basic.feature#1").getAttribute("aria-label")).toBe("Passed");
    expect(mark("s:tests/basic.feature#2").getAttribute("aria-label")).toBe("Failed");
    expect(mark("f:tests/basic.feature").getAttribute("aria-label")).toBe("Failed");
    expect(mark("f:tests/outline.feature").getAttribute("aria-label")).toBe("Not run");
  });

  it("swaps the chevron when a folder opens and closes, and icons folders and features", () => {
    const { load } = setup();
    load();
    const row = () => rowByKey("d:tests"); // a toggle draws the row again
    const toggle = within(row()).getByRole("button", { name: "Collapse tests" });
    const chevron = toggle.querySelector("svg");
    expect(drawn(chevron, ChevronDown)).toBe(true);
    // no transform: one on every open row slows every hit test of a large tree
    expect(chevron!.getAttribute("class") ?? "").not.toContain("rotate");
    // svgs in a row: chevron, status mark, then the folder or file icon
    expect(drawn(row().querySelectorAll("svg")[2], FolderOpen)).toBe(true);
    expect(drawn(rowByKey("f:tests/basic.feature").querySelectorAll("svg")[2], FileText)).toBe(
      true,
    );
    // a leaf keeps the place of its chevron but hides it
    expect(rowByKey("s:tests/basic.feature#1").querySelector("button")!.className).toContain(
      "invisible",
    );
    fireEvent.click(toggle);
    flush();
    const closed = within(row()).getByRole("button", { name: "Expand tests" });
    expect(drawn(closed.querySelector("svg"), ChevronRight)).toBe(true);
    expect(drawn(row().querySelectorAll("svg")[2], Folder)).toBe(true);
    expect(row().parentElement!.getAttribute("aria-expanded")).toBe("false");
  });

  it("marks the selected row with its background and the accent bar, rings focus inside the row", () => {
    const { load } = setup();
    load();
    const row = rowByKey("s:tests/basic.feature#2");
    expect(row.className).toContain("h-6");
    expect(row.className).toContain("rounded-sm");
    expect(row.className).toContain("hover:bg-hover");
    expect(row.className).not.toContain("bg-selected");
    fireEvent.click(screen.getByText("failing match"));
    flush();
    expect(row.className).toContain("bg-selected");
    expect(row.className).toContain("shadow-[inset_2px_0_0_var(--color-accent)]");
    expect(rowByKey("s:tests/basic.feature#1").className).not.toContain("bg-selected");
    // the tree scrolls: the ring goes inside the row (R10.1)
    for (const b of row.querySelectorAll("button"))
      expect(b.className).toContain("focus-visible:-outline-offset-2");
  });

  it("shows the run button muted, green when its row is hovered, focused or selected", () => {
    const { load } = setup();
    load();
    const key = "s:tests/basic.feature#2";
    const run = (k: string) => within(rowByKey(k)).getByTestId("run-node");
    const green = /(^| )text-passed( |$)/;
    expect(rowByKey(key).className).toContain("group");
    expect(run(key).className).toContain("text-muted");
    expect(run(key).className).toContain("group-hover:text-passed");
    expect(run(key).className).toContain("group-focus-within:text-passed");
    expect(run(key).className).toContain("size-6"); // 24 x 24 (R10.3)
    expect(run(key).className).not.toMatch(green);
    expect(run(key).getAttribute("aria-label")).toBe("Run failing match");
    fireEvent.click(screen.getByText("failing match"));
    flush();
    expect(run(key).className).toMatch(green);
    expect(run("s:tests/basic.feature#1").className).not.toMatch(green);
  });

  describe("label tooltip", () => {
    // jsdom has no layout: say how wide the label is and how much of it shows
    const measure = (el: HTMLElement, scrollWidth: number, clientWidth: number) => {
      Object.defineProperty(el, "scrollWidth", { value: scrollWidth, configurable: true });
      Object.defineProperty(el, "clientWidth", { value: clientWidth, configurable: true });
    };
    const tip = () => document.querySelector("[role=tooltip]");
    const rest = (el: HTMLElement) => {
      fireEvent.pointerEnter(el.parentElement!);
      vi.advanceTimersByTime(500);
    };

    it("shows a cut label whole in a tooltip, not as a title", () => {
      vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
      try {
        const { load } = setup();
        load();
        const label = screen.getByText("failing match");
        expect(label.className).toContain("truncate");
        expect(label.hasAttribute("title")).toBe(false);
        expect(label.parentElement!.className).toContain("min-w-0");
        // measured when the tooltip would show, not once at mount
        measure(label, 200, 100);
        rest(label);
        expect(tip()!.textContent).toBe("failing match");
      } finally {
        vi.useRealTimers();
      }
    });

    it("shows nothing for a label that fits (R9.4: truncated text only)", () => {
      vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
      try {
        const { load } = setup();
        load();
        const label = screen.getByText("failing match");
        measure(label, 100, 100);
        rest(label);
        expect(tip()).toBeNull();
      } finally {
        vi.useRealTimers();
      }
    });

    it("keeps a row's tooltips on its row: the mark's right, the label's over it, the run's left", () => {
      vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
      // every box 24 x 24 in the middle of the window, so no side runs out of room
      vi.spyOn(Element.prototype, "getBoundingClientRect").mockReturnValue(
        DOMRect.fromRect({ x: 500, y: 300, width: 24, height: 24 }),
      );
      try {
        const { load } = setup();
        load();
        const row = rowByKey("s:tests/basic.feature#2");
        const label = screen.getByText("failing match");
        measure(label, 200, 100);
        const sideOf = (el: HTMLElement) => {
          rest(el);
          const side = (tip() as HTMLElement).dataset.side;
          fireEvent.pointerLeave(el.parentElement!);
          flush();
          return side;
        };
        expect(sideOf(within(row).getByRole("img"))).toBe("right");
        expect(sideOf(label)).toBe("over");
        expect(sideOf(within(row).getByTestId("run-node"))).toBe("left");
      } finally {
        vi.useRealTimers();
        vi.restoreAllMocks();
      }
    });
  });

  it("gives tags the 20 px tag chip, so they clear the rows above and below", () => {
    const { load } = setup();
    load();
    const tag = screen.getByText("smoke"); // the tree tag; the filter chip reads @smoke
    expect(tag.className).toContain("h-5");
    expect(tag.className).not.toContain("h-6");
    expect(tag.className).toContain("rounded-sm");
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

  it("shows discovery as busy: actions off, Cancel on, Finding tests…", () => {
    const { send, load } = setup();
    load();
    send({ type: "busy", busy: true });
    for (const id of ["run-all-button", "open-button", "refresh-button"])
      expect(button(id).disabled).toBe(true);
    for (const b of screen.getAllByTestId("run-node"))
      expect((b as HTMLButtonElement).disabled).toBe(true);
    expect(button("cancel-button").disabled).toBe(false);
    expect(screen.getByTestId("run-outcome").textContent).toBe("Finding tests…");
    expect(screen.getByTestId("progress-stripe")).toBeTruthy();
    expect(screen.getByTestId("progress-bar").getAttribute("aria-busy")).toBe("true");
    expect(screen.getByTestId("progress-bar").hasAttribute("aria-valuenow")).toBe(false);
    send({ type: "busy", busy: false });
    expect(button("run-all-button").disabled).toBe(false);
    expect(button("cancel-button").disabled).toBe(true);
    expect(screen.getByTestId("run-outcome").textContent).toBe("");
    expect(screen.queryByTestId("progress-stripe")).toBeNull();
    expect(screen.getByTestId("progress-bar").getAttribute("aria-busy")).toBe("false");
  });

  const startRun = (send: (e: AppEvent) => void) =>
    send({
      type: "runStarted",
      selection: "d:",
      runId: "20261025-120000-000",
      keys: [],
      total: 4,
      startedAt: Date.now(),
      env: null,
    });

  it("keeps the stripe until the first result, then reports progress", () => {
    const { send, load } = setup();
    load();
    startRun(send);
    expect(screen.getByTestId("progress-stripe")).toBeTruthy();
    send({ type: "scenarios", updates: [{ key: "s:tests/basic.feature#1", status: "running" }] });
    expect(screen.getByTestId("progress-stripe")).toBeTruthy();
    send({ type: "scenarios", updates: [{ key: "s:tests/basic.feature#1", status: "failed" }] });
    expect(screen.queryByTestId("progress-stripe")).toBeNull();
    const bar = screen.getByTestId("progress-bar");
    expect(bar.getAttribute("role")).toBe("progressbar");
    expect(bar.getAttribute("aria-label")).toBe("Run progress");
    expect(bar.getAttribute("aria-valuemin")).toBe("0");
    expect(bar.getAttribute("aria-valuemax")).toBe("4");
    expect(bar.getAttribute("aria-valuenow")).toBe("1");
    send({ type: "scenarios", updates: [{ key: "s:tests/basic.feature#2", status: "passed" }] });
    expect(bar.getAttribute("aria-valuenow")).toBe("2");
  });

  it("drops the stripe when a run is cancelled before any result", () => {
    const { send, load } = setup();
    load();
    startRun(send);
    send({
      type: "runFinished",
      outcome: "cancelled",
      exitCode: 1,
      summary: null,
      reportAvailable: false,
    });
    expect(screen.queryByTestId("progress-stripe")).toBeNull();
  });

  it("fills smoothly and keeps the stripe still under reduced motion", () => {
    const { send, load } = setup();
    load();
    send({ type: "busy", busy: true });
    const stripe = screen.getByTestId("progress-stripe");
    expect(stripe.className).toContain("animate-slide");
    expect(stripe.className).toContain("motion-reduce:animate-none");
    const segments = [...screen.getByTestId("progress-bar").children].filter((c) => c !== stripe);
    expect(segments).toHaveLength(3); // passed, failed, cancelled
    for (const s of segments) {
      expect(s.className).toContain("transition-[width]");
      expect(s.className).toContain("motion-reduce:transition-none");
    }
  });

  it("draws the stripe over the last run's results", () => {
    const { send, load } = setup();
    load();
    startRun(send);
    send({ type: "scenarios", updates: [{ key: "s:tests/basic.feature#1", status: "passed" }] });
    send({
      type: "runFinished",
      outcome: "finished",
      exitCode: 0,
      summary: null,
      reportAvailable: false,
    });
    send({ type: "busy", busy: true });
    // last of the positioned children paints on top: the old bar must not hide the stripe
    const bar = screen.getByTestId("progress-bar");
    expect(bar.lastElementChild).toBe(screen.getByTestId("progress-stripe"));
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
    send({ type: "opening", root: "D:/p" });
    expect(button("run-all-button").disabled).toBe(true);
    const loading = screen.getByTestId("tree-message");
    expect(loading.textContent).toBe("Finding tests…");
    expect(loading.parentElement!.querySelector("svg")!.getAttribute("class")).toContain(
      "animate-spin",
    );
    send({
      type: "project",
      project: { root: "D:/p", layout: "maven" },
      tree: node("folder", "d:", "p"),
      error: null,
    });
    expect(button("run-all-button").disabled).toBe(true);
    const empty = screen.getByTestId("tree-message");
    expect(empty.textContent).toBe("No feature files found");
    expect(drawn(empty.parentElement!.querySelector("svg"), FileSearch)).toBe(true);
    expect(empty.parentElement!.textContent).toContain(
      "Grace looks for .feature files under the project folder.",
    );
    expect(screen.getByTestId("project-layout").textContent).toBe("Maven");
    send({
      type: "project",
      project: { root: "D:/p", layout: "maven" },
      tree: null,
      error: "bad syntax",
    });
    const failed = screen.getByTestId("tree-message");
    expect(failed.textContent).toBe("bad syntax");
    expect(failed.getAttribute("role")).toBe("alert"); // a Notice of kind error
    expect(failed.className).toContain("bg-failed-tint");
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
                node("scenario", SCENARIO, "coin", { line: 3, tags: ["slow"] }),
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
      expect(mark.hasAttribute("title")).toBe(false);
      expect(mark.getAttribute("aria-label")).toBe(FLAKY_TITLE);
      expect(mark.className).toContain("bg-cancelled-tint");
      expect(drawn(mark.querySelector("svg"), Shuffle)).toBe(true);
      expect(mark.querySelector("svg")!.getAttribute("width")).toBe("14"); // R4.2
      expect(countOf(key)).toBeNull();
    }
    expect(countOf("o:tests/x.feature#3")!.textContent).toBe("1 flaky");
    expect(countOf("o:tests/x.feature#3")!.className).toContain("bg-cancelled-tint");
    expect(markOf("o:tests/x.feature#3")).toBeNull();
    expect(countOf("f:tests/x.feature")!.textContent).toBe("2 flaky");
    expect(countOf("d:tests")!.textContent).toBe("2 flaky");
    for (const key of [STEADY, "s:tests/x.feature#3.2"]) {
      expect(markOf(key)).toBeNull();
      expect(countOf(key)).toBeNull();
    }
    // the mark sits right after the tags and right before the run button
    const cells = [...rowOf(SCENARIO).querySelectorAll<HTMLElement>("span, button")];
    const at = (pred: (c: HTMLElement) => boolean) => cells.findIndex(pred);
    const tag = at((c) => c.textContent === "slow");
    const flaky = at((c) => c.dataset.testid === "flaky-mark");
    const run = at((c) => c.dataset.testid === "run-node");
    expect(tag).toBeGreaterThan(-1);
    expect(flaky).toBeGreaterThan(tag);
    expect(run).toBeGreaterThan(flaky);
  });

  it("shows the flaky explanation in a tooltip, left of the mark: below, it would cover the next row", () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    vi.spyOn(Element.prototype, "getBoundingClientRect").mockReturnValue(
      DOMRect.fromRect({ x: 500, y: 300, width: 24, height: 24 }),
    );
    try {
      const { send } = setup();
      send({
        type: "project",
        project: { root: "D:/p", layout: "standalone" },
        tree: flakyTree(),
        error: null,
      });
      send({ type: "flaky", tests: [{ key: SCENARIO, envs: proof }] });
      fireEvent.pointerEnter(markOf(SCENARIO)!.parentElement!);
      vi.advanceTimersByTime(500);
      const tip = document.querySelector<HTMLElement>("[role=tooltip]")!;
      expect(tip.textContent).toBe(FLAKY_TITLE);
      expect(tip.dataset.side).toBe("left");
    } finally {
      vi.useRealTimers();
      vi.restoreAllMocks();
    }
  });

  it("names the mark for a screen reader", () => {
    const { send } = setup();
    send({
      type: "project",
      project: { root: "D:/p", layout: "standalone" },
      tree: flakyTree(),
      error: null,
    });
    send({ type: "flaky", tests: [{ key: SCENARIO, envs: proof }] });
    expect(screen.getByRole("img", { name: FLAKY_TITLE })).toBe(markOf(SCENARIO));
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

// The virtual tree: only the rows in view are drawn, as one tab stop with arrow keys (spec 2026-10-30).
describe("virtual tree", () => {
  /** One folder of features of 10 scenarios each; even tests carry the tag `even`. */
  const bigTree = (tests: number): TestNode =>
    node("folder", "d:", "big", {
      children: [
        node("folder", "d:tests", "tests", {
          children: Array.from({ length: tests / 10 }, (_, f) =>
            node("feature", `f:tests/f${f}.feature`, `f${f}`, {
              file: `tests/f${f}.feature`,
              children: Array.from({ length: 10 }, (_, s) => {
                const i = f * 10 + s;
                return node("scenario", `s:tests/f${f}.feature#${s + 1}`, `test ${i}`, {
                  line: s + 1,
                  tags: i % 2 ? [] : ["even"],
                });
              }),
            }),
          ),
        }),
      ],
    });
  /** The keys of the rows a person can scroll to, all open and unfiltered. */
  const order = (t: TestNode) => treeRows(t, new Set(), NO_FILTER).map((r) => r.node.key);
  const open = (send: (e: AppEvent) => void, t: TestNode, root = "D:/p") =>
    send({ type: "project", project: { root, layout: "standalone" }, tree: t, error: null });
  const startRun = (send: (e: AppEvent) => void) =>
    send({
      type: "runStarted",
      selection: "d:",
      runId: "20261030-120000-000",
      keys: [],
      total: 4,
      startedAt: Date.now(),
      env: null,
    });

  const items = () => [
    ...screen.getByTestId("tree").querySelectorAll<HTMLElement>("[role=treeitem]"),
  ];
  const item = (key: string) => items().find((r) => r.dataset.key === key);
  /** The drawn rows top to bottom: the focus row is first in the DOM, wherever it sits. */
  const shown = () =>
    items()
      .sort((a, b) => parseFloat(a.style.top) - parseFloat(b.style.top))
      .map((r) => r.dataset.key!);
  const scroller = () => screen.getByTestId("tree").parentElement!;
  const scrollTo = (top: number) => {
    scroller().scrollTop = top;
    scroller().dispatchEvent(new Event("scroll"));
    flush();
  };
  /** jsdom has no layout: the tree reads a 600 px view. */
  const expectInView = (key: string) => {
    const top = parseFloat(item(key)!.style.top);
    expect(top).toBeGreaterThanOrEqual(scroller().scrollTop);
    expect(top + 24).toBeLessThanOrEqual(scroller().scrollTop + 600);
  };
  const focusRow = (key: string) => {
    item(key)!.focus();
    flush();
  };
  /** A key on the focused element; returns the event. */
  const press = (key: string, init: KeyboardEventInit = {}) => {
    const e = new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...init });
    document.activeElement!.dispatchEvent(e);
    flush();
    return e;
  };
  const focused = () => (document.activeElement as HTMLElement).dataset.key;
  const selected = () => items().filter((r) => r.getAttribute("aria-selected") === "true");
  const chevron = (key: string) => within(rowByKey(key)).getAllByRole("button")[0];
  const click = (el: HTMLElement) => {
    fireEvent.click(el);
    flush();
  };
  const heading = () => screen.getByRole("heading").textContent;
  /** What Tab can reach inside the tree. */
  const tabbable = () =>
    [...screen.getByTestId("tree").querySelectorAll<HTMLElement>("button, [tabindex]")].filter(
      (el) => el.tabIndex >= 0 && !(el as HTMLButtonElement).disabled,
    );

  describe("drawing (R1, R2)", () => {
    it("draws the first 25 rows of a project of 30 tests", () => {
      const { send } = setup();
      const t = bigTree(30);
      open(send, t);
      const drawnKeys = items().map((r) => r.dataset.key);
      for (const key of order(t).slice(0, 25)) expect(drawnKeys).toContain(key);
    });

    it("shows the same rows, order and indents as before, and only matches under Hide non-matching", () => {
      const { send, load } = setup();
      load();
      expect(shown()).toEqual([
        "d:tests",
        "f:tests/basic.feature",
        "s:tests/basic.feature#1",
        "s:tests/basic.feature#2",
        "f:tests/outline.feature",
        "o:tests/outline.feature#1",
        "s:tests/outline.feature#1.1",
        "s:tests/outline.feature#1.2",
      ]);
      // 16 px per level, as the nested lists gave
      expect(shown().map((k) => rowByKey(k).style.marginLeft)).toEqual([
        "0px",
        "16px",
        "32px",
        "32px",
        "16px",
        "32px",
        "48px",
        "48px",
      ]);
      expect(items().every((r) => r.style.height === "24px")).toBe(true);
      send({ type: "filter", filter: { include: ["smoke"], exclude: [], hideNonMatching: true } });
      expect(shown()).toEqual(["d:tests", "f:tests/basic.feature", "s:tests/basic.feature#1"]);
    });

    it("keeps closed folders through a refresh, and opens them for another project", () => {
      const { send, load } = setup();
      load();
      click(chevron("f:tests/basic.feature"));
      expect(item("f:tests/basic.feature")!.getAttribute("aria-expanded")).toBe("false");
      expect(item("s:tests/basic.feature#1")).toBeUndefined();
      load(); // a refresh: the same project, a new tree
      expect(item("f:tests/basic.feature")!.getAttribute("aria-expanded")).toBe("false");
      expect(item("s:tests/basic.feature#1")).toBeUndefined();
      open(send, tree(), "D:/other");
      load();
      expect(item("f:tests/basic.feature")!.getAttribute("aria-expanded")).toBe("true");
      expect(item("s:tests/basic.feature#1")).toBeDefined();
    });

    it("starts everything open in another project", () => {
      const { send, load } = setup();
      load();
      click(chevron("f:tests/basic.feature"));
      open(send, tree(), "D:/other");
      expect(item("f:tests/basic.feature")!.getAttribute("aria-expanded")).toBe("true");
    });

    it("keeps the scroll position and the drawn rows when a run updates 500 statuses", () => {
      const { send } = setup();
      const t = bigTree(2000);
      open(send, t);
      scrollTo(26_408); // half way
      const before = shown();
      const nodes = screen.getAllByTestId("tree-node");
      const tests = order(t)
        .filter((k) => k.startsWith("s:"))
        .slice(800, 1300); // tests 800 to 1299, the drawn ones among them
      send({
        type: "runStarted",
        selection: "d:",
        runId: "20261030-120000-000",
        keys: tests,
        total: tests.length,
        startedAt: Date.now(),
        env: null,
      });
      send({ type: "scenarios", updates: tests.map((key) => ({ key, status: "passed" })) });
      expect(rowByKey("s:tests/f100.feature#1").dataset.status).toBe("passed");
      expect(scroller().scrollTop).toBe(26_408);
      expect(shown()).toEqual(before);
      expect(screen.getAllByTestId("tree-node")).toEqual(nodes); // not one row drawn again
    });
  });

  describe("keys (R3)", () => {
    it("is one tab stop: the first row, with nothing selected", () => {
      const { load } = setup();
      load();
      expect(tabbable()).toEqual([item("d:tests")]);
    });

    it("moves and selects with Down, and the details pane follows", () => {
      const { load } = setup();
      load();
      focusRow("d:tests");
      press("ArrowDown");
      expect(focused()).toBe("f:tests/basic.feature");
      expect(selected()).toEqual([item("f:tests/basic.feature")]);
      expect(heading()).toBe("basic API checks");
    });

    describe("Right and Left", () => {
      it("opens a closed feature with Right, focus staying on it", () => {
        const { load } = setup();
        load();
        click(chevron("f:tests/basic.feature"));
        focusRow("f:tests/basic.feature");
        expect(press("ArrowRight").defaultPrevented).toBe(true);
        expect(item("f:tests/basic.feature")!.getAttribute("aria-expanded")).toBe("true");
        expect(focused()).toBe("f:tests/basic.feature");
        expect(item("s:tests/basic.feature#1")).toBeDefined();
      });

      it("moves to the first scenario of an open feature with Right, and selects it", () => {
        const { load } = setup();
        load();
        focusRow("f:tests/basic.feature");
        press("ArrowRight");
        expect(focused()).toBe("s:tests/basic.feature#1");
        expect(selected()).toEqual([item("s:tests/basic.feature#1")]);
      });

      it("closes an open feature with Left, focus staying on it", () => {
        const { load } = setup();
        load();
        focusRow("f:tests/basic.feature");
        press("ArrowLeft");
        expect(item("f:tests/basic.feature")!.getAttribute("aria-expanded")).toBe("false");
        expect(item("s:tests/basic.feature#1")).toBeUndefined();
        expect(focused()).toBe("f:tests/basic.feature");
      });

      it("moves from a scenario to its feature with Left, and selects it", () => {
        const { load } = setup();
        load();
        focusRow("s:tests/basic.feature#2");
        press("ArrowLeft");
        expect(focused()).toBe("f:tests/basic.feature");
        expect(selected()).toEqual([item("f:tests/basic.feature")]);
      });

      it("does nothing with Left on a closed top-level folder", () => {
        const { load } = setup();
        load();
        click(chevron("d:tests"));
        focusRow("d:tests");
        press("ArrowLeft");
        expect(shown()).toEqual(["d:tests"]);
        expect(item("d:tests")!.getAttribute("aria-expanded")).toBe("false");
        expect(focused()).toBe("d:tests");
        expect(selected()).toEqual([]);
      });
    });

    it("moves to the last row with End, a screen up with Page Up, and the first with Home", () => {
      const { send } = setup();
      const t = bigTree(2000);
      open(send, t);
      expect(items()).toHaveLength(35); // of 2,201 rows
      const keys = order(t);
      focusRow("d:tests");
      press("End");
      expect(focused()).toBe(keys[2200]);
      expect(selected()).toEqual([item(keys[2200])]);
      expectInView(keys[2200]);
      press("PageUp"); // 25 whole rows in the 600 px view
      expect(focused()).toBe(keys[2175]);
      press("Home");
      expect(focused()).toBe("d:tests");
      expectInView("d:tests");
    });

    it("runs the focused row with Ctrl+Enter, and not during a run", () => {
      const { api, send, load } = setup();
      load();
      focusRow("s:tests/basic.feature#1");
      expect(press("Enter", { ctrlKey: true }).defaultPrevented).toBe(true);
      expect(api.run).toHaveBeenCalledTimes(1);
      expect(api.run).toHaveBeenCalledWith("s:tests/basic.feature#1");
      expect(selected()).toEqual([]); // runs, does not select
      startRun(send);
      press("Enter", { ctrlKey: true });
      expect(api.run).toHaveBeenCalledTimes(1);
    });

    it("runs nothing with Ctrl+Enter on a row the filter mutes: it has no run button", () => {
      const { api, send, load } = setup();
      load();
      send({ type: "filter", filter: { include: ["smoke"], exclude: [], hideNonMatching: false } });
      focusRow("s:tests/basic.feature#2");
      press("Enter", { ctrlKey: true });
      expect(api.run).not.toHaveBeenCalled();
    });

    it("keeps the row's buttons out of the tab order, and they still click", () => {
      const { api, load } = setup();
      load();
      const buttons = within(screen.getByTestId("tree")).getAllByRole("button");
      expect(buttons.length).toBeGreaterThan(0);
      for (const b of buttons) expect(b.getAttribute("tabindex")).toBe("-1");
      click(screen.getByRole("button", { name: "Run passing hello" }));
      expect(api.run).toHaveBeenCalledWith("s:tests/basic.feature#1");
    });

    it("makes a clicked row the focused one, so the next key moves from it", () => {
      const { load } = setup();
      load();
      focusRow("d:tests");
      click(within(rowByKey("f:tests/outline.feature")).getByText("outline.feature"));
      press("ArrowDown");
      expect(focused()).toBe("o:tests/outline.feature#1");
      expect(selected()).toEqual([item("o:tests/outline.feature#1")]);
    });
  });

  describe("focus and scrolling (R4)", () => {
    it("takes focus to a folder closed by its chevron, keeping the selection", () => {
      const { load } = setup();
      load();
      click(screen.getByText("passing hello"));
      expect(focused()).toBe("s:tests/basic.feature#1");
      click(chevron("f:tests/basic.feature"));
      expect(focused()).toBe("f:tests/basic.feature");
      expect(heading()).toBe("passing hello");
    });

    it("moves focus to the nearest shown row above when Hide non-matching hides it", () => {
      const { send, load } = setup();
      load();
      click(screen.getByText("failing match"));
      send({ type: "filter", filter: { include: ["smoke"], exclude: [], hideNonMatching: true } });
      expect(focused()).toBe("s:tests/basic.feature#1");
      expect(heading()).toBe("failing match");
    });

    it("keeps focus in the tree when a refresh removes the focused row", () => {
      const { send, load } = setup();
      load();
      focusRow("s:tests/basic.feature#2");
      const t = tree();
      t.children[0].children.shift(); // basic.feature is gone
      open(send, t);
      expect(document.activeElement!.getAttribute("role")).toBe("treeitem");
      expect(focused()).toBe("d:tests");
    });

    it("scrolls to the selected row after a filter change takes rows away above the view", () => {
      const { send } = setup();
      open(send, bigTree(2000));
      const target = "s:tests/f150.feature#1"; // test 1500, even, row 1652
      scrollTo(8 + 1652 * 24 - 300);
      click(screen.getByText("test 1500"));
      scrollTo(8 + 1000 * 24); // row 1000 on top, the selected row far below
      send({ type: "filter", filter: { include: ["even"], exclude: [], hideNonMatching: true } });
      expect(selected()).toEqual([item(target)]);
      expectInView(target);
    });

    it("does not scroll to the selected row for a new tree and the same filter", () => {
      const { send } = setup();
      open(send, bigTree(2000));
      scrollTo(8 + 1652 * 24 - 300);
      click(screen.getByText("test 1500"));
      scrollTo(8 + 200 * 24);
      open(send, bigTree(2000)); // a refresh, which main follows with the filter in force
      send({ type: "filter", filter: { ...NO_FILTER } });
      expect(scroller().scrollTop).toBe(8 + 200 * 24);
    });

    it("shows a cut label whole when Down moves to its row, and closes it on the next Down", () => {
      const { load } = setup();
      load();
      const label = screen.getByText("failing match");
      Object.defineProperty(label, "scrollWidth", { value: 200, configurable: true });
      Object.defineProperty(label, "clientWidth", { value: 100, configurable: true });
      const tip = () => document.querySelector<HTMLElement>("[role=tooltip]");
      focusRow("s:tests/basic.feature#1");
      expect(tip()).toBeNull();
      press("ArrowDown");
      expect(tip()!.textContent).toBe("failing match");
      expect(tip()!.dataset.side).toBe("over");
      expect(label.parentElement!.contains(tip())).toBe(true);
      press("ArrowDown");
      expect(tip()).toBeNull();
    });

    it("shows a cut label whole when Left or Right moves to its row", () => {
      const { load } = setup();
      load();
      const cut = (label: HTMLElement) => {
        Object.defineProperty(label, "scrollWidth", { value: 200, configurable: true });
        Object.defineProperty(label, "clientWidth", { value: 100, configurable: true });
      };
      cut(within(rowByKey("f:tests/basic.feature")).getAllByRole("button")[1]);
      cut(screen.getByText("passing hello"));
      const tip = () => document.querySelector<HTMLElement>("[role=tooltip]");
      focusRow("s:tests/basic.feature#2");
      press("ArrowLeft");
      expect(focused()).toBe("f:tests/basic.feature");
      expect(tip()!.textContent).toBe("basic.feature · basic API checks");
      press("ArrowRight");
      expect(focused()).toBe("s:tests/basic.feature#1");
      expect(tip()!.textContent).toBe("passing hello");
    });

    it("names the run shortcut on every row and on the run button's tooltip", () => {
      vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
      try {
        const { load } = setup();
        load();
        expect(items().map((r) => r.getAttribute("aria-keyshortcuts"))).toEqual(
          Array(8).fill("Control+Enter"),
        );
        const run = screen.getByRole("button", { name: "Run passing hello" });
        fireEvent.pointerEnter(run.parentElement!);
        vi.advanceTimersByTime(500);
        expect(document.querySelector("[role=tooltip]")!.textContent).toBe(
          "Run passing hello (Ctrl+Enter)",
        );
      } finally {
        vi.useRealTimers();
      }
    });
  });

  describe("screen readers (R5)", () => {
    it("gives rows their level, place and open state, in a tree named Tests", () => {
      const { send } = setup();
      const scenario = (f: string, n: number) =>
        node("scenario", `s:tests/${f}.feature#${n}`, `${f}${n}`, { line: n });
      open(
        send,
        node("folder", "d:", "p", {
          children: [
            node("folder", "d:tests", "tests", {
              children: [
                node("feature", "f:tests/a.feature", "a", {
                  file: "tests/a.feature",
                  children: [scenario("a", 1)],
                }),
                node("feature", "f:tests/b.feature", "b", {
                  file: "tests/b.feature",
                  children: [1, 2, 3].map((n) => scenario("b", n)),
                }),
              ],
            }),
          ],
        }),
      );
      const aria = (key: string) =>
        ["level", "posinset", "setsize", "expanded"].map((a) =>
          item(key)!.getAttribute(`aria-${a}`),
        );
      expect(aria("f:tests/b.feature")).toEqual(["2", "2", "2", "true"]);
      expect(aria("s:tests/b.feature#3")).toEqual(["3", "3", "3", null]);
      expect(screen.getByRole("tree", { name: "Tests" })).toBe(screen.getByTestId("tree"));
    });

    it("names a row by its label, status, flaky chip and tags", () => {
      const { send } = setup();
      open(
        send,
        node("folder", "d:", "p", {
          children: [
            node("feature", "f:tests/login.feature", "login", {
              file: "tests/login.feature",
              tags: ["smoke"],
              children: [
                node("scenario", "s:tests/login.feature#1", "login works", { line: 3 }),
                node("scenario", "s:tests/login.feature#2", "logout", { line: 8 }),
              ],
            }),
          ],
        }),
      );
      startRun(send);
      send({
        type: "scenarios",
        updates: [{ key: "s:tests/login.feature#1", status: "failed", error: "boom" }],
      });
      const proof: FlakyEnv[] = [{ env: null, passed: 1, failed: 1, runs: [] }];
      send({
        type: "flaky",
        tests: [
          { key: "s:tests/login.feature#1", envs: proof },
          { key: "s:tests/login.feature#2", envs: proof },
        ],
      });
      expect(screen.getByRole("treeitem", { name: "login.feature, failed, 2 flaky, smoke" })).toBe(
        item("f:tests/login.feature"),
      );
      expect(screen.getByRole("treeitem", { name: "login works, failed, flaky" })).toBe(
        item("s:tests/login.feature#1"),
      );
    });
  });
});
