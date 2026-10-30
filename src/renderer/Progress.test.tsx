import { cleanup, fireEvent, render, screen } from "@solidjs/testing-library";
import { CircleAlert, CircleCheck, CircleSlash, LoaderCircle, type IconNode } from "lucide";
import { createStore, flush } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AppEvent } from "../shared/events";
import type { TestNode } from "../shared/model";
import { Icon } from "./Icon";
import { Progress } from "./Progress";
import { initialState, reduce } from "./reduce";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

const leaf = (n: number): TestNode => ({
  key: `s:tests/x.feature#${n}`,
  kind: "scenario",
  name: `test ${n}`,
  file: "tests/x.feature",
  line: n,
  refId: null,
  tags: [],
  children: [],
});
const tree = (count: number): TestNode => ({
  key: "d:",
  kind: "folder",
  name: "p",
  file: "",
  line: null,
  refId: null,
  tags: [],
  children: [
    {
      key: "f:tests/x.feature",
      kind: "feature",
      name: "x",
      file: "tests/x.feature",
      line: null,
      refId: null,
      tags: [],
      children: Array.from({ length: count }, (_, i) => leaf(i + 1)),
    },
  ],
});
const loaded = (count: number): AppEvent => ({
  type: "project",
  project: { root: "D:/p", layout: "standalone" },
  tree: tree(count),
  error: null,
});
const started = (total: number, env: string | null = null): AppEvent => ({
  type: "runStarted",
  selection: "d:",
  runId: "20261029-120000-000",
  keys: [],
  total,
  startedAt: Date.now(),
  env,
});
const result = (n: number, status: "passed" | "failed" | "interrupted"): AppEvent => ({
  type: "scenarios",
  updates: [{ key: leaf(n).key, status }],
});
const history = (): AppEvent => ({
  type: "history",
  runs: [
    {
      id: "20261028-100000-000",
      startedAt: 1,
      durationMillis: 2,
      env: null,
      threads: 1,
      filter: { include: [], exclude: [], hideNonMatching: false },
      selection: null,
      failedOf: null,
      outcome: "finished",
      exitCode: 0,
      passed: 1,
      failed: 0,
      total: 1,
      reportAvailable: true,
    },
  ],
});
const finished = (outcome: "finished" | "cancelled" | "failed"): AppEvent => ({
  type: "runFinished",
  outcome,
  exitCode: outcome === "finished" ? 0 : 1,
  summary: null,
  reportAvailable: false,
});

function setup() {
  const [state, set] = createStore(initialState());
  render(() => <Progress state={state} />);
  const send = (...events: AppEvent[]) => {
    for (const e of events) set((draft) => reduce(draft, e));
    flush();
  };
  return { send };
}

const text = (id: string) => screen.getByTestId(id).textContent;
/** The open tooltip: a popover, which jsdom does not count as accessible. */
const tip = () => document.querySelector('[role="tooltip"]')!;
/** What Icon draws for `icon`, to tell icons apart. */
function shapeOf(icon: IconNode) {
  const host = document.body.appendChild(document.createElement("div"));
  const { unmount } = render(() => <Icon icon={icon} />, { container: host });
  const html = host.querySelector("svg")!.innerHTML;
  unmount();
  host.remove();
  return html;
}
const outcomeIcon = () => screen.getByTestId("run-outcome").querySelector("svg");

