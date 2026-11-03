import { cleanup, fireEvent, render, screen } from "@solidjs/testing-library";
import type { JSX } from "@solidjs/web";
import { createSignal, flush, untrack } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AppEvent } from "../shared/events";
import type { Api } from "../shared/ipc";
import { BottomPanel } from "./BottomPanel";
import { Console, createConsoleSearch } from "./Console";
import { CONSOLE_LIMIT, type AppState } from "./reduce";
import { createAppStore } from "./store";

afterEach(() => {
  cleanup();
  CSS.highlights.clear();
  document.querySelectorAll("dialog").forEach((d) => d.remove());
  vi.useRealTimers();
});

/** A Console with its own search, as BottomPanel gives it one. */
function Plain(props: { lines: string[]; children?: JSX.Element }) {
  const search = createConsoleSearch({
    lines: () => props.lines,
    dropped: () => 0,
    shown: () => true,
    visible: () => true,
    showConsole: () => {},
  });
  return (
    <Console lines={props.lines} search={search}>
      {props.children}
    </Console>
  );
}

function setup() {
  const [lines, setLines] = createSignal<string[]>([]);
  render(() => <Plain lines={lines()} />);
  const el = screen.getByTestId("console");
  // jsdom has no layout: fake a 10px-per-line scroller 100px tall
  Object.defineProperty(el, "scrollHeight", { get: () => untrack(() => lines().length * 10) });
  Object.defineProperty(el, "clientHeight", { value: 100 });
  const add = (...more: string[]) => {
    setLines((l) => [...l, ...more]);
    flush();
  };
  return { el, add };
}

describe("Console", () => {
  it("keeps following once the store holds the line cap", () => {
    let emit: (e: AppEvent) => void = () => {};
    const api = {
      ready: async () => {},
      onEvent: (cb: (e: AppEvent) => void) => ((emit = cb), () => {}),
    } as unknown as Api;
    const store = createAppStore(api);
    render(() => <Plain lines={store.state.console} />);
    const el = screen.getByTestId("console");
    // scrollHeight grows with the newest line number, not with the array length
    Object.defineProperty(el, "scrollHeight", {
      get: () => untrack(() => (Number(store.state.console.at(-1)?.slice(1) ?? -1) + 1) * 10),
    });
    Object.defineProperty(el, "clientHeight", { value: 100 });
    let n = 0;
    const append = (count: number) => {
      emit({ type: "console", lines: Array.from({ length: count }, () => `l${n++}`) });
      flush();
    };
    append(CONSOLE_LIMIT);
    append(10);
    expect(store.state.console).toHaveLength(CONSOLE_LIMIT);
    expect(el.scrollTop).toBe((CONSOLE_LIMIT + 10) * 10);
    append(5);
    expect(el.scrollTop).toBe((CONSOLE_LIMIT + 15) * 10);
  });

  it("keeps its children above the lines past the line cap", () => {
    let emit: (e: AppEvent) => void = () => {};
    const api = {
      ready: async () => {},
      onEvent: (cb: (e: AppEvent) => void) => ((emit = cb), () => {}),
    } as unknown as Api;
    const store = createAppStore(api);
    render(() => (
      <Plain lines={store.state.console}>
        <p data-testid="head">h</p>
      </Plain>
    ));
    const el = screen.getByTestId("console");
    Object.defineProperty(el, "scrollHeight", {
      get: () => untrack(() => (store.state.console.length + 1) * 10),
    });
    Object.defineProperty(el, "clientHeight", { value: 100 });
    emit({ type: "console", lines: Array.from({ length: CONSOLE_LIMIT + 10 }, (_, i) => `l${i}`) });
    flush();
    expect(store.state.console).toHaveLength(CONSOLE_LIMIT);
    expect(el.firstElementChild).toBe(screen.getByTestId("head"));
    expect(el.scrollTop).toBe((CONSOLE_LIMIT + 1) * 10);
  });

  it("styles ERROR and WARN lines", () => {
    const { add } = setup();
    add("plain", "12:00 [main] ERROR boom", "12:00 [main] WARN careful");
    expect(screen.getByText("plain").className).not.toMatch(/failed|cancelled/);
    expect(screen.getByText(/ERROR boom/).className).toContain("text-failed");
    expect(screen.getByText(/WARN careful/).className).toContain("text-cancelled");
  });

  it("follows new lines unless the user scrolled up", () => {
    const { el, add } = setup();
    add(...Array.from({ length: 30 }, (_, i) => `l${i}`));
    expect(el.scrollTop).toBe(300);
    el.scrollTop = 0; // user scrolls up
    el.dispatchEvent(new Event("scroll"));
    add("more");
    expect(el.scrollTop).toBe(0);
    el.scrollTop = 195; // within 20px of the bottom (310 - 100 = 210)
    el.dispatchEvent(new Event("scroll"));
    add("again");
    expect(el.scrollTop).toBe(320);
  });
});

