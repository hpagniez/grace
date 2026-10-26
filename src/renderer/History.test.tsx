import { render, screen, cleanup, fireEvent, within } from "@solidjs/testing-library";
import { flush } from "solid-js";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import type { AppEvent } from "../shared/events";
import type { HistoryEntry } from "../shared/history";
import type { Api } from "../shared/ipc";
import { App } from "./App";
import { durationLabel } from "./History";

// jsdom has no modal dialog: open and close only flip the attribute.
beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute("open", "");
  };
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute("open");
  };
});

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
const entry = (id: string, over: Partial<HistoryEntry> = {}): HistoryEntry => ({
  id,
  startedAt: at(25, 14, 32),
  durationMillis: 72_000,
  env: null,
  threads: 1,
  filter: { include: [], exclude: [], hideNonMatching: false },
  selection: { kind: "folder", file: "", name: "clean", line: null, key: "d:" },
  failedOf: null,
  outcome: "finished",
  exitCode: 1,
  passed: 41,
  failed: 2,
  total: 43,
  reportAvailable: true,
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
    rerun: vi.fn(async (_id: string) => {}),
    rerunFailed: vi.fn(async (_id: string) => {}),
    openRunReport: vi.fn(async (_id: string) => {}),
    clearHistory: vi.fn(async () => {}),
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
    tree: null,
    error: null,
  });
  const load = (...runs: HistoryEntry[]) => send({ type: "history", runs });
  return { api, send, load };
}

const click = (el: HTMLElement) => {
  fireEvent.click(el);
  flush();
};
const showHistory = () => click(screen.getByTestId("tab-history"));
const rows = () => screen.queryAllByTestId("history-row");
const inRow = (i: number, id: string) => within(rows()[i]).getByTestId(id) as HTMLButtonElement;
const text = (i: number, id: string) => within(rows()[i]).getByTestId(id).textContent;
const rowButtons = ["history-report", "history-rerun", "history-rerun-failed"] as const;
const toolbarRerunFailed = () => screen.getByTestId("rerun-failed-button") as HTMLButtonElement;
const clear = () => screen.getByTestId("clear-history") as HTMLButtonElement;
const confirm = () => screen.getByTestId("clear-history-dialog") as HTMLDialogElement;
const report = () => screen.getByTestId("report-button") as HTMLButtonElement;
const RUN = "20261025-150000-000";
const started = (): AppEvent => ({
  type: "runStarted",
  selection: "d:",
  runId: RUN,
  keys: [],
  total: 0,
  startedAt: at(25, 15, 0),
  env: null,
});
// the toolbar Report is enabled after a finished run whose report exists
const finished = (): AppEvent => ({
  type: "runFinished",
  outcome: "finished",
  exitCode: 0,
  summary: null,
  reportAvailable: true,
});

describe("durationLabel", () => {
  it("shows Xs under a minute and Xm YYs from a minute", () => {
    expect(durationLabel(0)).toBe("0s");
    expect(durationLabel(999)).toBe("0s");
    expect(durationLabel(18_000)).toBe("18s");
    expect(durationLabel(59_999)).toBe("59s");
    expect(durationLabel(60_000)).toBe("1m 00s");
    expect(durationLabel(72_000)).toBe("1m 12s");
    expect(durationLabel(125_400)).toBe("2m 05s");
  });
});

