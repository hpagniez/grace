import { cleanup, fireEvent, render, screen } from "@solidjs/testing-library";
import { createStore, flush } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AppEvent } from "../shared/events";
import type { Api } from "../shared/ipc";
import { btn, btnIcon, ghost, NARROW_QUERY, narrowIcon, narrowLabel, primary } from "./buttons";
import { initialState, reduce } from "./reduce";
import { NAME_TAIL, Toolbar } from "./Toolbar";
import { countPill } from "./ui";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  delete (window as { matchMedia?: unknown }).matchMedia; // jsdom has none
});

/** A `matchMedia` that answers `matches` for the narrow query, and tells its listeners when it changes. */
function media(matches: boolean) {
  const listeners = new Set<() => void>();
  const query = {
    matches,
    addEventListener: vi.fn((_: string, f: () => void) => listeners.add(f)),
    removeEventListener: vi.fn((_: string, f: () => void) => listeners.delete(f)),
  };
  const ask = vi.fn(() => query);
  window.matchMedia = ask as unknown as typeof window.matchMedia;
  return {
    query,
    ask,
    listeners,
    change: (now: boolean) => {
      query.matches = now;
      for (const f of listeners) f();
      flush();
    },
  };
}

const ROOT = "D:\\work\\petstore-api";

/** A finished run of 5 tests, `failed` of which failed. */
const failedRun = (failed: number) => ({
  id: "r1",
  startedAt: 1,
  durationMillis: 2,
  env: null,
  threads: 1,
  filter: { include: [], exclude: [], hideNonMatching: false },
  selection: null,
  failedOf: null,
  outcome: "finished" as const,
  exitCode: failed > 0 ? 1 : 0,
  passed: 5 - failed,
  failed,
  total: 5,
  reportAvailable: true,
});

function setup(root = ROOT) {
  const [state, set] = createStore(initialState());
  const api = {
    closeProject: vi.fn(async () => {}),
    openProject: vi.fn(async () => {}),
    refresh: vi.fn(async () => {}),
    run: vi.fn(async () => {}),
    cancel: vi.fn(async () => {}),
    openReport: vi.fn(async () => {}),
    rerunFailed: vi.fn(async () => {}),
    debugCommand: vi.fn(async () => {}),
    setWatch: vi.fn(async (_on: boolean) => {}),
  };
  render(() => <Toolbar state={state} api={api as unknown as Api} />);
  const send = (...events: AppEvent[]) => {
    for (const e of events) set((draft) => reduce(draft, e));
    flush();
  };
  send({
    type: "project",
    project: { root, layout: "maven" },
    tree: {
      key: "d:",
      kind: "folder",
      name: "p",
      file: "",
      line: null,
      refId: null,
      tags: [],
      children: [
        {
          key: "s:a#1",
          kind: "scenario",
          name: "a",
          file: "a.feature",
          line: 1,
          refId: null,
          tags: [],
          children: [],
        },
      ],
    },
    error: null,
  });
  return { api, send };
}

const header = () => document.querySelector("header")!;
const button = (id: string) => screen.getByTestId(id) as HTMLButtonElement;
/** The wrapper a Tooltip puts around its target. */
const wrapperOf = (el: HTMLElement) => el.closest("span.inline-flex") as HTMLElement;
/** The open tooltip: a popover, which jsdom does not count as accessible. */
const tip = () => document.querySelector('[role="tooltip"]')!;
const hover = (el: HTMLElement) => {
  fireEvent.pointerEnter(wrapperOf(el));
  vi.advanceTimersByTime(500);
};

