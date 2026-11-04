import { render, screen, cleanup, fireEvent, within } from "@solidjs/testing-library";
import {
  ChevronRight,
  CircleAlert,
  CircleCheck,
  CircleMinus,
  CircleSlash,
  CircleX,
  LoaderCircle,
  MousePointerClick,
} from "lucide";
import { createStore, flush } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AppEvent, Step } from "../shared/events";
import type { Api } from "../shared/ipc";
import type { TestNode } from "../shared/model";
import { deriveStatus } from "../shared/status";
import { App } from "./App";
import { Details } from "./Details";
import { initialState, reduce } from "./reduce";
import { drawn } from "../../test/drawn";
import { openFeatures } from "../../test/openFeatures";

// The real deriveStatus, counted: its own recursion is not, so each call is one walk of a subtree.
vi.mock("../shared/status", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../shared/status")>();
  return { ...actual, deriveStatus: vi.fn(actual.deriveStatus) };
});

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
const OK = "s:tests/x.feature#1";
const BAD = "s:tests/x.feature#2";
const WAIT = "s:tests/x.feature#3";
const tree = (): TestNode =>
  node("folder", "d:", "p", {
    children: [
      node("feature", "f:tests/x.feature", "x", {
        children: [
          node("scenario", OK, "fine", { line: 3, tags: ["smoke", "slow"] }),
          node("scenario", BAD, "broken", { line: 6 }),
          node("scenario", WAIT, "waiting", { line: 9 }),
        ],
      }),
    ],
  });

const step = (text: string, status: string, over: Partial<Step> = {}): Step => ({
  keyword: "Given",
  text,
  line: 4,
  status,
  millis: 3,
  background: false,
  log: null,
  ...over,
});

