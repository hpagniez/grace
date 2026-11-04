import { cleanup, fireEvent, render, screen, within } from "@solidjs/testing-library";
import { Globe, History, SquareTerminal } from "lucide";
import { flush } from "solid-js";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api } from "../shared/ipc";
import { BottomPanel } from "./BottomPanel";
import { TABS_KEY } from "./panelTabs";
import { initialState, type AppState } from "./reduce";
import { drawn } from "../../test/drawn";

const showPanel = vi.fn();
beforeEach(() => {
  localStorage.clear();
  showPanel.mockClear();
});
afterEach(cleanup);

function show(panel: AppState["panel"], runs = 0, run: Partial<AppState["run"]> = {}) {
  const state: AppState = {
    ...initialState(),
    panel,
    run: { ...initialState().run, ...run },
    history: Array.from({ length: runs }, () => ({})) as unknown as AppState["history"],
  };
  render(() => (
    <BottomPanel
      state={state}
      api={{} as Api}
      showPanel={showPanel}
      selectCall={() => {}}
      toggleGroup={() => {}}
      height={192}
      setHeight={() => {}}
      main={() => undefined}
      setDragging={() => {}}
    />
  ));
}
const tabs = () => ({
  console: screen.getByTestId("tab-console"),
  history: screen.getByTestId("tab-history"),
  http: screen.getByTestId("tab-http"),
});

describe("bottom panel tabs (R18.2)", () => {
  it("shows each tab's icon, its label and, for History and HTTP, the count in a pill", () => {
    show("console", 2);
    const { console, history, http } = tabs();
    expect(drawn(console.querySelector("svg"), SquareTerminal)).toBe(true);
    expect(drawn(history.querySelector("svg"), History)).toBe(true);
    expect(drawn(http.querySelector("svg"), Globe)).toBe(true);
    expect(console.querySelector("svg")!.getAttribute("width")).toBe("14");
    expect(console.querySelector("span")).toBeNull();
    // the pill shows the bare figure; the brackets are spoken, so the name stays "History (2)"
    const pill = history.querySelector("span")!;
    expect(pill.className).toContain("rounded-full");
    expect(pill.className).toContain("relative"); // or its spoken brackets escape the sheet's clip
    expect(pill.textContent).toBe(" (2)");
    expect(within(pill).getByText("(", { exact: false }).className).toContain("sr-only");
    expect(history.textContent).toBe("History (2)");
    expect(http.textContent).toBe("HTTP (0)");
    expect(console.textContent).toBe("Console");
  });

  it("marks the shown tab with the accent line and the others muted", () => {
    show("history");
    const { console, history, http } = tabs();
    expect(history.getAttribute("aria-selected")).toBe("true");
    expect(history.className).toContain("border-accent");
    expect(history.className).toContain("font-medium");
    expect(history.className).toContain("text-ink");
    for (const other of [console, http]) {
      expect(other.getAttribute("aria-selected")).toBe("false");
      expect(other.className).toContain("border-transparent");
      expect(other.className).toContain("text-muted");
    }
  });
});

describe("splitter (R18.1)", () => {
  it("is a 1 px line with a 6 px hit area that turns into a 2 px accent line on hover and drag", () => {
    show("console");
    const cls = screen.getByTestId("splitter").className;
    expect(cls).toContain("h-px");
    expect(cls).toContain("bg-line");
    expect(cls).toContain("before:h-1.5"); // the hit area
    expect(cls).toContain("after:h-0.5");
    expect(cls).toContain("after:bg-accent");
    expect(cls).toContain("hover:after:opacity-100");
    expect(cls).toContain("active:after:opacity-100");
  });
});

describe("the Variables tab (step debugger R7.4)", () => {
  it("shows only while a debug run is going", () => {
    show("console", 0, { active: true, debug: false });
    expect(screen.queryByTestId("tab-variables")).toBeNull();
    cleanup();
    show("variables", 0, { active: true, debug: true });
    expect(screen.getByTestId("tab-variables").getAttribute("aria-selected")).toBe("true");
    expect(screen.getByTestId("variables")).toBeTruthy();
  });
});