describe("before any run (R14.3)", () => {
  it("counts the project's tests and says Not run yet, with no zero counts", () => {
    const { send } = setup();
    send(loaded(24));
    expect(text("progress-text")).toBe("24 tests · Not run yet");
    expect(screen.queryByTestId("progress-passed")).toBeNull();
    expect(screen.queryByTestId("progress-failed")).toBeNull();
    expect(screen.queryByTestId("progress-elapsed")).toBeNull();
    expect(
      screen.getByTestId("progress-text").closest("section")!.querySelector("[data-status]"),
    ).toBeNull();
  });

  it("says test for one", () => {
    const { send } = setup();
    send(loaded(1));
    expect(text("progress-text")).toBe("1 test · Not run yet");
  });

  it("only counts the tests once the project has runs in its history, and still shows no zero counts", () => {
    const { send } = setup();
    send(loaded(24), history());
    expect(text("progress-text")).toBe("24 tests");
    expect(screen.queryByTestId("progress-passed")).toBeNull();
    expect(screen.queryByTestId("progress-elapsed")).toBeNull();
    send({ type: "history", runs: [] }); // Clear history
    expect(text("progress-text")).toBe("24 tests · Not run yet");
  });

  it("keeps the progress bar and its values", () => {
    const { send } = setup();
    send(loaded(3));
    const bar = screen.getByTestId("progress-bar");
    expect(bar.getAttribute("role")).toBe("progressbar");
    expect(bar.getAttribute("aria-valuenow")).toBe("0");
    expect(text("run-outcome")).toBe("");
    expect(outcomeIcon()).toBeNull();
  });

  it("is over once a run has started, and also once an outcome is in", () => {
    const { send } = setup();
    send(loaded(4), started(4));
    expect(text("progress-text")).toBe("0 / 4");
    cleanup();
    const second = setup();
    second.send(loaded(4), finished("failed")); // refused before it started
    expect(text("progress-text")).toBe("0 / 0");
    expect(text("run-outcome")).toBe("Run failed");
  });
});

describe("after a run (R14.2)", () => {
  it("shows done / total in the mono and each count behind its StatusMark and word", () => {
    const { send } = setup();
    send(loaded(4), started(4), result(1, "passed"), result(2, "failed"));
    const progress = screen.getByTestId("progress-text");
    expect(progress.textContent).toBe("2 / 4");
    expect(progress.className).toContain("font-mono");
    expect(progress.className).toContain("tabular-nums");
    const passed = screen.getByTestId("progress-passed");
    expect(passed.textContent).toBe("1");
    expect(passed.className).toContain("font-mono");
    expect(passed.className).toContain("tabular-nums");
    expect(passed.parentElement!.textContent).toBe("1 passed");
    // the word after the mark names the status, so the mark is hidden from the accessibility tree
    const passedMark = passed.parentElement!.querySelector<HTMLElement>("[data-status]")!;
    expect(passedMark.dataset.status).toBe("passed");
    expect(passedMark.getAttribute("aria-hidden")).toBe("true");
    expect(passedMark.hasAttribute("role")).toBe(false);
    const failed = screen.getByTestId("progress-failed");
    expect(failed.parentElement!.textContent).toBe("1 failed");
    const failedMark = failed.parentElement!.querySelector<HTMLElement>("[data-status]")!;
    expect(failedMark.dataset.status).toBe("failed");
    expect(failedMark.getAttribute("aria-hidden")).toBe("true");
    expect(failedMark.hasAttribute("role")).toBe(false);
    expect(screen.queryAllByRole("img")).toHaveLength(0);
  });

  it("shows the elapsed time in the mono", () => {
    const { send } = setup();
    send(loaded(1), started(1));
    const elapsed = screen.getByTestId("progress-elapsed");
    expect(elapsed.textContent).toMatch(/^\d\d:\d\d$/);
    expect(elapsed.className).toContain("font-mono");
    expect(elapsed.className).toContain("tabular-nums");
  });

  it("names the outcome with its icon", () => {
    const { send } = setup();
    send(loaded(2), started(2));
    expect(text("run-outcome")).toBe("Running…");
    expect(outcomeIcon()!.innerHTML).toBe(shapeOf(LoaderCircle));
    expect(outcomeIcon()!.getAttribute("class")).toContain("animate-spin");
    expect(outcomeIcon()!.getAttribute("class")).toContain("text-running");
    expect(outcomeIcon()!.getAttribute("aria-hidden")).toBe("true");

    for (const [outcome, word, icon] of [
      ["finished", "Finished", CircleCheck],
      ["cancelled", "Cancelled", CircleSlash],
      ["failed", "Run failed", CircleAlert],
    ] as const) {
      send(started(2), finished(outcome));
      expect(text("run-outcome"), outcome).toBe(word);
      expect(outcomeIcon()!.innerHTML, outcome).toBe(shapeOf(icon));
      expect(outcomeIcon()!.getAttribute("class")).not.toContain("animate-spin");
    }
  });

  it("shows Finding tests… with the turning icon while the runner discovers", () => {
    const { send } = setup();
    send(loaded(2), { type: "busy", busy: true });
    expect(text("run-outcome")).toBe("Finding tests…");
    expect(outcomeIcon()!.getAttribute("class")).toContain("animate-spin");
  });

  it("shows the run's environment chip, hides it when there is none, and explains it in a tooltip", () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    const { send } = setup();
    send(loaded(2), started(2));
    const chip = screen.getByTestId("run-env");
    expect(chip.textContent).toBe("");
    expect(chip.parentElement!.className).toContain("has-[>:empty]:hidden");
    send(started(2, "qa"));
    expect(chip.textContent).toBe("qa");
    fireEvent.pointerEnter(chip.parentElement!);
    vi.advanceTimersByTime(500);
    expect(tip().textContent).toBe("Environment of the run");
  });
});