describe("order and shape (R13.1)", () => {
  it("is one 44 px row on the ground", () => {
    setup();
    const cls = header().className;
    for (const c of ["min-h-11", "bg-ground", "items-center"]) expect(cls).toContain(c);
    expect(cls).not.toMatch(/dark:|zinc/);
  });

  it("puts the controls in the order of the spec, Run all last", () => {
    setup();
    const wanted = [
      "home-button",
      "project-name",
      "project-layout",
      "env-input",
      "threads-input",
      "properties-button",
      "refresh-button",
      "watch-button",
      "open-button",
      "report-button",
      "rerun-failed-button",
      "cancel-button",
      "run-all-button",
    ];
    const ids = [...header().querySelectorAll("[data-testid]")]
      .map((e) => e.getAttribute("data-testid")!)
      .filter((id) => wanted.includes(id));
    expect(ids).toEqual(wanted);
  });

  it("separates Properties from Refresh and Open from Report with 1 px dividers", () => {
    setup();
    const kinds = [...header().children]
      .filter((c) => c.tagName !== "DIALOG") // Properties' dialog follows its button
      .map((c) =>
        c.getAttribute("aria-hidden") === "true" && c.className.includes("w-px")
          ? "divider"
          : (c.querySelector("[data-testid]")?.getAttribute("data-testid") ??
            c.getAttribute("data-testid")),
      );
    const at = (id: string) => kinds.indexOf(id);
    expect(kinds.filter((k) => k === "divider")).toHaveLength(2);
    expect(kinds[at("properties-button") + 1]).toBe("divider");
    expect(kinds[at("open-button") + 1]).toBe("divider");
    const divider = header().querySelector("[aria-hidden=true].w-px")!;
    for (const c of ["h-5", "w-px", "bg-line"]) expect(divider.className).toContain(c);
  });

  it("every button keeps its test id and its label, and is 28 px tall or an icon of 28", () => {
    setup();
    const label = (id: string) => button(id).textContent;
    expect(label("open-button")).toBe("Open…");
    expect(label("report-button")).toBe("Report");
    expect(label("cancel-button")).toBe("Cancel");
    expect(label("run-all-button")).toBe("Run all");
    expect(label("properties-button")).toBe("Properties…");
    expect(button("open-button").className).toContain("h-7");
    expect(button("refresh-button").className).toContain("size-7");
    expect(button("home-button").className).toContain("size-7");
  });
});