/** The bottom panel on a store, wired as App wires it, with main's events sent by hand. */
function panel(shown: AppState["panel"] = "console") {
  let emit: (e: AppEvent) => void = () => {};
  const api = {
    ready: async () => {},
    onEvent: (cb: (e: AppEvent) => void) => ((emit = cb), () => {}),
  } as unknown as Api;
  const store = createAppStore(api);
  store.showPanel(shown);
  render(() => (
    <BottomPanel
      state={store.state}
      api={api}
      showPanel={store.showPanel}
      selectCall={() => {}}
      toggleGroup={() => {}}
      height={192}
      setHeight={() => {}}
      main={() => undefined}
      setDragging={() => {}}
    />
  ));
  const send = (e: AppEvent) => {
    emit(e);
    flush();
  };
  return {
    send,
    lines: (...l: string[]) => send({ type: "console", lines: l }),
    show: (p: AppState["panel"]) => {
      store.showPanel(p);
      flush();
    },
  };
}

const press = (key: string, init: KeyboardEventInit = {}) => {
  fireEvent.keyDown(document.activeElement ?? document.body, { key, ...init });
  flush();
};
const ctrlF = () => press("f", { ctrlKey: true });
const field = () => screen.getByTestId("find-field") as HTMLInputElement;
const count = () => screen.getByTestId("find-count").textContent;
const type = (text: string) => {
  fireEvent.input(field(), { target: { value: text } });
  flush();
};
/** The console's line elements, after the banner. */
const rows = () => [...screen.getByTestId("console").children].slice(1) as HTMLElement[];
const painted = (name: string) => [...(CSS.highlights.get(name) ?? [])] as Range[];
const started: AppEvent = {
  type: "runStarted",
  selection: "all",
  runId: "r2",
  keys: [],
  total: 0,
  startedAt: 2,
  env: null,
  debug: false,
};