function setup() {
  let emit: (e: AppEvent) => void = () => {};
  const api = {
    ready: vi.fn(async () => {}),
    openProject: vi.fn(async () => {}),
    refresh: vi.fn(async () => {}),
    run: vi.fn(async () => {}),
    steps: vi.fn(async () => null),
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
  send({
    type: "project",
    project: { root: "D:/p", layout: "standalone" },
    tree: tree(),
    error: null,
  });
  openFeatures();
  const select = (name: string) => {
    fireEvent.click(screen.getByText(name));
    flush();
  };
  /** A run of the first two tests that has finished: `fine` passed, `broken` failed. */
  const finished = () => {
    send({
      type: "runStarted",
      selection: "d:",
      runId: "20261025-150000-000",
      keys: [OK, BAD, WAIT],
      total: 3,
      startedAt: Date.now(),
      env: null,
      debug: false,
    });
    send({
      type: "scenarios",
      updates: [
        { key: OK, status: "passed", durationMillis: 1500 },
        { key: BAD, status: "failed", durationMillis: 40, error: "boom at x.feature:7" },
      ],
    });
    send({
      type: "runFinished",
      outcome: "finished",
      exitCode: 1,
      summary: null,
      reportAvailable: false,
    });
  };
  return { send, select, finished };
}

const details = () => screen.getByTestId("details");

afterEach(cleanup);

describe("details: a test", () => {
  it("heads it with its name, a tinted status badge with its icon and word, and the meta line", () => {
    const { select, finished } = setup();
    finished();
    select("fine");
    expect(within(details()).getByRole("heading", { level: 2 }).textContent).toBe("fine");
    const status = screen.getByTestId("details-status");
    expect(status.textContent).toBe("Passed"); // the word is the badge's name: the icon is decorative
    expect(status.className).toContain("rounded-full");
    expect(status.className).toContain("bg-passed-tint");
    expect(drawn(status.querySelector("svg"), CircleCheck)).toBe(true);
    expect(status.querySelector("svg")!.getAttribute("aria-hidden")).toBe("true");
    const badgeMark = status.querySelector<HTMLElement>("[data-status]")!; // the shared mark, hidden: the word names it
    expect(badgeMark.dataset.status).toBe("passed");
    expect(badgeMark.getAttribute("aria-hidden")).toBe("true");
    expect(within(status).queryByRole("img")).toBeNull();
    expect(screen.getByTestId("details-location").textContent).toBe("tests/x.feature:3");
    expect(screen.getByTestId("details-location").className).toContain("font-mono");
    expect(screen.getByTestId("details-duration").textContent).toBe("1.50 s");
    expect(screen.getByTestId("details-duration").className).toContain("font-mono");
    // tags are chips
    for (const tag of ["smoke", "slow"]) {
      const chip = within(details()).getByText(tag);
      expect(chip.className).toContain("rounded-sm");
      expect(chip.className).toContain("border-line");
    }
  });

  it("shows a status badge for a failed and a not-run test", () => {
    const { select, finished } = setup();
    finished();
    select("broken");
    const failed = screen.getByTestId("details-status");
    expect(failed.textContent).toBe("Failed");
    expect(failed.className).toContain("bg-failed-tint");
    expect(drawn(failed.querySelector("svg"), CircleX)).toBe(true);
    select("waiting");
    const pending = screen.getByTestId("details-status");
    expect(pending.textContent).toBe("Pending");
    expect(pending.className).toContain("bg-skipped-tint");
  });

  it("puts a failed test's error in a failed box titled Error", () => {
    const { select, finished } = setup();
    finished();
    select("broken");
    const error = screen.getByTestId("details-error");
    expect(error.textContent).toBe("boom at x.feature:7"); // the message alone
    expect(error.className).toContain("font-mono");
    const box = error.parentElement!;
    expect(box.className).toContain("rounded-lg");
    expect(box.className).toContain("bg-failed-tint");
    const title = within(box).getByText("Error");
    expect(drawn(title.querySelector("svg"), CircleAlert)).toBe(true);
    // no error box for a passed test
    select("fine");
    expect(screen.queryByTestId("details-error")).toBeNull();
  });

  it("shows each step with a mark named by its status", () => {
    const { send, select, finished } = setup();
    finished();
    send({
      type: "steps",
      key: BAD,
      steps: [
        step("path 'a'", "passed"),
        step("method get", "failed"),
        step("match 1 == 1", "skipped", { keyword: "Then" }),
        step("print 'x'", "something-new", { keyword: "*", background: true }),
      ],
    });
    select("broken");
    const rows = screen.getAllByTestId("step");
    expect(rows.map((r) => r.dataset.status)).toEqual([
      "passed",
      "failed",
      "skipped",
      "something-new",
    ]);
    const marks = rows.map((r) => within(r).getByRole("img"));
    expect(marks.map((m) => m.getAttribute("aria-label"))).toEqual([
      "Passed",
      "Failed",
      "Skipped",
      "Interrupted", // a status Karate has not shown us: the amber mark
    ]);
    expect(drawn(marks[0].querySelector("svg"), CircleCheck)).toBe(true);
    expect(drawn(marks[1].querySelector("svg"), CircleX)).toBe(true);
    expect(drawn(marks[2].querySelector("svg"), CircleMinus)).toBe(true);
    expect(drawn(marks[3].querySelector("svg"), CircleSlash)).toBe(true);
    expect(marks[0].querySelector("svg")!.getAttribute("width")).toBe("14");
    // the keyword in 500, the text in the mono, the duration in the mono
    expect(within(rows[2]).getByText("Then").className).toContain("font-medium");
    expect(within(rows[2]).getByText("match 1 == 1").className).toContain("font-mono");
    expect(rows[0].textContent).toContain("Given path 'a'");
    expect(within(rows[0]).getByText("3 ms").className).toContain("font-mono");
    // only the background step has the chip
    expect(rows.map((r) => !!within(r).queryByText("Background"))).toEqual([
      false,
      false,
      false,
      true,
    ]);
  });

  it("shows a step's comments above it, muted, as written (step comments R2)", () => {
    const { send, select, finished } = setup();
    finished();
    send({
      type: "steps",
      key: BAD,
      steps: [
        step("path 'a'", "passed"),
        step("method get", "failed", { comments: ["# why", "# where from"] }),
      ],
    });
    select("broken");
    const rows = screen.getAllByTestId("step");
    expect(within(rows[0]).queryByTestId("step-comments")).toBeNull();
    const comments = within(rows[1]).getByTestId("step-comments");
    expect([...comments.children].map((c) => c.textContent)).toEqual(["# why", "# where from"]);
    expect(comments.className).toContain("text-muted");
    expect(comments.className).toContain("font-mono");
    // above the step: the comments come before the step's text in the row
    expect(rows[1].textContent!.indexOf("# where from")).toBeLessThan(
      rows[1].textContent!.indexOf("method get"),
    );
  });

  it("toggles a step's log with a ghost button and a chevron that turns", () => {
    const { send, select, finished } = setup();
    finished();
    send({ type: "steps", key: BAD, steps: [step("print 'x'", "passed", { log: "x" })] });
    select("broken");
    const toggle = screen.getByTestId("step-toggle");
    expect(toggle.textContent).toBe("Log");
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    expect(toggle.getAttribute("aria-label")).toBe("Show log of print 'x'");
    expect(toggle.className).toContain("hover:bg-hover"); // a ghost button
    const chevron = toggle.querySelector("svg")!;
    expect(drawn(chevron, ChevronRight)).toBe(true);
    expect(chevron.getAttribute("class")).not.toContain("rotate-90");
    expect(screen.queryByTestId("step-log")).toBeNull();
    fireEvent.click(toggle);
    flush();
    expect(toggle.getAttribute("aria-expanded")).toBe("true");
    expect(toggle.getAttribute("aria-label")).toBe("Hide log of print 'x'");
    expect(toggle.querySelector("svg")!.getAttribute("class")).toContain("rotate-90");
    const log = screen.getByTestId("step-log");
    expect(log.textContent).toBe("x");
    expect(log.className).toContain("bg-sunken");
    fireEvent.click(toggle);
    flush();
    expect(screen.queryByTestId("step-log")).toBeNull();
  });

  it("says steps come when the feature finishes, with the spinner while a run is on", () => {
    const { send, select, finished } = setup();
    finished();
    select("broken");
    const note = within(details()).getByText("Steps are available when the feature finishes");
    expect(note.className).toContain("text-muted");
    expect(note.querySelector("svg")).toBeNull(); // nothing runs
    send({
      type: "runStarted",
      selection: "d:",
      runId: "20261025-150100-000",
      keys: [OK, BAD, WAIT],
      total: 3,
      startedAt: Date.now(),
      env: null,
      debug: false,
    });
    const spinner = within(details())
      .getByText("Steps are available when the feature finishes")
      .querySelector("svg");
    expect(drawn(spinner, LoaderCircle)).toBe(true);
    expect(spinner!.getAttribute("class")).toContain("animate-spin");
  });
});

describe("details: a group", () => {
  it("counts the tests per status, each with its mark, figure and word", () => {
    const { select, finished } = setup();
    finished();
    select("x.feature");
    expect(within(details()).getByRole("heading", { level: 2 }).textContent).toBe("x");
    const status = screen.getByTestId("details-status");
    expect(status.textContent).toBe("Failed");
    expect(status.className).toContain("bg-failed-tint");
    expect(screen.getByTestId("details-location").textContent).toBe("tests/x.feature");
    const counts = screen.getByTestId("details-counts");
    expect(counts.textContent).toContain("3 scenarios");
    expect(within(counts).getByText("3 scenarios").className).toContain("tabular-nums"); // R2.4
    // the word after each mark names it, so the marks are hidden from the accessibility tree
    expect(within(counts).queryAllByRole("img")).toHaveLength(0);
    const marks = [...counts.querySelectorAll<HTMLElement>("[data-status]")];
    expect(marks.map((m) => m.dataset.status)).toEqual(["passed", "failed", "pending"]);
    expect(
      marks.every((m) => m.getAttribute("aria-hidden") === "true" && !m.hasAttribute("role")),
    ).toBe(true);
    expect(drawn(marks[1].querySelector("svg"), CircleX)).toBe(true);
    // a mark, then the figure in tabular numbers, then the word
    const failed = marks[1].parentElement!;
    expect(failed.textContent).toBe("1 Failed");
    expect(within(failed).getByText("1").className).toContain("tabular-nums");
  });
});

describe("details: a group during a run", () => {
  it("works out the group's status once per update, however many places show it", () => {
    const [state, set] = createStore(initialState());
    const send = (e: AppEvent) => {
      set((draft) => reduce(draft, e));
      flush();
    };
    render(() => <Details state={state} api={{} as Api} select={() => {}} reveal={() => {}} />);
    send({
      type: "project",
      project: { root: "D:/p", layout: "standalone" },
      tree: tree(),
      error: null,
    });
    set((draft) => void (draft.selected = "f:tests/x.feature"));
    flush();
    send({
      type: "runStarted",
      selection: "d:",
      runId: "20261030-120000-000",
      keys: [OK, BAD, WAIT],
      total: 3,
      startedAt: Date.now(),
      env: null,
      debug: false,
    });
    vi.mocked(deriveStatus).mockClear();
    send({ type: "scenarios", updates: [{ key: BAD, status: "failed", error: "boom" }] });
    expect(screen.getByTestId("details-status").textContent).toBe("Failed");
    expect(vi.mocked(deriveStatus)).toHaveBeenCalledTimes(1);
  });
});

describe("details: nothing selected", () => {
  it("is an empty state", () => {
    setup();
    const text = within(details()).getByText("Select a test to see its details");
    expect(text.className).toContain("text-muted");
    expect(drawn(text.parentElement!.querySelector("svg"), MousePointerClick)).toBe(true);
  });
});

describe("details: long text", () => {
  // jsdom has no layout: say how wide the text is and how much of it shows
  const measure = (el: HTMLElement, scrollWidth: number, clientWidth: number) => {
    Object.defineProperty(el, "scrollWidth", { value: scrollWidth, configurable: true });
    Object.defineProperty(el, "clientWidth", { value: clientWidth, configurable: true });
  };
  const tip = () => document.querySelector("[role=tooltip]");
  const rest = (el: HTMLElement) => {
    fireEvent.pointerEnter(el.parentElement!);
    vi.advanceTimersByTime(500);
  };
  const withTimers = (fn: () => void) => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    try {
      fn();
    } finally {
      vi.useRealTimers();
    }
  };

  it("shows the location whole in a tooltip only when it is cut", () => {
    withTimers(() => {
      const { select, finished } = setup();
      finished();
      select("fine");
      const location = screen.getByTestId("details-location");
      expect(location.className).toContain("truncate");
      expect(location.hasAttribute("title")).toBe(false);
      measure(location, 100, 100);
      rest(location);
      expect(tip()).toBeNull();
      measure(location, 300, 100);
      rest(location);
      expect(tip()!.textContent).toBe("tests/x.feature:3");
    });
  });

  it("shows a step whole in a tooltip only when it is cut", () => {
    withTimers(() => {
      const { send, select, finished } = setup();
      finished();
      send({ type: "steps", key: BAD, steps: [step("match response == { a: 1 }", "passed")] });
      select("broken");
      const text = within(screen.getByTestId("step")).getByText("match response == { a: 1 }");
      const cell = text.parentElement!; // the truncating span holds the keyword and the text
      expect(cell.className).toContain("truncate");
      measure(cell, 100, 100);
      rest(cell);
      expect(tip()).toBeNull();
      measure(cell, 300, 100);
      rest(cell);
      expect(tip()!.textContent).toBe("Given match response == { a: 1 }");
      // over its row: below, it would take the click meant for the next row
      expect((tip() as HTMLElement).dataset.side).toBe("over");
    });
  });
});