describe("a narrow window", () => {
  const LABELLED = [
    ["properties-button", "Properties…"],
    ["open-button", "Open…"],
    ["report-button", "Report"],
    ["rerun-failed-button", "Re-run failed"],
    ["cancel-button", "Cancel"],
  ] as const;

  it("never wraps a button's label", () => {
    for (const style of [btn, primary, ghost, btnIcon])
      expect(style).toContain("whitespace-nowrap");
  });

  it("below 1280 px hides the label of five buttons from sight but not from their names", () => {
    setup();
    for (const [id, label] of LABELLED) {
      const spoken = button(id).querySelector(`.${CSS.escape(narrowLabel)}`)!;
      expect(spoken.textContent, id).toBe(label);
      expect(spoken.tagName).toBe("SPAN");
      expect(narrowLabel).toBe("max-[1280px]:sr-only");
    }
    // the names: a label (Re-run failed adds its count once there is one)
    for (const name of ["Properties…", "Open…", "Report", "Cancel", "Re-run failed"])
      expect(screen.getByRole("button", { name }), name).toBeTruthy();
    expect(screen.getByRole("button", { name: "Home" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Refresh" })).toBeTruthy();
  });

  it("shrinks four of them to a 28 px icon and keeps the count pill of Re-run failed", () => {
    const { send } = setup();
    for (const id of ["properties-button", "open-button", "report-button", "cancel-button"])
      for (const c of narrowIcon.split(" ")) expect(button(id).className, id).toContain(c);
    expect(button("rerun-failed-button").className).toContain("max-[1280px]:px-2");
    send({ type: "history", runs: [failedRun(2)] });
    const spans = [...button("rerun-failed-button").querySelectorAll("span")];
    expect(spans.at(-1)!.textContent).toBe("2"); // the pill follows the spoken label
  });

  it("keeps Run all's label in sight", () => {
    setup();
    const run = button("run-all-button");
    expect(run.textContent).toBe("Run all");
    expect(run.querySelector("span")).toBeNull();
    expect(run.className).not.toContain("max-[1280px]");
  });

  it("asks for the width the classes use", () => {
    expect(NARROW_QUERY).toBe("(width < 1280px)"); // Tailwind's max-[1280px] is below 1280 px
    for (const c of [narrowIcon, narrowLabel]) expect(c).toContain("max-[1280px]:");
  });

  it("gives each of the five a tooltip with the label while the label is hidden, also when disabled", () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    const m = media(true);
    setup();
    expect(m.ask).toHaveBeenCalledWith(NARROW_QUERY);
    for (const [id, label] of LABELLED) {
      hover(button(id));
      expect(tip().textContent, id).toBe(label);
      fireEvent.pointerLeave(wrapperOf(button(id)));
      flush();
    }
    expect(button("report-button").disabled).toBe(true);
  });

  it("gives them no tooltip while the label shows, which a tooltip would only repeat", () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    media(false);
    setup();
    for (const [id] of LABELLED) {
      hover(button(id));
      expect(document.querySelector('[role="tooltip"]'), id).toBeNull();
      fireEvent.pointerLeave(wrapperOf(button(id)));
    }
    // the icon-only Home and Refresh and the paths keep theirs at any width
    hover(button("refresh-button"));
    expect(tip().textContent).toBe("Refresh");
  });

  it("follows the window as it is resized, and stops listening with the toolbar", () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    const m = media(false);
    setup();
    hover(button("open-button"));
    expect(document.querySelector('[role="tooltip"]')).toBeNull();
    m.change(true);
    hover(button("open-button"));
    expect(tip().textContent).toBe("Open…");
    fireEvent.pointerLeave(wrapperOf(button("open-button")));
    flush();
    m.change(false);
    hover(button("open-button"));
    expect(document.querySelector('[role="tooltip"]')).toBeNull();
    // the toolbar, the Properties button and the Env and Threads boxes each listen
    expect(m.listeners.size).toBe(4);
    cleanup();
    expect(m.listeners.size).toBe(0);
    expect(m.query.removeEventListener).toHaveBeenCalledTimes(4);
  });

  const BOXES = [
    ["env-input", "Env", "combobox"],
    ["threads-input", "Threads", "spinbutton"],
  ] as const;

  it("below 1280 px hides the words Env and Threads from sight but not from their boxes' names", () => {
    setup();
    for (const [id, word, role] of BOXES) {
      const label = screen.getByTestId(id).closest("label")!;
      const spoken = label.querySelector(`.${CSS.escape(narrowLabel)}`)!;
      expect(spoken.tagName, id).toBe("SPAN");
      expect(spoken.textContent, id).toBe(word);
      expect(screen.getByRole(role, { name: word })).toBe(screen.getByTestId(id));
    }
  });

  it("then says Env and Threads in a tooltip beside the box, and nothing while the words show", () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    const m = media(true);
    setup();
    for (const [id, word] of BOXES) {
      hover(screen.getByTestId(id));
      expect(tip().textContent, id).toBe(word); // left of the box: checked in e2e, jsdom has no layout
      fireEvent.pointerLeave(wrapperOf(screen.getByTestId(id)));
      flush();
    }
    m.change(false);
    for (const [id] of BOXES) {
      hover(screen.getByTestId(id));
      expect(document.querySelector('[role="tooltip"]'), id).toBeNull();
      fireEvent.pointerLeave(wrapperOf(screen.getByTestId(id)));
    }
  });
});

