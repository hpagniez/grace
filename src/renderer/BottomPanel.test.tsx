import { cleanup, render, screen, within } from "@solidjs/testing-library";
import { Globe, History, SquareTerminal, type IconNode } from "lucide";
import { afterEach, describe, expect, it } from "vitest";
import type { Api } from "../shared/ipc";
import { BottomPanel } from "./BottomPanel";
import { initialState, type AppState } from "./reduce";

afterEach(cleanup);

/** Whether the svg holds this icon: every shape of the Lucide icon is in it. */
const drawn = (svg: Element | null | undefined, icon: IconNode) =>
  !!svg &&
  icon.every(([, attrs]) => Object.values(attrs).every((v) => svg.innerHTML.includes(String(v))));

function show(panel: AppState["panel"], runs = 0) {
  const state: AppState = {
    ...initialState(),
    panel,
    history: Array.from({ length: runs }, () => ({})) as unknown as AppState["history"],
  };
  render(() => (
    <BottomPanel
      state={state}
      api={{} as Api}
      showPanel={() => {}}
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