describe("details: styling", () => {
  it("uses no raw palette class, no dark: variant and no title", () => {
    const { send, select, finished } = setup();
    finished();
    send({
      type: "steps",
      key: BAD,
      steps: [step("print 'x'", "failed", { log: "x", background: true })],
    });
    select("broken");
    fireEvent.click(screen.getByTestId("step-toggle"));
    flush();
    const all = [...details().querySelectorAll("*")].map((e) => e.getAttribute("class") ?? "");
    expect(all.join(" ")).not.toMatch(/\b(zinc|red|green|sky|amber|slate|gray)-\d|dark:/);
    expect(details().querySelectorAll("[title]")).toHaveLength(0);
  });
});

describe("details: steps and breakpoints (step debugger R4, R7)", () => {
  const leafNode = node("scenario", "s:tests/x.feature#1", "variables", { line: 6 });
  const project: AppEvent = {
    type: "project",
    project: { root: "D:/p", layout: "standalone" },
    tree: node("folder", "d:", "p", { file: "", children: [leafNode] }),
    error: null,
  };
  const fromFile = [
    { keyword: "*", text: "def base = 1", line: 4, background: true },
    { keyword: "*", text: "def name = 'grace'", line: 7, background: false },
    {
      keyword: "*",
      text: "def user = { id: 7 }",
      line: 8,
      background: false,
      comments: ["# the user"],
    },
  ];
  const pausedAt = (line: number, depth = 0, file = "tests/x.feature"): AppEvent => ({
    type: "debugPaused",
    paused: {
      key: leafNode.key,
      file,
      line,
      depth,
      prefix: "*",
      keyword: "def",
      text: "x",
      vars: [],
    },
  });

  async function show() {
    const [state, set] = createStore(initialState());
    const send = (e: AppEvent) => {
      set((draft) => reduce(draft, e));
      flush();
    };
    const api = {
      steps: vi.fn(async () => fromFile),
      toggleBreakpoint: vi.fn(async () => {}),
    };
    render(() => (
      <Details state={state} api={api as unknown as Api} select={() => {}} reveal={() => {}} />
    ));
    send(project);
    set((draft) => void (draft.selected = leafNode.key));
    flush();
    await vi.waitFor(() => expect(screen.getAllByTestId("step")).toHaveLength(3));
    return { api, send, set };
  }
  const toggles = () => screen.getAllByTestId("breakpoint");

  it("lists the steps from the feature file before a run, with no status or time", async () => {
    const { api } = await show();
    expect(api.steps).toHaveBeenCalledTimes(1);
    expect(api.steps).toHaveBeenCalledWith(leafNode.key);
    const [first] = screen.getAllByTestId("step");
    expect(first.dataset.status).toBeUndefined();
    expect(first.textContent).toContain("Background");
    expect(first.textContent).toContain("def base = 1");
    expect(first.querySelector("[role=img]")).toBeNull();
  });

  it("toggles a breakpoint on a step and shows main's set", async () => {
    const { api, send } = await show();
    expect(toggles()[2].getAttribute("aria-label")).toBe("Add breakpoint at line 8");
    fireEvent.click(toggles()[2]);
    expect(api.toggleBreakpoint).toHaveBeenCalledWith("tests/x.feature", 8);
    send({ type: "breakpoints", breakpoints: [{ file: "tests/x.feature", line: 8 }] });
    expect(toggles()[2].getAttribute("aria-label")).toBe("Remove breakpoint at line 8");
    expect(toggles()[2].querySelector("span")!.className).toContain("bg-failed");
    expect(toggles()[1].querySelector("span")!.className).not.toContain("bg-failed");
  });

  it("marks the step the run is paused before, or names the called feature", async () => {
    const { send } = await show();
    send(pausedAt(8));
    const rows = screen.getAllByTestId("step");
    expect(rows[2].dataset.paused).toBe("");
    expect(rows[2].textContent).toContain("Paused");
    expect(within(rows[2]).getByTestId("step-comments").textContent).toBe("# the user");
    expect(rows[1].dataset.paused).toBeUndefined();
    send(pausedAt(5, 1, "called.feature"));
    expect(screen.getAllByTestId("step").some((r) => r.dataset.paused === "")).toBe(false);
    expect(screen.getByTestId("paused-in").textContent).toBe("Paused in called.feature:5");
  });

  it("the run's steps replace the file's, with the same toggles", async () => {
    const { send } = await show();
    const run = fromFile.map((s) => ({ ...s, status: "passed", millis: 3, log: null }));
    send({ type: "steps", key: leafNode.key, steps: run });
    expect(screen.getAllByTestId("step")[0].dataset.status).toBe("passed");
    expect(toggles()).toHaveLength(3);
  });
});