describe("Home and Refresh, the icon-only buttons", () => {
  it("Home holds the logo, is named Home and shows that in a tooltip", () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    setup();
    const home = screen.getByRole("button", { name: "Home" });
    expect(home).toBe(button("home-button"));
    const logo = home.querySelector("img")!;
    expect(logo.getAttribute("src")).toContain("icon-small");
    expect(logo.getAttribute("alt")).toBe("");
    expect(logo.getAttribute("width")).toBe("20");
    expect(logo.getAttribute("height")).toBe("20");
    hover(home);
    expect(tip().textContent).toBe("Home");
  });

  it("Home goes home, and is off while busy", () => {
    const { api, send } = setup();
    fireEvent.click(button("home-button"));
    expect(api.closeProject).toHaveBeenCalledTimes(1);
    send({ type: "busy", busy: true });
    expect(button("home-button").disabled).toBe(true);
  });

  it("Refresh is named Refresh, shows an icon and a tooltip with that word, also while disabled", () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    const { api, send } = setup();
    const refresh = screen.getByRole("button", { name: "Refresh" }) as HTMLButtonElement;
    expect(refresh).toBe(button("refresh-button"));
    expect(refresh.textContent).toBe("");
    expect(refresh.querySelector("svg")!.getAttribute("aria-hidden")).toBe("true");
    fireEvent.click(refresh);
    expect(api.refresh).toHaveBeenCalledTimes(1);
    send({ type: "busy", busy: true });
    expect(refresh.disabled).toBe(true);
    hover(refresh);
    expect(tip().textContent).toBe("Refresh");
  });
});

describe("the project", () => {
  it("shows the folder name and the layout, and leaves the path to the status bar (spec R1.1)", () => {
    setup();
    expect(screen.getByTestId("project-name").textContent).toBe("petstore-api");
    expect(screen.getByTestId("project-name").className).toContain("font-medium");
    expect(screen.getByTestId("project-layout").textContent).toBe("Maven");
    expect(screen.queryByTestId("project-path")).toBeNull();
    expect(header().textContent).not.toContain(ROOT);
  });

  it("names a folder opened with forward slashes too, and one with a trailing separator", () => {
    setup("D:/work/storefront/");
    expect(screen.getByTestId("project-name").textContent).toBe("storefront");
  });

  const parts = () => [...screen.getByTestId("project-name").children].map((c) => c.textContent);

  it("cuts a long name in the middle: its start gives way first, its last 12 characters stay (R1.2)", () => {
    setup("D:\\work\\payments-gateway-contract-tests");
    expect(NAME_TAIL).toBe(12);
    expect(parts()).toEqual(["payments-gateway-co", "ntract-tests"]);
    const name = screen.getByTestId("project-name");
    expect(name.textContent).toBe("payments-gateway-contract-tests");
    expect(name.className).toContain("flex");
    const [head, tail] = name.children;
    expect(head!.className).toContain("shrink-100000");
    for (const el of [head!, tail!])
      for (const c of ["min-w-0", "truncate"]) expect(el.className).toContain(c);
  });

  it("keeps a name of 12 characters or fewer in its end part", () => {
    setup();
    expect(parts()).toEqual(["", "petstore-api"]);
  });

  it("explains a cut name in a tooltip, and says nothing while it fits (R1.3)", () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    setup("D:\\work\\payments-gateway-contract-tests");
    const name = screen.getByTestId("project-name");
    hover(name);
    expect(document.querySelector('[role="tooltip"]')).toBeNull();
    fireEvent.pointerLeave(wrapperOf(name));
    flush();
    // jsdom has no layout: make the start look cut
    Object.defineProperty(name.firstElementChild!, "scrollWidth", { value: 300 });
    Object.defineProperty(name.firstElementChild!, "clientWidth", { value: 100 });
    hover(name);
    expect(tip().textContent).toBe("payments-gateway-contract-tests");
  });

  it("lets the chip go, whole, before the name is cut (R1.2)", () => {
    setup();
    const chip = screen.getByTestId("project-layout");
    // one 20 px line that wraps and hides the second: the chip that no longer fits goes there
    const row = chip.parentElement!;
    for (const c of ["flex", "flex-wrap", "h-5", "overflow-hidden", "min-w-0", "flex-1"])
      expect(row.className, c).toContain(c);
    expect(wrapperOf(screen.getByTestId("project-name")).parentElement).toBe(row);
    // the chip is never cut; the name can shrink
    expect(chip.className).toContain("shrink-0");
    expect(chip.className).not.toContain("truncate");
    expect(wrapperOf(screen.getByTestId("project-name")).className).toContain("min-w-0");
  });

  it("has no title attribute left", () => {
    setup();
    expect(header().querySelector("[title]")).toBeNull();
  });
});