describe("reordering the tabs (reorder panel tabs)", () => {
  const order = () => screen.getAllByRole("tab").map((t) => t.id.replace("tab-", ""));
  const saved = () => localStorage.getItem(TABS_KEY);
  const tab = (id: string) => screen.getByTestId(`tab-${id}`);
  const dataTransfer = () => ({ effectAllowed: "", dropEffect: "" });
  /** Fires one drag event; the flush stands for the browser running each in its own task. */
  const drag = (
    type: "dragStart" | "dragOver" | "drop" | "dragEnd" | "dragLeave",
    id: string,
    init: object = { dataTransfer: dataTransfer() },
  ) => {
    const target = type === "dragLeave" ? screen.getByRole("tablist") : tab(id);
    const notCancelled = fireEvent[type](target, init);
    flush();
    return notCancelled;
  };
  /** Starts a drag on `from` and holds it over `over`, as the browser does. */
  const dragOver = (from: string, over: string) => {
    drag("dragStart", from);
    drag("dragOver", over);
  };
  const alt = (id: string, key: "ArrowLeft" | "ArrowRight") =>
    fireEvent.keyDown(tab(id), { key, altKey: true });

  it("starts with Console, History, HTTP", () => {
    show("console");
    expect(order()).toEqual(["console", "history", "http"]);
    expect(saved()).toBeNull();
  });

  it("reads the saved order before it first paints", () => {
    localStorage.setItem(TABS_KEY, '["http","console","history"]');
    show("console");
    expect(order()).toEqual(["http", "console", "history"]);
  });

  it.each(["nope", '["history","console"]', '["history","history","http"]', '["a","b","c"]'])(
    "shows the default order for the unusable %s and leaves it saved",
    (raw) => {
      localStorage.setItem(TABS_KEY, raw);
      show("console");
      expect(order()).toEqual(["console", "history", "http"]);
      expect(saved()).toBe(raw);
    },
  );

  it("makes the three tabs draggable, and not Variables", () => {
    show("console", 0, { active: true, debug: true });
    expect(order()).toEqual(["console", "history", "http", "variables"]);
    for (const id of ["console", "history", "http"])
      expect(tab(id).getAttribute("draggable")).toBe("true");
    expect(tab("variables").hasAttribute("draggable")).toBe(false);
  });

  it.each([
    ["console", "http", ["history", "http", "console"]],
    ["http", "console", ["http", "console", "history"]],
    ["history", "console", ["history", "console", "http"]],
  ])("drops %s on %s: the order becomes %j and is saved", (from, onto, want) => {
    show("console");
    dragOver(from, onto);
    drag("drop", onto);
    expect(order()).toEqual(want);
    expect(saved()).toBe(JSON.stringify(want));
  });

  it("marks the side where the dragged tab will land, and nothing over itself", () => {
    show("console");
    dragOver("console", "history");
    expect(tab("history").getAttribute("data-drop")).toBe("right");
    expect(tab("history").className).toContain("after:bg-accent");
    expect(tab("console").hasAttribute("data-drop")).toBe(false);
    drag("dragOver", "console");
    expect(document.querySelector("[data-drop]")).toBeNull();
    drag("dragEnd", "console", {});
    cleanup();

    show("console");
    dragOver("http", "history");
    expect(tab("history").getAttribute("data-drop")).toBe("left");
    expect(tab("history").className).toContain("after:bg-accent");
  });

  it("allows the drop only over the three tabs", () => {
    show("console", 0, { active: true, debug: true });
    drag("dragStart", "console");
    expect(drag("dragOver", "http")).toBe(false); // cancelled: a drop target
    expect(drag("dragOver", "variables")).toBe(true);
    drag("drop", "variables");
    expect(order()).toEqual(["console", "history", "http", "variables"]);
  });

  it("changes and saves nothing when a drag ends on itself, outside or with Escape", () => {
    show("console");
    dragOver("console", "history");
    drag("dragLeave", "", { relatedTarget: document.body });
    expect(document.querySelector("[data-drop]")).toBeNull();
    drag("dragEnd", "console", {}); // outside the list, or Escape: no drop, only the end
    drag("dragStart", "console");
    drag("drop", "console");
    expect(order()).toEqual(["console", "history", "http"]);
    expect(saved()).toBeNull();
    expect(document.querySelector("[data-drop]")).toBeNull();
  });

  it("keeps the shown tab while it moves tabs", () => {
    show("console");
    dragOver("http", "console");
    drag("drop", "console");
    alt("history", "ArrowLeft");
    expect(showPanel).not.toHaveBeenCalled();
    expect(tab("console").getAttribute("aria-selected")).toBe("true");
  });

  it("ignores what is dragged in from outside", () => {
    show("console");
    expect(drag("dragOver", "history")).toBe(true); // not cancelled
    expect(document.querySelector("[data-drop]")).toBeNull();
    drag("drop", "history");
    expect(order()).toEqual(["console", "history", "http"]);
    expect(saved()).toBeNull();
  });

  it("moves a focused tab one place with Alt+Right and Alt+Left, keeps its focus, and saves", () => {
    show("console");
    tab("console").focus();
    alt("console", "ArrowRight");
    expect(order()).toEqual(["history", "console", "http"]);
    expect(document.activeElement).toBe(tab("console"));
    expect(saved()).toBe('["history","console","http"]');
    alt("console", "ArrowRight");
    expect(order()).toEqual(["history", "http", "console"]);
    expect(document.activeElement).toBe(tab("console"));
    alt("console", "ArrowRight"); // the last place
    expect(order()).toEqual(["history", "http", "console"]);
    alt("console", "ArrowLeft");
    expect(order()).toEqual(["history", "console", "http"]);
    expect(document.activeElement).toBe(tab("console"));
  });

  it("stops Alt+Left at the first place without saving", () => {
    show("console");
    alt("console", "ArrowLeft");
    expect(order()).toEqual(["console", "history", "http"]);
    expect(saved()).toBeNull();
  });

  it("leaves other keys alone", () => {
    show("console");
    for (const init of [
      { key: "ArrowRight" },
      { key: "ArrowRight", altKey: true, ctrlKey: true },
      { key: "ArrowRight", altKey: true, shiftKey: true },
      { key: "ArrowDown", altKey: true },
    ])
      expect(fireEvent.keyDown(tab("console"), init)).toBe(true); // not cancelled
    expect(order()).toEqual(["console", "history", "http"]);
    expect(saved()).toBeNull();
  });

  it("does not move Variables with the keys", () => {
    show("variables", 0, { active: true, debug: true });
    alt("variables", "ArrowLeft");
    expect(order()).toEqual(["console", "history", "http", "variables"]);
    expect(saved()).toBeNull();
  });

  it("works when storage throws: the default order shows and a move is only forgotten", () => {
    const broken = () => {
      throw new Error("denied");
    };
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(broken);
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(broken);
    try {
      show("console");
      expect(order()).toEqual(["console", "history", "http"]);
      alt("console", "ArrowRight");
      expect(order()).toEqual(["history", "console", "http"]);
    } finally {
      vi.restoreAllMocks();
    }
  });
});