describe("details: open in VS Code (open-in-editor R1, R2)", () => {
  const scenario = node("scenario", "s:tests/x.feature#1", "greets", { line: 6 });
  const row = node("row", "s:tests/x.feature#2.1", "row 1", { line: null });

  function show(selected: TestNode) {
    const [state, set] = createStore(initialState());
    const send = (e: AppEvent) => {
      set((draft) => reduce(draft, e));
      flush();
    };
    const api = { steps: vi.fn(async () => null), openInEditor: vi.fn(async () => {}) };
    render(() => (
      <Details state={state} api={api as unknown as Api} select={() => {}} reveal={() => {}} />
    ));
    send({
      type: "project",
      project: { root: "D:/p", layout: "standalone" },
      tree: node("folder", "d:", "p", { file: "", children: [scenario, row] }),
      error: null,
    });
    set((draft) => void (draft.selected = selected.key));
    flush();
    const start = () =>
      send({
        type: "runStarted",
        selection: "d:",
        runId: "20261103-090000-000",
        keys: [scenario.key],
        total: 1,
        startedAt: Date.now(),
        env: null,
        debug: false,
      });
    const fail = (error: string) => {
      start();
      send({ type: "scenarios", updates: [{ key: scenario.key, status: "failed", error }] });
    };
    return { api, start, fail };
  }
  const open = () => screen.queryByRole("button", { name: "Open in VS Code" });
  const inError = () => within(screen.getByTestId("details-error")).queryAllByRole("button");

  it("opens the test at its line, during a run too", () => {
    const { api, start } = show(scenario);
    fireEvent.click(open()!);
    expect(api.openInEditor).toHaveBeenCalledWith("tests/x.feature", 6);
    start();
    expect(open()).not.toBeNull();
  });

  it("shows no button for a row with no line", () => {
    show(row);
    expect(open()).toBeNull();
  });

  it("makes the place Karate names on the error's first line a link", () => {
    const error = String.raw`C:\p\tests\auth.feature:9` + "\nThen status 200\nexpected status: 200";
    const { api, fail } = show(scenario);
    fail(error);
    expect(screen.getByTestId("details-error").textContent).toBe(error);
    fireEvent.click(
      screen.getByRole("button", { name: String.raw`Open C:\p\tests\auth.feature:9 in VS Code` }),
    );
    expect(api.openInEditor).toHaveBeenCalledWith(String.raw`C:\p\tests\auth.feature`, 9);
  });

  it("takes a lowercase drive and / separators", () => {
    const { api, fail } = show(scenario);
    fail("c:/p/a.feature:3\nboom");
    fireEvent.click(inError()[0]);
    expect(api.openInEditor).toHaveBeenCalledWith("c:/p/a.feature", 3);
  });

  it.each([
    "Config evaluation failed: karate-config.js - js failed:\nLine: 4",
    "boom at x.feature:7",
  ])("shows an error without a place as text: %s", (error) => {
    const { fail } = show(scenario);
    fail(error);
    expect(inError()).toHaveLength(0);
    expect(screen.getByTestId("details-error").textContent).toBe(error);
  });
});