describe("the bar (R14.1)", () => {
  const segments = () => {
    const stripe = screen.queryByTestId("progress-stripe");
    return [...screen.getByTestId("progress-bar").children].filter((c) => c !== stripe);
  };

  it("is 6 px, rounded, with 2 px gaps between passed, failed and cancelled segments", () => {
    const { send } = setup();
    send(loaded(4), started(4), result(1, "passed"), result(2, "failed"), result(3, "interrupted"));
    const bar = screen.getByTestId("progress-bar");
    for (const c of ["h-1.5", "gap-0.5", "rounded-full", "overflow-hidden"])
      expect(bar.className).toContain(c);
    const [passed, failed, cancelled] = segments() as HTMLElement[];
    expect(segments()).toHaveLength(3);
    expect(passed.className).toContain("bg-passed");
    expect(failed.className).toContain("bg-failed");
    expect(cancelled.className).toContain("bg-cancelled");
    expect([passed, failed, cancelled].map((s) => s.style.width)).toEqual(["25%", "25%", "25%"]);
    for (const s of segments()) {
      expect(s.className).toContain("transition-[width]");
      expect(s.className).toContain("motion-reduce:transition-none");
    }
  });

  it("gives a segment its next width only once its last transition has started (Chromium holds one beside a starting spinner)", () => {
    const frames: FrameRequestCallback[] = [];
    vi.stubGlobal("requestAnimationFrame", (cb: FrameRequestCallback) => frames.push(cb));
    const { send } = setup();
    send(loaded(4), started(4), result(1, "passed"));
    const [passed] = segments() as HTMLElement[];
    expect(passed.style.width).toBe("25%");
    let startTime: number | null = null; // waiting for the compositor to start it
    passed.getAnimations = () => [{ startTime } as Animation];
    send(result(2, "passed"));
    expect(text("progress-passed")).toBe("2"); // the words do not wait
    expect(passed.style.width).toBe("25%");
    frames.shift()!(0);
    flush();
    expect(passed.style.width).toBe("25%");
    startTime = 100;
    frames.shift()!(16);
    flush();
    expect(passed.style.width).toBe("50%");
    expect(frames).toHaveLength(0);
  });

  it("keeps the waiting stripe in the running colour, over the segments and still under reduced motion", () => {
    const { send } = setup();
    send(loaded(2), { type: "busy", busy: true });
    const stripe = screen.getByTestId("progress-stripe");
    expect(stripe.className).toContain("animate-slide");
    expect(stripe.className).toContain("motion-reduce:animate-none");
    expect(stripe.className).toContain("bg-running");
    expect(screen.getByTestId("progress-bar").lastElementChild).toBe(stripe);
  });

  it("uses no raw palette class and no dark: variant", () => {
    const { send } = setup();
    send(loaded(2), started(2), result(1, "passed"));
    const all = [...document.querySelectorAll("*")].map((e) => e.getAttribute("class") ?? "");
    expect(all.join(" ")).not.toMatch(/\b(zinc|red|green|sky|amber)-\d|dark:/);
  });
});
