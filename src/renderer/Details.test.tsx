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
  type IconNode,
} from "lucide";
import { flush } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AppEvent, Step } from "../shared/events";
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

/** Whether the svg holds this icon: every shape of the Lucide icon is in it. */
const drawn = (svg: Element | null | undefined, icon: IconNode) =>
  !!svg &&
  icon.every(([, attrs]) => Object.values(attrs).every((v) => svg.innerHTML.includes(String(v))));
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
    const marks = within(counts).getAllByRole("img");
    expect(marks.map((m) => m.getAttribute("aria-label"))).toEqual(["Passed", "Failed", "Pending"]);
    expect(drawn(marks[1].querySelector("svg"), CircleX)).toBe(true);
    // a mark, then the figure in tabular numbers, then the word
    const failed = marks[1].parentElement!;
    expect(failed.textContent).toBe("1 Failed");
    expect(within(failed).getByText("1").className).toContain("tabular-nums");
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