describe("the actions", () => {
  it("Run all is the only primary button, with its play icon in the on-primary ink", () => {
    setup();
    const run = button("run-all-button");
    for (const c of primary.split(" ").filter((c) => c.startsWith("bg-primary")))
      expect(run.className).toContain(c);
    const filled = [...header().querySelectorAll("button")].filter((b) =>
      b.className.includes("bg-primary"),
    );
    expect(filled).toEqual([run]);
    expect(run.querySelector("svg")!.getAttribute("class")).toContain("text-on-primary");
    expect(run.disabled).toBe(false);
  });

  it("shows the newest run's failed count in a pill on Re-run failed", () => {
    const { api, send } = setup();
    expect(button("rerun-failed-button").disabled).toBe(true);
    send({ type: "history", runs: [failedRun(2)] });
    const rerun = button("rerun-failed-button");
    expect(rerun.textContent).toBe("Re-run failed 2");
    const pill = [...rerun.querySelectorAll("span")].at(-1)!; // after the spoken label
    expect(pill.textContent).toBe("2");
    for (const c of countPill.split(" ")) expect(pill.className).toContain(c);
    expect(rerun.disabled).toBe(false);
    fireEvent.click(rerun);
    expect(api.rerunFailed).toHaveBeenCalledExactlyOnceWith("r1");
  });

  it("shows no pill while there is nothing to re-run: a 0 on a disabled button says nothing", () => {
    const { send } = setup();
    for (const runs of [[], [failedRun(0)]]) {
      send({ type: "history", runs });
      const rerun = button("rerun-failed-button");
      expect(rerun.disabled).toBe(true);
      expect(rerun.textContent).toBe("Re-run failed");
      expect(rerun.querySelector(`.${CSS.escape(countPill.split(" ")[0]!)}`)).toBeNull();
      expect(screen.getByRole("button", { name: "Re-run failed" })).toBe(rerun);
    }
  });

  it("keeps Run all, Cancel and Open as before", () => {
    const { api, send } = setup();
    fireEvent.click(button("run-all-button"));
    expect(api.run).toHaveBeenCalledWith("d:");
    fireEvent.click(button("open-button"));
    expect(api.openProject).toHaveBeenCalledTimes(1);
    expect(button("cancel-button").disabled).toBe(true);
    send({ type: "busy", busy: true });
    expect(button("cancel-button").disabled).toBe(false);
    expect(button("run-all-button").disabled).toBe(true);
    expect(button("open-button").disabled).toBe(true);
    fireEvent.click(button("cancel-button"));
    expect(api.cancel).toHaveBeenCalledTimes(1);
  });

  it("draws an icon on the text buttons, decoratively", () => {
    setup();
    for (const id of [
      "properties-button",
      "open-button",
      "report-button",
      "rerun-failed-button",
      "cancel-button",
      "run-all-button",
    ])
      expect(button(id).querySelector("svg")!.getAttribute("aria-hidden"), id).toBe("true");
  });
});

describe("the boxes", () => {
  it("Env and Threads use the shared input style", () => {
    setup();
    for (const id of ["env-input", "threads-input"]) {
      const cls = screen.getByTestId(id).className;
      for (const c of ["h-7", "border-control", "bg-surface", "rounded-md"])
        expect(cls, id).toContain(c);
      expect(cls).not.toMatch(/zinc|dark:/);
    }
  });
});

