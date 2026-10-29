import { cleanup, fireEvent, render, screen } from "@solidjs/testing-library";
import { createStore, flush } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AppEvent } from "../shared/events";
import type { Api } from "../shared/ipc";
import { btn, btnIcon, ghost, NARROW_QUERY, narrowIcon, narrowLabel, primary } from "./buttons";
import { initialState, reduce } from "./reduce";
import { Toolbar } from "./Toolbar";
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
      for (const f of [...listeners]) f();
      flush();
    },
  };
}

const ROOT = "D:\\work\\petstore-api";

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
      "project-path",
      "env-input",
      "threads-input",
      "properties-button",
      "refresh-button",
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

  it("below 1100 px hides the label of five buttons from sight but not from their names", () => {
    setup();
    for (const [id, label] of LABELLED) {
      const spoken = button(id).querySelector(`.${CSS.escape(narrowLabel)}`)!;
      expect(spoken.textContent, id).toBe(label);
      expect(spoken.tagName).toBe("SPAN");
      expect(narrowLabel).toBe("max-[1100px]:sr-only");
    }
    // the names: a label and, for Re-run failed, its count
    for (const name of ["Properties…", "Open…", "Report", "Cancel", "Re-run failed 0"])
      expect(screen.getByRole("button", { name }), name).toBeTruthy();
    expect(screen.getByRole("button", { name: "Home" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Refresh" })).toBeTruthy();
  });

  it("shrinks four of them to a 28 px icon and keeps the count pill of Re-run failed", () => {
    setup();
    for (const id of ["properties-button", "open-button", "report-button", "cancel-button"])
      for (const c of narrowIcon.split(" ")) expect(button(id).className, id).toContain(c);
    expect(button("rerun-failed-button").className).toContain("max-[1100px]:px-2");
    const spans = [...button("rerun-failed-button").querySelectorAll("span")];
    expect(spans.at(-1)!.textContent).toBe("0"); // the pill follows the spoken label
  });

  it("keeps Run all's label in sight", () => {
    setup();
    const run = button("run-all-button");
    expect(run.textContent).toBe("Run all");
    expect(run.querySelector("span")).toBeNull();
    expect(run.className).not.toContain("max-[1100px]");
  });

  it("asks for the width the classes use", () => {
    expect(NARROW_QUERY).toBe("(width < 1100px)"); // Tailwind's max-[1100px] is below 1100 px
    for (const c of [narrowIcon, narrowLabel]) expect(c).toContain("max-[1100px]:");
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
    // the toolbar and the Properties button each listen
    expect(m.listeners.size).toBe(2);
    cleanup();
    expect(m.listeners.size).toBe(0);
    expect(m.query.removeEventListener).toHaveBeenCalledTimes(2);
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
  it("shows the folder name, the layout and the full path", () => {
    setup();
    expect(screen.getByTestId("project-name").textContent).toBe("petstore-api");
    expect(screen.getByTestId("project-name").className).toContain("font-medium");
    expect(screen.getByTestId("project-layout").textContent).toBe("Maven");
    const path = screen.getByTestId("project-path");
    expect(path.textContent).toBe(ROOT);
    expect(path.className).toContain("font-mono");
    expect(path.className).toContain("text-muted");
  });

  it("names a folder opened with forward slashes too, and one with a trailing separator", () => {
    setup("D:/work/storefront/");
    expect(screen.getByTestId("project-name").textContent).toBe("storefront");
  });

  it("truncates the path from its start but keeps its characters in order", () => {
    setup("D:\\" + "folder\\".repeat(30));
    const path = screen.getByTestId("project-path");
    expect(path.getAttribute("style")).toMatch(/direction:\s*rtl/);
    expect(path.getAttribute("style")).toMatch(/text-align:\s*left/);
    expect(path.className).toContain("truncate");
    const inner = path.querySelector("span")!;
    expect(inner.getAttribute("dir")).toBe("ltr");
    expect(inner.textContent).toBe("D:\\" + "folder\\".repeat(30)); // ends with a backslash, unreordered
    expect(path.childElementCount).toBe(1);
  });

  it("shows the full path in a tooltip", () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    setup();
    hover(screen.getByTestId("project-path"));
    expect(tip().textContent).toBe(ROOT);
  });

  it("keeps the layout chip in sight above 1100 px: it truncates last, and only below 1100 px may it hide", () => {
    setup();
    const chip = screen.getByTestId("project-layout");
    expect(chip.className).toContain("max-[1100px]:@max-[160px]:hidden");
    expect(chip.className).not.toMatch(/(^| )@max-\[160px\]:hidden/); // never by the container alone
    expect(chip.parentElement!.className).toContain("@container");
    // squeezed, the path gives way first, then the name, then the chip
    const shrink = (el: HTMLElement) =>
      Number(/shrink-\[?([\d.]+)\]?/.exec(el.className)?.[1] ?? 1);
    const path = wrapperOf(screen.getByTestId("project-path"));
    const name = wrapperOf(screen.getByTestId("project-name"));
    expect(shrink(path)).toBeGreaterThan(shrink(name));
    expect(shrink(name)).toBeGreaterThan(shrink(chip));
    for (const c of ["min-w-0", "truncate"]) expect(chip.className).toContain(c);
  });

  it("the path's tooltip lets the row shrink", () => {
    setup();
    expect(wrapperOf(screen.getByTestId("project-path")).className).toContain("min-w-0");
  });

  it("has no title attribute left", () => {
    setup();
    expect(header().querySelector("[title]")).toBeNull();
  });
});

describe("the actions", () => {
  it("Run all is the only primary button, with its play icon in the play colour", () => {
    setup();
    const run = button("run-all-button");
    for (const c of primary.split(" ").filter((c) => c.startsWith("bg-accent")))
      expect(run.className).toContain(c);
    const filled = [...header().querySelectorAll("button")].filter((b) =>
      b.className.includes("bg-accent"),
    );
    expect(filled).toEqual([run]);
    expect(run.querySelector("svg")!.getAttribute("class")).toContain("text-play");
    expect(run.disabled).toBe(false);
  });

  it("shows the newest run's failed count in a pill on Re-run failed", () => {
    const { api, send } = setup();
    expect(button("rerun-failed-button").disabled).toBe(true);
    send({
      type: "history",
      runs: [
        {
          id: "r1",
          startedAt: 1,
          durationMillis: 2,
          env: null,
          threads: 1,
          filter: { include: [], exclude: [], hideNonMatching: false },
          selection: null,
          failedOf: null,
          outcome: "finished",
          exitCode: 1,
          passed: 3,
          failed: 2,
          total: 5,
          reportAvailable: true,
        },
      ],
    });
    const rerun = button("rerun-failed-button");
    expect(rerun.textContent).toBe("Re-run failed 2");
    const pill = [...rerun.querySelectorAll("span")].at(-1)!; // after the spoken label
    expect(pill.textContent).toBe("2");
    for (const c of countPill.split(" ")) expect(pill.className).toContain(c);
    expect(rerun.disabled).toBe(false);
    fireEvent.click(rerun);
    expect(api.rerunFailed).toHaveBeenCalledExactlyOnceWith("r1");
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
