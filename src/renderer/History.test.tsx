import { render, screen, cleanup, fireEvent, within } from "@solidjs/testing-library";
import {
  Check,
  FileText,
  History as HistoryIcon,
  Minus,
  Play,
  RotateCcw,
  type IconNode,
} from "lucide";
import { flush } from "solid-js";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import type { AppEvent } from "../shared/events";
import type { HistoryEntry } from "../shared/history";
import type { Api } from "../shared/ipc";
import { App } from "./App";
import { btnDialog, dangerDialog, ghostSmall } from "./buttons";
import { durationLabel } from "./History";
import { chipExcluded, chipIncluded, dialog as dialogStyle } from "./ui";

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

/** Whether `svg` draws `icon`: every attribute of its shapes is in the markup. */
const drawn = (svg: Element | null | undefined, icon: IconNode) =>
  !!svg &&
  icon.every(([, attrs]) => Object.values(attrs).every((v) => svg.innerHTML.includes(String(v))));
const cell = (i: number, id: string) => within(rows()[i]).getByTestId(id);
const RAW = /\b(zinc|red|green|sky|amber|slate|gray)-\d|dark:| title=/; // R9.3

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

  describe("What ran", () => {
    // Only the timers the tooltip needs, with Date still on the test's "now".
    beforeEach(() => {
      vi.useFakeTimers({ toFake: ["Date", "setTimeout", "clearTimeout"] });
      vi.setSystemTime(new Date(2026, 9, 25, 15, 0));
    });
    const tip = () => document.querySelector("[role=tooltip]");
    const what = (i: number) => cell(i, "history-what");
    const rest = (el: HTMLElement) => {
      fireEvent.pointerEnter(el.parentElement!); // the tooltip's wrapper
      vi.advanceTimersByTime(500);
    };

    it("puts where the node sits in a tooltip, not in a title", () => {
      const { load } = setup();
      load(
        entry("a", {
          selection: {
            kind: "scenario",
            file: "tests/users.feature",
            name: "create user",
            line: 6,
            key: "s:tests/users.feature#1",
          },
        }),
        entry("b", {
          selection: {
            kind: "feature",
            file: "tests/users.feature",
            name: "users",
            line: null,
            key: "f:",
          },
        }),
        entry("c", {
          selection: { kind: "folder", file: "tests/api", name: "api", line: null, key: "d:" },
        }),
      );
      showHistory();
      expect(text(0, "history-what")).toBe("create user");
      expect(text(1, "history-what")).toBe("tests/users.feature");
      for (const [i, path] of [
        [0, "tests/users.feature › create user"],
        [1, "tests/users.feature"],
        [2, "tests/api"],
      ] as const) {
        expect(what(i).hasAttribute("title")).toBe(false);
        rest(what(i));
        expect(tip()!.textContent, `row ${i}`).toBe(path);
        fireEvent.pointerLeave(what(i).parentElement!);
        flush();
      }
      expect(screen.getByTestId("history").querySelectorAll("[title]")).toHaveLength(0);
    });

    it("says nothing for a run that has no path (All, Failed tests)", () => {
      const { load } = setup();
      load(entry("a"), entry("b", { selection: null }));
      showHistory();
      for (const i of [0, 1]) {
        rest(what(i));
        expect(tip(), `row ${i}`).toBeNull();
      }
    });

    it("cuts a long label at 14 rem, inside a wrapper that can shrink", () => {
      const { load } = setup();
      load(entry("a"));
      showHistory();
      expect(what(0).className).toContain("truncate");
      expect(what(0).className).toContain("min-w-0");
      expect(what(0).parentElement!.className).toContain("min-w-0");
      expect(what(0).parentElement!.className).toContain("max-w-56");
    });
  });

  it("shows a status mark before the words of the result, and the mark is hidden", () => {
    const { load } = setup();
    load(
      entry("a"), // finished with 2 failed
      entry("b", { failed: 0, passed: 3, total: 3 }),
      entry("c", { outcome: "cancelled", exitCode: null, failed: 0, passed: 1, total: 5 }),
      entry("d", { outcome: "failed", exitCode: 3 }),
    );
    showHistory();
    const marks = [0, 1, 2, 3].map((i) =>
      cell(i, "history-result").querySelector<HTMLElement>("[data-status]")!,
    );
    expect(marks.map((m) => m.dataset.status)).toEqual(["failed", "passed", "cancelled", "failed"]);
    // the words name the status: the mark is aria-hidden, with no role and no name
    for (const m of marks) {
      expect(m.getAttribute("aria-hidden")).toBe("true");
      expect(m.hasAttribute("role")).toBe(false);
    }
    expect(within(screen.getByTestId("history")).queryAllByRole("img")).toHaveLength(0);
    // a block flex: inline-flex would sit on the baseline and lift the words above the rest of the row
    expect(cell(0, "history-result").firstElementChild!.className).toMatch(/(^| )flex /);
    expect(cell(0, "history-result").className).toContain("tabular-nums"); // R2.4: it holds counts
    expect(text(0, "history-result")).toBe("41 passed, 2 failed of 43");
    expect(text(1, "history-result")).toBe("3 passed, 0 failed of 3");
    expect(text(2, "history-result")).toBe("Cancelled, 1 passed, 0 failed of 5");
    expect(text(3, "history-result")).toBe("Error (exit 3)");
  });

  it("draws a tag by its shape: included with a check, excluded with a minus and a strike", () => {
    const { load } = setup();
    load(entry("a", { filter: { include: ["smoke"], exclude: ["slow"], hideNonMatching: false } }));
    showHistory();
    const [inc, exc] = within(rows()[0]).getAllByTestId("history-tag");
    expect(inc.className).toContain(chipIncluded);
    expect(drawn(inc.querySelector("svg"), Check)).toBe(true);
    expect(exc.className).toContain(chipExcluded);
    expect(exc.className).toContain("line-through");
    expect(drawn(exc.querySelector("svg"), Minus)).toBe(true);
    expect(inc.textContent).toBe("@smoke");
    expect(exc.textContent).toBe("not @slow");
  });

  it("right-aligns Threads and Duration in tabular figures", () => {
    const { load } = setup();
    load(entry("a", { threads: 8 }));
    showHistory();
    const heads = [...screen.getByTestId("history").querySelectorAll("th")];
    for (const name of ["Threads", "Duration"])
      expect(heads.find((h) => h.textContent === name)!.className, name).toContain("text-right");
    for (const id of ["history-threads", "history-duration"]) {
      expect(cell(0, id).className, id).toContain("text-right");
      expect(cell(0, id).className, id).toContain("tabular-nums");
    }
    expect(cell(0, "history-when").className).toContain("tabular-nums");
    expect(cell(0, "history-duration").className).toContain("font-mono");
  });

  it("sticks muted sentence-case headers to the top, and gives rows room under them", () => {
    const { load } = setup();
    load(entry("a"));
    showHistory();
    for (const th of screen.getByTestId("history").querySelectorAll("th"))
      for (const c of [
        "sticky",
        "top-0",
        "bg-surface",
        "text-xs",
        "text-muted",
        "font-medium",
        "h-7",
      ])
        expect(th.className, `${th.textContent} ${c}`).toContain(c);
    // 28 px rows (R18.4). A focused button is scrolled clear of the 28 px header by the scroller's
    // scroll-padding, which a row's scroll-margin alone does not do (R10.2; checked in Chromium).
    for (const c of ["h-7", "border-b", "border-line", "scroll-mt-7"])
      expect(rows()[0].className).toContain(c);
    expect(screen.getByTestId("history").className).toContain("scroll-pt-7");
    expect(rows()[0].className).not.toMatch(/\b(odd|even):/); // no zebra
  });

  it("sets the table cells in 12 px (R2.2)", () => {
    const { load } = setup();
    load(entry("a"));
    showHistory();
    const table = screen.getByTestId("history").querySelector("table")!;
    expect(table.className).toContain("text-xs");
    expect(table.className).not.toContain("text-sm");
  });

  it("uses no raw palette class, no dark: variant and no title", () => {
    const { load } = setup();
    load(entry("a", { filter: { include: ["smoke"], exclude: ["slow"], hideNonMatching: false } }));
    showHistory();
    expect(screen.getByTestId("history").outerHTML).not.toMatch(RAW);
  });

  it("says No runs yet in an empty state with the history icon", () => {
    const { load } = setup();
    showHistory();
    const empty = screen.getByTestId("history-empty");
    expect(empty.textContent).toBe("No runs yet. Runs appear here when they finish.");
    const icon = empty.parentElement!.querySelector("svg")!;
    expect(drawn(icon, HistoryIcon)).toBe(true);
    expect(icon.getAttribute("width")).toBe("24");
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
    // small ghost buttons, each with its icon
    for (const id of rowButtons) expect(inRow(1, id).className, id).toContain(ghostSmall);
    expect(drawn(inRow(1, "history-report").querySelector("svg"), FileText)).toBe(true);
    expect(drawn(inRow(1, "history-rerun").querySelector("svg"), Play)).toBe(true);
    expect(drawn(inRow(1, "history-rerun-failed").querySelector("svg"), RotateCcw)).toBe(true);
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
    expect(confirm().open).toBe(false);
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

  it("is a dialog-style box: a 16 px title, Delete (danger) first, then Cancel (focused)", () => {
    const { load } = setup();
    load(entry("a"), entry("b"));
    showHistory();
    click(clear());
    expect(confirm().className).toContain(dialogStyle);
    expect(confirm().getAttribute("aria-labelledby")).toBe("clear-history-title");
    const title = document.getElementById("clear-history-title")!;
    expect(title.className).toContain("text-lg");
    expect(title.className).toContain("font-semibold");
    const del = screen.getByTestId("clear-history-confirm");
    const cancel = screen.getByTestId("clear-history-cancel");
    expect(del.className).toContain(dangerDialog);
    expect(cancel.className).toContain(btnDialog);
    expect(del.compareDocumentPosition(cancel) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(cancel.hasAttribute("autofocus")).toBe(true);
    // R2.2: dialog buttons are 14 px: the dialog variants, not an override of the standard ones
    for (const b of [del, cancel]) {
      expect(b.className).toContain("text-base");
      expect(b.className).not.toMatch(/text-sm|!/);
    }
    expect(confirm().outerHTML).not.toMatch(RAW);
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

  // The dialog fades out over 100 ms (R5.2): its title and buttons must stay until it is gone.
  describe("while it closes", () => {
    const whole = (runs: string) => {
      expect(within(confirm()).getByText(runs)).toBeTruthy();
      expect(within(confirm()).getByTestId("clear-history-confirm")).toBeTruthy();
      expect(within(confirm()).getByTestId("clear-history-cancel")).toBeTruthy();
    };
    const THREE = "Delete the 3 runs of this project and their reports?";

    it("keeps its title and buttons after Cancel and after Escape", () => {
      const { load } = setup();
      load(entry("a"), entry("b"), entry("c"));
      showHistory();
      click(clear());
      click(screen.getByTestId("clear-history-cancel"));
      expect(confirm().open).toBe(false);
      whole(THREE);
      click(clear());
      confirm().removeAttribute("open"); // Escape: the browser closes the dialog itself
      confirm().dispatchEvent(new Event("close"));
      flush();
      whole(THREE);
    });

    it("keeps the count it was opened with when main answers Delete with an empty list", () => {
      const { api, load } = setup();
      load(entry("a"), entry("b"), entry("c"));
      showHistory();
      click(clear());
      click(screen.getByTestId("clear-history-confirm"));
      expect(api.clearHistory).toHaveBeenCalledOnce();
      load(); // main's reply, while the dialog is still fading out
      expect(rows()).toHaveLength(0);
      whole(THREE); // not "Delete the 0 runs of this project and their reports?"
    });

    it("counts again each time it opens", () => {
      const { load } = setup();
      load(entry("a"), entry("b"), entry("c"));
      showHistory();
      click(clear());
      click(screen.getByTestId("clear-history-cancel"));
      load(entry("a"), entry("b"), entry("c"), entry("d"));
      click(clear());
      whole("Delete the 4 runs of this project and their reports?");
    });
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
    expect(toolbarRerunFailed().textContent).toBe("Re-run failed 2");
    expect(toolbarRerunFailed().disabled).toBe(false);
    click(toolbarRerunFailed());
    expect(api.rerunFailed).toHaveBeenCalledExactlyOnceWith("new");
  });

  it("sits between Report and Cancel, before Run all (spec R13.1)", () => {
    setup();
    const slot = (el: HTMLElement) => el.parentElement!; // each sits in its tooltip's wrapper
    expect(slot(toolbarRerunFailed()).previousElementSibling).toBe(slot(report()));
    expect(slot(toolbarRerunFailed()).nextElementSibling).toBe(
      slot(screen.getByTestId("cancel-button")),
    );
    expect(slot(screen.getByTestId("cancel-button")).nextElementSibling).toBe(
      screen.getByTestId("run-all-button"),
    );
  });

  it("is disabled with no history, with no failure in the newest run, and while busy", () => {
    const { send, load } = setup();
    expect(toolbarRerunFailed().textContent).toBe("Re-run failed 0");
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