describe("find (console search R3)", () => {
  it("Ctrl+F opens the find bar from another tab", () => {
    panel("http");
    ctrlF();
    expect(screen.getByTestId("tab-console").getAttribute("aria-selected")).toBe("true");
    expect(screen.getByTestId("find-bar")).toBeTruthy();
    expect(document.activeElement).toBe(field());
  });

  it("counts matches and steps with Enter and Shift+Enter, wrapping", () => {
    const p = panel();
    p.lines("get user 42", "create user");
    ctrlF();
    type("user");
    expect(count()).toBe("1 of 2");
    press("Enter");
    expect(count()).toBe("2 of 2");
    press("Enter");
    expect(count()).toBe("1 of 2");
    press("Enter", { shiftKey: true });
    expect(count()).toBe("2 of 2");
    fireEvent.click(screen.getByTestId("find-next"));
    flush();
    expect(count()).toBe("1 of 2");
    fireEvent.click(screen.getByTestId("find-prev"));
    flush();
    expect(count()).toBe("2 of 2");
  });

  it("shows No results, and nothing for no text, and names its parts", () => {
    const p = panel();
    p.lines("get user 42");
    ctrlF();
    expect(count()).toBe("");
    type("nothing-like-this");
    expect(count()).toBe("No results");
    expect(screen.getByTestId("find-count").getAttribute("role")).toBe("status");
    expect(field().getAttribute("aria-label")).toBe("Find in console");
    expect(field().placeholder).toBe("Find");
    for (const [id, name] of [
      ["find-prev", "Previous match"],
      ["find-next", "Next match"],
      ["find-close", "Close find"],
    ])
      expect(screen.getByTestId(id).getAttribute("aria-label"), id).toBe(name);
  });

  it("gives its buttons their keys as tooltips", () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    panel();
    ctrlF();
    for (const [id, text] of [
      ["find-prev", "Previous match (Shift+Enter)"],
      ["find-next", "Next match (Enter)"],
      ["find-close", "Close (Escape)"],
      ["find-button", "Find (Ctrl+F)"],
    ]) {
      const wrap = screen.getByTestId(id).closest("span.inline-flex") as HTMLElement;
      fireEvent.pointerEnter(wrap);
      vi.advanceTimersByTime(500);
      expect(document.querySelector('[role="tooltip"]')?.textContent, id).toBe(text);
      fireEvent.pointerLeave(wrap);
      flush();
    }
  });

  it("paints matches without redrawing the rows", () => {
    const p = panel();
    p.lines("get user 42", "create user");
    const before = rows();
    ctrlF();
    type("user");
    expect(painted("console-match").map(String)).toEqual(["user", "user"]);
    expect(painted("console-match-current")).toEqual([painted("console-match")[0]]);
    expect(rows().every((r, i) => r === before[i])).toBe(true);
  });

  it("Escape closes the bar, keeps the text and gives the focus back", () => {
    const p = panel();
    p.lines("get user 42");
    screen.getByTestId("tab-history").focus();
    ctrlF();
    type("user");
    press("Escape");
    expect(screen.queryByTestId("find-bar")).toBeNull();
    expect(CSS.highlights.size).toBe(0);
    expect(document.activeElement).toBe(screen.getByTestId("tab-history"));
    ctrlF();
    expect(field().value).toBe("user");
    expect([field().selectionStart, field().selectionEnd]).toEqual([0, 4]);
    expect(painted("console-match")).toHaveLength(1);
  });

  it("keeps the text across a tab switch and a new run", () => {
    const p = panel();
    p.lines("get user 42");
    ctrlF();
    type("user");
    p.show("history");
    p.show("console");
    expect(field().value).toBe("user");
    p.send(started);
    p.lines("user again");
    expect(count()).toBe("1 of 1");
  });

  it("keeps the current match on its line as lines arrive, without scrolling", () => {
    const p = panel();
    p.lines(...Array.from({ length: 30 }, (_, i) => (i % 10 === 5 ? `user ${i}` : `l${i}`)));
    const el = screen.getByTestId("console");
    Object.defineProperty(el, "scrollHeight", { get: () => rows().length * 10 });
    Object.defineProperty(el, "clientHeight", { value: 100 });
    ctrlF();
    type("user");
    press("Enter");
    expect(count()).toBe("2 of 3");
    el.scrollTop = 77; // the user scrolls away from the bottom
    el.dispatchEvent(new Event("scroll"));
    p.lines("user later");
    expect(count()).toBe("2 of 4");
    expect(painted("console-match-current")[0].startContainer.textContent).toBe("user 15");
    expect(el.scrollTop).toBe(77);
  });

  it("keeps the current match's line when older lines drop at the cap", () => {
    const p = panel();
    p.lines(
      ...Array.from({ length: CONSOLE_LIMIT }, (_, i) =>
        i === 150 || i === 9000 ? `user ${i}` : `l${i}`,
      ),
    );
    ctrlF();
    type("user");
    press("Enter");
    expect(painted("console-match-current")[0].startContainer.textContent).toBe("user 9000");
    p.lines(...Array.from({ length: 100 }, (_, i) => `m${i}`));
    expect(painted("console-match-current")[0].startContainer.textContent).toBe("user 9000");
    expect(count()).toBe("2 of 2");
  });

  it("does not search while the bar is closed", () => {
    const p = panel();
    ctrlF();
    type("user");
    press("Escape");
    p.lines("user more");
    expect(CSS.highlights.size).toBe(0);
  });

  it("ignores Ctrl+F while a dialog is open, and under the Settings page", () => {
    panel();
    const d = document.createElement("dialog");
    d.setAttribute("open", "");
    document.body.append(d);
    ctrlF();
    expect(screen.queryByTestId("find-bar")).toBeNull();
    d.remove();
    screen.getByTestId("bottom-panel").checkVisibility = () => false;
    ctrlF();
    expect(screen.queryByTestId("find-bar")).toBeNull();
  });

  it("opens from the Find button, which only the Console tab shows", () => {
    const p = panel();
    const button = screen.getByTestId("find-button");
    expect(button.getAttribute("aria-label")).toBe("Find");
    fireEvent.click(button);
    flush();
    expect(document.activeElement).toBe(field());
    p.show("history");
    expect(screen.queryByTestId("find-button")).toBeNull();
  });
});