describe("bottom panel tabs", () => {
  it("starts on Console and switches to History (n) and back", () => {
    const { load } = setup();
    load(entry("a"), entry("b"));
    expect(screen.getByTestId("tab-console").textContent).toBe("Console");
    expect(screen.getByTestId("tab-history").textContent).toBe("History (2)");
    expect(screen.getByTestId("tab-console").getAttribute("aria-selected")).toBe("true");
    expect(screen.getByTestId("console")).toBeTruthy();
    expect(screen.queryByTestId("history")).toBeNull();
    expect(screen.queryByTestId("clear-history")).toBeNull();
    showHistory();
    // a tablist holds tabs only: Clear history sits beside it
    const tablist = screen.getByRole("tablist");
    expect([...tablist.children].map((c) => c.getAttribute("role"))).toEqual(["tab", "tab", "tab"]);
    expect(tablist.contains(clear())).toBe(false);
    expect(screen.getByTestId("tab-history").getAttribute("aria-selected")).toBe("true");
    expect(screen.getByTestId("history")).toBeTruthy();
    expect(screen.queryByTestId("console")).toBeNull();
    click(screen.getByTestId("tab-console"));
    expect(screen.getByTestId("console")).toBeTruthy();
    expect(screen.queryByTestId("history")).toBeNull();
  });

  it("starting a run keeps the History tab", () => {
    const { send } = setup();
    showHistory();
    send({
      type: "runStarted",
      selection: "d:",
      runId: "20261025-150000-000",
      keys: [],
      total: 0,
      startedAt: at(25, 15, 0),
      env: null,
    });
    expect(screen.getByTestId("tab-history").getAttribute("aria-selected")).toBe("true");
    expect(screen.getByTestId("history")).toBeTruthy();
  });
});

describe("history list", () => {
  it("lists runs with labels", () => {
    const { load } = setup();
    load(
      entry("20261025-143200-000", {
        filter: { include: ["smoke"], exclude: ["slow"], hideNonMatching: false },
      }),
      entry("20261024-172000-000", {
        startedAt: at(24, 17, 20),
        env: "qa",
        durationMillis: 18_000,
        outcome: "cancelled",
        exitCode: null,
        passed: 1,
        failed: 0,
        total: 5,
        selection: {
          kind: "scenario",
          file: "tests/users.feature",
          name: "create user",
          line: 6,
          key: "s:tests/users.feature#1",
        },
      }),
      entry("20261020-090500-000", {
        startedAt: at(20, 9, 5),
        durationMillis: 400,
        selection: null,
        failedOf: at(25, 14, 32),
        outcome: "failed",
        exitCode: 3,
      }),
    );
    showHistory();
    expect(rows()).toHaveLength(3);

    expect(text(0, "history-when")).toBe("Today 14:32");
    expect(text(0, "history-env")).toBe("—");
    const chips = within(rows()[0]).getAllByTestId("history-tag");
    expect(chips.map((c) => c.textContent)).toEqual(["@smoke", "not @slow"]);
    expect(text(0, "history-what")).toBe("All");
    expect(text(0, "history-result")).toBe("41 passed, 2 failed of 43");
    expect(text(0, "history-duration")).toBe("1m 12s");

    expect(text(1, "history-when")).toBe("Yesterday 17:20");
    expect(text(1, "history-env")).toBe("qa");
    expect(text(1, "history-tags")).toBe("—");
    expect(within(rows()[1]).queryAllByTestId("history-tag")).toHaveLength(0);
    expect(text(1, "history-what")).toBe("create user");
    expect(within(rows()[1]).getByTestId("history-what").title).toBe(
      "tests/users.feature › create user",
    );
    expect(text(1, "history-result")).toBe("Cancelled, 1 passed, 0 failed of 5");
    expect(text(1, "history-duration")).toBe("18s");

    expect(text(2, "history-when")).toBe("2026-10-20 09:05");
    expect(text(2, "history-what")).toBe("Failed tests of Today 14:32");
    expect(text(2, "history-result")).toBe("Error (exit 3)");
    expect(text(2, "history-duration")).toBe("0s");
  });

  it("shows each run's thread count, or a dash", () => {
    const { load } = setup();
    load(entry("a", { threads: 8 }), entry("b", { threads: null }));
    showHistory();
    expect(
      [...screen.getByTestId("history").querySelectorAll("th")].map((c) => c.textContent),
    ).toEqual(["When", "Env", "Threads", "Tags", "What ran", "Result", "Duration", "Actions"]);
    expect(text(0, "history-threads")).toBe("8");
    expect(text(1, "history-threads")).toBe("—");
  });

  it("puts the path of a feature or folder in the tooltip", () => {
    const { load } = setup();
    load(
      entry("a", {
        selection: {
          kind: "feature",
          file: "tests/users.feature",
          name: "users",
          line: null,
          key: "f:",
        },
      }),
      entry("b", {
        selection: { kind: "folder", file: "tests/api", name: "api", line: null, key: "d:" },
      }),
    );
    showHistory();
    expect(within(rows()[0]).getByTestId("history-what").title).toBe("tests/users.feature");
    expect(text(0, "history-what")).toBe("tests/users.feature");
    expect(within(rows()[1]).getByTestId("history-what").title).toBe("tests/api");
  });

  it("says No runs yet", () => {
    const { load } = setup();
    showHistory();
    expect(screen.getByTestId("history-empty").textContent).toBe("No runs yet");
    expect(screen.getByTestId("tab-history").textContent).toBe("History (0)");
    expect(rows()).toHaveLength(0);
    load(entry("a"));
    expect(screen.queryByTestId("history-empty")).toBeNull();
    expect(rows()).toHaveLength(1);
  });
});