describe("details: step through a group (#229 R4)", () => {
  const [A, B, C, D] = [1, 2, 3, 4].map((n) => `s:tests/g.feature#${n}`);
  const NAMES = ["alpha", "beta", "gamma", "delta"];
  const REASON = "expected status: 200, actual: 503";
  const failure = (line: number, reason = REASON) => {
    const head = `C:/p/tests/g.feature:${line}\nThen status 200\n`;
    return { error: head + reason, reasonAt: head.length };
  };
  const groupTree = (): TestNode =>
    node("folder", "d:", "p", {
      file: "",
      children: [
        node("feature", "f:tests/g.feature", "g", {
          file: "tests/g.feature",
          children: [A, B, C, D].map((key, i) =>
            node("scenario", key, NAMES[i], { file: "tests/g.feature", line: 3 * i + 3 }),
          ),
        }),
      ],
    });

  function mount() {
    let emit: (e: AppEvent) => void = () => {};
    const api = {
      ready: vi.fn(async () => {}),
      steps: vi.fn(async () => null),
      toggleBreakpoint: vi.fn(async () => {}),
      onEvent: (cb: (e: AppEvent) => void) => ((emit = cb), () => {}),
    };
    render(() => <App api={api as unknown as Api} />);
    const send = (e: AppEvent) => {
      emit(e);
      flush();
    };
    send({
      type: "project",
      project: { root: "D:/p", layout: "standalone" },
      tree: groupTree(),
      error: null,
    });
    openFeatures();
    const run = (keys: string[]) =>
      send({
        type: "runStarted",
        selection: "d:",
        runId: `20261104-12000${keys.length}-000`,
        keys,
        total: keys.length,
        startedAt: Date.now(),
        env: null,
        debug: false,
      });
    run([A, B, C, D]);
    send({
      type: "scenarios",
      updates: [
        { key: A, status: "failed", ...failure(5) },
        { key: B, status: "failed", ...failure(8) },
        { key: C, status: "failed", ...failure(11) },
        { key: D, status: "passed" },
      ],
    });
    send({
      type: "runFinished",
      outcome: "finished",
      exitCode: 1,
      summary: null,
      reportAvailable: false,
    });
    return { send, run };
  }
  const rowOf = (key: string) =>
    screen.getAllByTestId("tree-node").find((n) => n.dataset.key === key)!;
  const pick = (key: string) => {
    fireEvent.click(within(rowOf(key)).getByText(NAMES[[A, B, C, D].indexOf(key)]));
    flush();
  };
  const selectedKey = () =>
    screen
      .getAllByTestId("tree-node")
      .find((n) => n.parentElement!.getAttribute("aria-selected") === "true")?.dataset.key;
  const group = () => screen.queryByTestId("error-group");
  const press = (name: string) => {
    const b = screen.getByRole("button", { name });
    b.focus();
    fireEvent.click(b);
    flush();
  };

  it("says where the test sits in its group", () => {
    mount();
    pick(A);
    expect(group()!.textContent).toBe("1 of 3 tests failed with this error");
    expect(screen.getByTestId("details-error").textContent).toBe(failure(5).error);
  });

  it("Next selects the next test and keeps the focus", () => {
    mount();
    pick(A);
    press("Next test with this error");
    expect(selectedKey()).toBe(B);
    expect(group()!.textContent).toBe("2 of 3 tests failed with this error");
    expect(document.activeElement).toBe(
      screen.getByRole("button", { name: "Next test with this error" }),
    );
  });

  it("Previous wraps round", () => {
    mount();
    pick(A);
    press("Previous test with this error");
    expect(selectedKey()).toBe(C);
  });

  it("a lone failure has no stepping", () => {
    const { send, run } = mount();
    run([D]);
    send({ type: "scenarios", updates: [{ key: D, status: "failed", ...failure(14, "boom") }] });
    send({
      type: "runFinished",
      outcome: "finished",
      exitCode: 1,
      summary: null,
      reportAvailable: false,
    });
    pick(D);
    expect(screen.getByTestId("details-error")).toBeTruthy();
    expect(group()).toBeNull();
  });

  it("no stepping while a run goes on", () => {
    const { run } = mount();
    run([D]);
    pick(A);
    expect(screen.getByTestId("details-error")).toBeTruthy();
    expect(group()).toBeNull();
  });

  it("has no breakpoint dot on a step below line 1 (#229 R1.4)", () => {
    const { send } = mount();
    pick(A);
    send({
      type: "steps",
      key: A,
      steps: [
        step("Scenario execution failed: boom", "failed", { keyword: "*", line: 0 }),
        step("ok", "passed"),
      ],
    });
    expect(within(details()).getAllByTestId("step")).toHaveLength(2);
    expect(within(details()).getAllByTestId("breakpoint")).toHaveLength(1);
  });
});