describe("toolbar: step debugger", () => {
  const started = (debug: boolean): AppEvent => ({
    type: "runStarted",
    selection: "s:a#1",
    runId: "20261102-120000-000",
    keys: ["s:a#1"],
    total: 1,
    startedAt: Date.now(),
    env: null,
    debug,
  });
  const paused: AppEvent = {
    type: "debugPaused",
    paused: {
      key: "s:a#1",
      file: "a.feature",
      line: 2,
      depth: 0,
      prefix: "*",
      keyword: "def",
      text: "x = 1",
      vars: [],
    },
  };
  const ids = () =>
    [...header().querySelectorAll("[data-testid]")].map((e) => e.getAttribute("data-testid"));

  it("a normal run keeps Run all; a debug run shows Pause until it pauses", () => {
    const { api, send } = setup();
    send(started(false));
    expect(ids()).toContain("run-all-button");
    expect(ids()).not.toContain("debug-pause");
    send(started(true));
    expect(ids()).not.toContain("run-all-button");
    expect(ids()).toContain("cancel-button");
    fireEvent.click(button("debug-pause"));
    expect(api.debugCommand).toHaveBeenCalledWith("pause");
  });

  it("paused: Continue, Step over and Step into, by click and by F5, F10 and F11", () => {
    const { api, send } = setup();
    send(started(true), paused);
    expect(ids().filter((id) => id?.startsWith("debug-"))).toEqual([
      "debug-continue",
      "debug-over",
      "debug-into",
    ]);
    fireEvent.click(button("debug-over"));
    expect(api.debugCommand).toHaveBeenLastCalledWith("over");
    for (const [key, command] of [
      ["F5", "continue"],
      ["F10", "over"],
      ["F11", "into"],
    ]) {
      fireEvent.keyDown(window, { key });
      expect(api.debugCommand).toHaveBeenLastCalledWith(command);
    }
    send({ type: "debugResumed" });
    api.debugCommand.mockClear();
    fireEvent.keyDown(window, { key: "F5" });
    expect(api.debugCommand).not.toHaveBeenCalled();
    expect(ids()).toContain("debug-pause");
  });
});

describe("the Watch toggle (watch mode R1)", () => {
  const watch = () => button("watch-button");

  it("is an icon button named Watch, off until main says on, with its own icon and the selected fill when on", () => {
    const { send } = setup();
    expect(watch()).toBe(screen.getByRole("button", { name: "Watch" }));
    expect(watch().className).toContain("size-7");
    expect(watch().textContent).toBe("");
    expect(watch().getAttribute("aria-pressed")).toBe("false");
    expect(watch().className).not.toContain("bg-selected");
    const off = watch().querySelector("svg")!.innerHTML;
    send({ type: "watch", on: true });
    expect(watch().getAttribute("aria-pressed")).toBe("true");
    expect(watch().className).toContain("bg-selected");
    expect(watch().querySelector("svg")!.innerHTML).not.toBe(off);
  });

  it("asks main for the other state on a click", () => {
    const { api, send } = setup();
    fireEvent.click(watch());
    expect(api.setWatch).toHaveBeenLastCalledWith(true);
    send({ type: "watch", on: true });
    fireEvent.click(watch());
    expect(api.setWatch).toHaveBeenLastCalledWith(false);
  });

  it("says what it does in its tooltip", () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    setup();
    hover(watch());
    expect(tip().textContent).toBe("Watch: run a feature when it is saved");
  });

  it("stays enabled during a run, and is disabled during an open and with no project", () => {
    const { send } = setup();
    send({ type: "busy", busy: true });
    expect(watch().disabled).toBe(false);
    send({ type: "opening", root: "D:\\other" });
    expect(watch().disabled).toBe(true);
    send(
      { type: "opening", root: null },
      { type: "project", project: null, tree: null, error: null },
    );
    expect(watch().disabled).toBe(true);
  });
});