describe("history actions", () => {
  it("Report is disabled without a report", () => {
    const { load } = setup();
    load(entry("a", { reportAvailable: false }), entry("b"));
    showHistory();
    expect(inRow(0, "history-report").disabled).toBe(true);
    expect(inRow(0, "history-rerun").disabled).toBe(false);
    expect(inRow(1, "history-report").disabled).toBe(false);
  });

  it("Re-run failed is disabled with no failure", () => {
    const { load } = setup();
    load(entry("a", { failed: 0, passed: 3, total: 3 }), entry("b"));
    showHistory();
    expect(inRow(0, "history-rerun-failed").disabled).toBe(true);
    expect(inRow(0, "history-rerun").disabled).toBe(false);
    expect(inRow(1, "history-rerun-failed").disabled).toBe(false);
  });

  it("actions call the api with the id", () => {
    const { api, load } = setup();
    load(entry("20261025-143200-000"), entry("20261024-172000-000"));
    showHistory();
    expect(rowButtons.map((id) => inRow(1, id).textContent)).toEqual([
      "Report",
      "Re-run",
      "Re-run failed",
    ]);
    click(inRow(1, "history-report"));
    expect(api.openRunReport).toHaveBeenCalledExactlyOnceWith("20261024-172000-000");
    click(inRow(0, "history-rerun"));
    expect(api.rerun).toHaveBeenCalledExactlyOnceWith("20261025-143200-000");
    click(inRow(1, "history-rerun-failed"));
    expect(api.rerunFailed).toHaveBeenCalledExactlyOnceWith("20261024-172000-000");
  });

  it("all actions are disabled while busy", () => {
    const { send, load } = setup();
    send(started());
    send(finished()); // the toolbar Report is enabled, so only the busy rule can disable it
    load(entry(RUN), entry("b"));
    showHistory();
    const all = () => [
      ...rows().flatMap((_, i) => rowButtons.map((id) => inRow(i, id))),
      toolbarRerunFailed(),
      report(),
      clear(),
    ];
    expect(all().every((b) => !b.disabled)).toBe(true);
    send({ type: "busy", busy: true });
    expect(all().every((b) => b.disabled)).toBe(true);
    send({ type: "busy", busy: false });
    expect(all().every((b) => !b.disabled)).toBe(true);
    // a run in progress: the runner is busy too, but the rule holds on its own
    send(started());
    showHistory(); // switching tabs stays possible
    expect(rows()).toHaveLength(2);
    expect(all().every((b) => b.disabled)).toBe(true);
  });
});

