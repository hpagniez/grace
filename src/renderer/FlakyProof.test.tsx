import { render, screen, cleanup, fireEvent, within } from "@solidjs/testing-library";
import { CircleCheck, CircleX, Shuffle, type IconNode } from "lucide";
import { flush } from "solid-js";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AppEvent } from "../shared/events";
import type { FlakyEnv, FlakyRun } from "../shared/flaky";
import type { Api } from "../shared/ipc";
import type { TestNode } from "../shared/model";
import { App } from "./App";

// "now" is Sunday 2026-10-25 15:00 local time; only Date is faked so flush() keeps working
beforeEach(() => {
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(new Date(2026, 9, 25, 15, 0));
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

const at = (day: number, h: number, m: number) => new Date(2026, 9, day, h, m).getTime();
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
const COIN = "s:tests/x.feature#1";
const tree = (): TestNode =>
  node("folder", "d:", "p", {
    children: [
      node("feature", "f:tests/x.feature", "x", {
        children: [
          node("scenario", COIN, "coin", { line: 3, tags: ["slow"] }),
          node("scenario", "s:tests/x.feature#2", "steady", { line: 6 }),
        ],
      }),
    ],
  });

const run = (id: string, over: Partial<FlakyRun> = {}): FlakyRun => ({
  id,
  startedAt: at(25, 14, 32),
  status: "passed",
  threads: 1,
  reportAvailable: true,
  ...over,
});
const env = (name: string | null, passed: number, failed: number, runs: FlakyRun[] = []) =>
  ({ env: name, passed, failed, runs }) satisfies FlakyEnv;

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
    openRunReport: vi.fn(async (_id: string) => {}),
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
  send({
    type: "environment",
    view: {
      env: "qa",
      suggestions: [],
      overlays: [],
      properties: { shared: [], byEnv: {} },
      secretsAvailable: true,
    },
  });
  const select = (name: string) => {
    fireEvent.click(screen.getByText(name));
    flush();
  };
  select("coin");
  const flaky = (...envs: FlakyEnv[]) => send({ type: "flaky", tests: [{ key: COIN, envs }] });
  return { api, send, select, flaky };
}

/** Whether the svg holds this icon: every shape of the Lucide icon is in it. */
const drawn = (svg: Element | null | undefined, icon: IconNode) =>
  !!svg &&
  icon.every(([, attrs]) => Object.values(attrs).every((v) => svg.innerHTML.includes(String(v))));
const boxes = () => screen.queryAllByTestId("flaky-proof");
const runs = (box: HTMLElement) => within(box).queryAllByTestId("flaky-run");
const reports = () => screen.queryAllByTestId("flaky-run-report") as HTMLButtonElement[];

describe("flaky proof in the details pane", () => {
  it("orders the boxes and words them", () => {
    const { flaky } = setup();
    flaky(env("dev", 1, 1), env(null, 3, 2), env("qa", 1, 2));
    const view = boxes().map((b) => [
      within(b).getByRole("heading").textContent,
      within(b).getByText(/^Passed /).textContent,
    ]);
    expect(view).toEqual([
      ["Flaky on qa", "Passed 1 time, failed 2 times with the same project files"],
      ["Flaky with no environment", "Passed 3 times, failed 2 times with the same project files"],
      ["Flaky on dev", "Passed 1 time, failed 1 time with the same project files"],
    ]);
  });

  it("sits after the tags and before the error", () => {
    const { send, flaky } = setup();
    flaky(env("qa", 1, 1, [run("a")]));
    send({
      type: "runStarted",
      selection: COIN,
      runId: "20261025-150000-000",
      keys: [COIN],
      total: 1,
      startedAt: at(25, 15, 0),
      env: "qa",
    });
    send({ type: "scenarios", updates: [{ key: COIN, status: "failed", error: "boom" }] });
    send({
      type: "runFinished",
      outcome: "finished",
      exitCode: 1,
      summary: null,
      reportAvailable: false,
    });
    const order = [...screen.getByTestId("details").querySelectorAll("[data-testid]")].map(
      (n) => (n as HTMLElement).dataset.testid,
    );
    expect(order.indexOf("details-location")).toBeLessThan(order.indexOf("flaky-proof"));
    expect(order.indexOf("flaky-proof")).toBeLessThan(order.indexOf("details-error"));
    // the tags have no test id
    const tag = within(screen.getByTestId("details")).getByText("slow");
    expect(tag.compareDocumentPosition(boxes()[0]) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it("lists the proof runs, newest first", () => {
    const { flaky } = setup();
    flaky(
      env("qa", 1, 2, [
        run("c", { startedAt: at(25, 14, 32), status: "failed", threads: 1 }),
        run("b", { startedAt: at(24, 17, 20), status: "passed", threads: 8 }),
        run("a", { startedAt: at(21, 9, 5), status: "failed", threads: null }),
      ]),
    );
    const rows = runs(boxes()[0]);
    expect(rows.map((r) => r.dataset.id)).toEqual(["c", "b", "a"]);
    expect(rows.map((r) => r.dataset.status)).toEqual(["failed", "passed", "failed"]);
    const cells = rows.map((r) =>
      within(r)
        .getAllByText(/./)
        .map((c) => c.textContent),
    );
    expect(cells).toEqual([
      ["Today 14:32", "Failed", "1 thread", "Report"],
      ["Yesterday 17:20", "Passed", "8 threads", "Report"],
      ["2026-10-21 09:05", "Failed", "—", "Report"],
    ]);
  });

  it("boxes the proof in the cancelled tint, headed by the shuffle icon", () => {
    const { flaky } = setup();
    flaky(env("qa", 1, 1));
    const box = boxes()[0];
    expect(box.className).toContain("rounded-lg");
    expect(box.className).toContain("bg-cancelled-tint");
    expect(drawn(within(box).getByRole("heading").querySelector("svg"), Shuffle)).toBe(true);
    expect(box.outerHTML).not.toMatch(/\b(zinc|red|green|sky|amber)-\d|dark:|title=/);
  });

  it("marks each proof run by its status: a hidden mark, then the word that names it", () => {
    const { flaky } = setup();
    flaky(env("qa", 1, 1, [run("b", { status: "failed" }), run("a", { status: "passed" })]));
    const rows = runs(boxes()[0]);
    expect(within(boxes()[0]).queryAllByRole("img")).toHaveLength(0);
    const marks = rows.map((r) => r.querySelector<HTMLElement>("[data-status]")!);
    expect(
      marks.every((m) => m.getAttribute("aria-hidden") === "true" && !m.hasAttribute("role")),
    ).toBe(true);
    expect(rows.map((r) => r.textContent)).toEqual([
      expect.stringContaining("Failed"),
      expect.stringContaining("Passed"),
    ]);
    expect(drawn(marks[0].querySelector("svg"), CircleX)).toBe(true);
    expect(drawn(marks[1].querySelector("svg"), CircleCheck)).toBe(true);
    expect(rows.map((r) => r.dataset.status)).toEqual(["failed", "passed"]); // tests read this
  });

  it("opens a run's report, only when it has one and the runner is idle", () => {
    const { api, send, flaky } = setup();
    flaky(env("qa", 1, 1, [run("b"), run("a", { status: "failed", reportAvailable: false })]));
    expect(reports().map((b) => b.disabled)).toEqual([false, true]);
    fireEvent.click(reports()[0]);
    expect(api.openRunReport).toHaveBeenCalledExactlyOnceWith("b");
    fireEvent.click(reports()[1]);
    expect(api.openRunReport).toHaveBeenCalledTimes(1);
    send({ type: "busy", busy: true });
    expect(reports().map((b) => b.disabled)).toEqual([true, true]);
    send({ type: "busy", busy: false });
    expect(reports().map((b) => b.disabled)).toEqual([false, true]);
    send({
      type: "runStarted",
      selection: COIN,
      runId: "20261025-150000-000",
      keys: [COIN],
      total: 1,
      startedAt: at(25, 15, 0),
      env: "qa",
    });
    expect(reports().map((b) => b.disabled)).toEqual([true, true]); // a run in progress locks them
    send({
      type: "runFinished",
      outcome: "finished",
      exitCode: 0,
      summary: null,
      reportAvailable: false,
    });
    expect(reports().map((b) => b.disabled)).toEqual([false, true]);
  });

  it("follows the environment in force", () => {
    const { send, select, flaky } = setup();
    flaky(env("dev", 1, 1), env("qa", 1, 1));
    expect(boxes().map((b) => within(b).getByRole("heading").textContent)).toEqual([
      "Flaky on qa",
      "Flaky on dev",
    ]);
    send({
      type: "environment",
      view: {
        env: "dev",
        suggestions: [],
        overlays: [],
        properties: { shared: [], byEnv: {} },
        secretsAvailable: true,
      },
    });
    select("coin"); // another environment clears the selection, not the proof
    expect(boxes().map((b) => within(b).getByRole("heading").textContent)).toEqual([
      "Flaky on dev",
      "Flaky on qa",
    ]);
  });

  it("shows no proof for a test that is not flaky", () => {
    const { select, flaky } = setup();
    expect(boxes()).toHaveLength(0);
    flaky(env("qa", 1, 1));
    expect(boxes()).toHaveLength(1);
    select("steady");
    expect(boxes()).toHaveLength(0);
    select("x.feature");
    expect(boxes()).toHaveLength(0);
  });
});