describe("Clear history", () => {
  it("confirms with the count, Cancel does nothing, Delete calls clearHistory", () => {
    const { api, load } = setup();
    load(entry("a"), entry("b"), entry("c"));
    showHistory();
    expect(clear().textContent).toBe("Clear history");
    expect(screen.queryByText(/Delete the/)).toBeNull();
    click(clear());
    expect(confirm().open).toBe(true);
    expect(
      within(confirm()).getByText("Delete the 3 runs of this project and their reports?"),
    ).toBeTruthy();
    expect(screen.getByTestId("clear-history-confirm").textContent).toBe("Delete");
    expect(screen.getByTestId("clear-history-cancel").textContent).toBe("Cancel");

    click(screen.getByTestId("clear-history-cancel"));
    expect(api.clearHistory).not.toHaveBeenCalled();
    expect(confirm().open).toBe(false);
    expect(rows()).toHaveLength(3);

    click(clear());
    click(screen.getByTestId("clear-history-confirm"));
    expect(api.clearHistory).toHaveBeenCalledOnce();
    expect(confirm().open).toBe(false);
  });

  it("words the confirmation for one run in the singular", () => {
    const { load } = setup();
    load(entry("a"));
    showHistory();
    click(clear());
    expect(
      within(confirm()).getByText("Delete the 1 run of this project and its report?"),
    ).toBeTruthy();
  });

  it("ignores a close event that arrives while the dialog is open again", () => {
    const { load } = setup();
    load(entry("a"), entry("b"));
    showHistory();
    click(clear());
    confirm().dispatchEvent(new Event("close")); // late: the dialog was reopened meanwhile
    flush();
    expect(screen.queryByText(/Delete the/)).not.toBeNull();
    confirm().removeAttribute("open"); // Escape
    confirm().dispatchEvent(new Event("close"));
    flush();
    expect(screen.queryByText(/Delete the/)).toBeNull();
  });

  it("disables the toolbar Report and Re-run failed once the history is empty", () => {
    const { send, load } = setup();
    send(started());
    send(finished());
    load(entry(RUN));
    expect(report().disabled).toBe(false);
    expect(toolbarRerunFailed().disabled).toBe(false);
    showHistory();
    click(clear());
    click(screen.getByTestId("clear-history-confirm"));
    load(); // main answers Delete with the empty list
    expect(report().disabled).toBe(true);
    expect(toolbarRerunFailed().disabled).toBe(true);
  });

  it("is disabled with no runs", () => {
    setup();
    showHistory();
    expect(clear().disabled).toBe(true);
  });
});

describe("toolbar Re-run failed", () => {
  it("shows the newest run's failed count and calls rerunFailed with its id", () => {
    const { api, load } = setup();
    load(entry("new", { failed: 2 }), entry("old", { failed: 5 }));
    expect(toolbarRerunFailed().textContent).toBe("Re-run failed (2)");
    expect(toolbarRerunFailed().disabled).toBe(false);
    click(toolbarRerunFailed());
    expect(api.rerunFailed).toHaveBeenCalledExactlyOnceWith("new");
  });

  it("sits right after Run all", () => {
    setup();
    expect(screen.getByTestId("run-all-button").nextElementSibling).toBe(toolbarRerunFailed());
  });

  it("is disabled with no history, with no failure in the newest run, and while busy", () => {
    const { send, load } = setup();
    expect(toolbarRerunFailed().textContent).toBe("Re-run failed (0)");
    expect(toolbarRerunFailed().disabled).toBe(true);
    load(entry("new", { failed: 0 }), entry("old", { failed: 5 }));
    expect(toolbarRerunFailed().disabled).toBe(true);
    load(entry("new", { failed: 1 }));
    expect(toolbarRerunFailed().disabled).toBe(false);
    send({ type: "busy", busy: true });
    expect(toolbarRerunFailed().disabled).toBe(true);
    send({ type: "busy", busy: false });
    load();
    expect(toolbarRerunFailed().disabled).toBe(true);
  });
});
