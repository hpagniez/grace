import { render, screen, cleanup, fireEvent } from "@solidjs/testing-library";
import { createSignal, flush } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";
import { scrollToShow } from "./virtual";
import { VirtualList, type VirtualListHandle } from "./VirtualList";

afterEach(cleanup);

const keys = (n: number) => Array.from({ length: n }, (_, i) => `r${i}`);
/** The view jsdom has: no height, so the 600 px fallback. */
const at = (scrollTop: number) => ({ scrollTop, height: 0, rowHeight: 24, pad: 0 });

function setup(n = 1000, selected: string | null = null) {
  const [items, setItems] = createSignal(keys(n));
  const [selectedKey, setSelectedKey] = createSignal(selected);
  const [cue, setCue] = createSignal(0);
  const onSelect = vi.fn();
  const onKey = vi.fn((_e: KeyboardEvent, _item: string, _index: number) => false);
  const press = vi.fn();
  let handle!: VirtualListHandle;
  render(() => (
    <VirtualList
      items={items()}
      keyOf={(k) => k}
      rowHeight={24}
      role="listbox"
      itemRole="option"
      label="Rows"
      testid="list"
      scrollerTestid="scroller"
      attrs={(k) => ({ "aria-selected": k === selectedKey() ? "true" : "false" })}
      selectedKey={selectedKey()}
      onSelect={onSelect}
      onKey={onKey}
      revealSelectedOn={cue()}
      ref={(h) => (handle = h)}
    >
      {(k, _i, keyboardFocused) => (
        <span data-keyboard={keyboardFocused() ? "yes" : "no"}>
          {k}
          <button type="button" onClick={press}>
            go
          </button>
        </span>
      )}
    </VirtualList>
  ));
  const scroller = screen.getByTestId("scroller");
  const list = screen.getByTestId("list");
  const rows = () => [...list.querySelectorAll<HTMLElement>("[role=option]")];
  const row = (k: string) => rows().find((r) => r.dataset.key === k);
  const stops = () => rows().filter((r) => r.getAttribute("tabindex") === "0");
  const focus = (k: string) => {
    row(k)!.focus();
    flush();
  };
  const scrollTo = (top: number) => {
    scroller.scrollTop = top;
    scroller.dispatchEvent(new Event("scroll"));
    flush();
  };
  /** Presses a key on the focused element; returns the event, to read defaultPrevented. */
  const key = (k: string) => {
    const e = new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true });
    document.activeElement!.dispatchEvent(e);
    flush();
    return e;
  };
  const change = (next: string[]) => {
    setItems(next);
    flush();
  };
  /** A change the person made, as a filter: the rows and the cue in one update. */
  const filter = (next: string[]) => {
    setItems(next);
    setCue((c) => c + 1);
    flush();
  };
  return {
    scroller,
    list,
    rows,
    row,
    stops,
    focus,
    scrollTo,
    key,
    change,
    filter,
    onSelect,
    onKey,
    press,
    setSelectedKey,
    handle: () => handle,
  };
}

describe("VirtualList", () => {
  it("draws only the rows in view, plus the margin", () => {
    const { list, rows } = setup();
    expect(rows()).toHaveLength(35);
    expect(list.dataset.rows).toBe("1000");
    expect(list.style.height).toBe("24000px");
    expect(list.parentElement).toBe(screen.getByTestId("scroller"));
  });

  it("draws the rows at the bottom after a scroll, and keeps the focus row", () => {
    const { focus, rows, row, scrollTo } = setup();
    focus("r0");
    scrollTo(23400);
    const drawn = rows().map((r) => r.dataset.key);
    expect(drawn).toEqual(expect.arrayContaining(keys(1000).slice(965)));
    expect(drawn).toHaveLength(36);
    expect(row("r0")?.getAttribute("tabindex")).toBe("0");
    expect(document.activeElement).toBe(row("r0"));
    expect(row("r999")?.style.top).toBe("23976px");
  });

  it("keeps focus on its row when the list jumps back to the top", () => {
    const { focus, row, scrollTo } = setup();
    focus("r10");
    scrollTo(2000);
    scrollTo(0);
    expect(document.activeElement).toBe(row("r10"));
  });

  it("keeps focus on its row when the list jumps onto rows below it", () => {
    const { focus, row, scrollTo } = setup();
    scrollTo(2400);
    focus("r100");
    expect(document.activeElement).toBe(row("r100"));
    scrollTo(0);
    expect(document.activeElement).toBe(row("r100"));
  });

  it("keeps focus on its row through 100 px steps both ways", () => {
    const { focus, row, scrollTo } = setup();
    scrollTo(1000);
    focus("r50");
    const steps = Array.from({ length: 21 }, (_, i) => i * 100);
    for (const top of [...steps, ...steps.toReversed()]) {
      scrollTo(top);
      expect(document.activeElement, `at ${top}`).toBe(row("r50"));
    }
  });

  it("has one tab stop: the selected row, else the first", () => {
    const { stops, setSelectedKey } = setup(1000, "r5");
    expect(stops().map((r) => r.dataset.key)).toEqual(["r5"]);
    setSelectedKey(null);
    flush();
    expect(stops().map((r) => r.dataset.key)).toEqual(["r0"]);
  });

  it("moves with Down, Up, Home, End, Page Down and Page Up, selecting as it goes", () => {
    const { focus, row, key, scroller, onSelect } = setup();
    focus("r0");
    key("ArrowDown");
    expect(document.activeElement).toBe(row("r1"));
    expect(onSelect).toHaveBeenLastCalledWith("r1");
    key("ArrowUp");
    expect(document.activeElement).toBe(row("r0"));
    key("PageDown");
    expect(document.activeElement).toBe(row("r25"));
    expect(onSelect).toHaveBeenLastCalledWith("r25");
    key("End");
    expect(document.activeElement).toBe(row("r999"));
    expect(scroller.scrollTop).toBe(scrollToShow(999, at(0)));
    expect(scroller.scrollTop).toBe(23400);
    key("PageUp");
    expect(document.activeElement).toBe(row("r974"));
    key("Home");
    expect(document.activeElement).toBe(row("r0"));
    expect(onSelect).toHaveBeenLastCalledWith("r0");
    expect(scroller.scrollTop).toBe(0);
  });

  it("does nothing at the ends, and keeps the page still", () => {
    const { focus, row, key, onSelect } = setup();
    focus("r0");
    const e = key("ArrowUp");
    expect(onSelect).not.toHaveBeenCalled();
    expect(e.defaultPrevented).toBe(true);
    expect(document.activeElement).toBe(row("r0"));
  });

  it("selects with Enter and Space", () => {
    const { focus, key, onSelect } = setup();
    focus("r2");
    expect(key("Enter").defaultPrevented).toBe(true);
    expect(onSelect).toHaveBeenLastCalledWith("r2");
    expect(key(" ").defaultPrevented).toBe(true);
    expect(onSelect).toHaveBeenCalledTimes(2);
    expect(onSelect).toHaveBeenLastCalledWith("r2");
  });

  it("passes other keys to onKey, and stops what it handled", () => {
    const { focus, key, onKey } = setup();
    focus("r0");
    const ignored = key("ArrowRight");
    expect(onKey).toHaveBeenCalledWith(ignored, "r0", 0);
    expect(ignored.defaultPrevented).toBe(false);
    onKey.mockReturnValue(true);
    expect(key("ArrowRight").defaultPrevented).toBe(true);
  });

  it("brings a scrolled-away focus row back before moving", () => {
    const { focus, row, key, scrollTo, scroller } = setup();
    focus("r10");
    scrollTo(20000);
    expect(document.activeElement).toBe(row("r10"));
    key("ArrowDown");
    expect(document.activeElement).toBe(row("r11"));
    expect(scroller.scrollTop).toBe(scrollToShow(11, at(20000)));
    expect(scroller.scrollTop).toBe(264);
  });

  it("takes focus to the row clicked, even through a button in it", () => {
    const { row, key, press, onSelect } = setup();
    fireEvent.click(row("r3")!.querySelector("button")!);
    flush();
    expect(press).toHaveBeenCalledTimes(1);
    expect(document.activeElement).toBe(row("r3"));
    key(" ");
    expect(onSelect).toHaveBeenLastCalledWith("r3");
    expect(press).toHaveBeenCalledTimes(1);
  });

  it("selects a row clicked on itself, as a screen reader's activate does, not through a click inside it", () => {
    const { row, onSelect } = setup();
    fireEvent.click(row("r3")!);
    flush();
    expect(document.activeElement).toBe(row("r3"));
    expect(onSelect).toHaveBeenCalledTimes(1);
    expect(onSelect).toHaveBeenLastCalledWith("r3");
    fireEvent.click(row("r4")!.querySelector("span")!);
    flush();
    expect(document.activeElement).toBe(row("r4"));
    expect(onSelect).toHaveBeenCalledTimes(1);
  });

  it("keeps a press on a row from taking focus, so its click arrives", () => {
    const { row, stops, press } = setup();
    const button = row("r3")!.querySelector("button")!;
    for (const target of [row("r3")!, button]) {
      const down = new MouseEvent("mousedown", { bubbles: true, cancelable: true });
      target.dispatchEvent(down);
      expect(down.defaultPrevented).toBe(true);
    }
    const focus = vi.spyOn(row("r3")!, "focus");
    fireEvent.click(button);
    flush();
    expect(document.activeElement).toBe(row("r3"));
    expect(stops()).toEqual([row("r3")]);
    expect(press).toHaveBeenCalledTimes(1);
    // jsdom cannot tell :focus-visible: the click asks for no keyboard focus ring
    expect(focus).toHaveBeenCalledWith({ preventScroll: true, focusVisible: false });
  });

  it("keeps the rows in view still when rows are added above", () => {
    const { scrollTo, change, scroller } = setup();
    scrollTo(240);
    change(["n0", "n1", "n2", "n3", "n4", ...keys(1000)]);
    expect(scroller.scrollTop).toBe(360);
  });

  it("moves focus to the nearest row above when the focus row leaves", () => {
    const { focus, row, change } = setup();
    focus("r5");
    change(keys(1000).filter((k) => !["r3", "r4", "r5"].includes(k)));
    expect(row("r2")?.getAttribute("tabindex")).toBe("0");
    expect(document.activeElement).toBe(row("r2"));
    // the nearest row above is the first row, the focus row while none is active
    change(keys(1000).filter((k) => k !== "r1" && k !== "r2"));
    expect(document.activeElement).toBe(row("r0"));
  });

  it("falls back to the selected row when an unfocused active row leaves", () => {
    const { row, stops, change } = setup(1000, "r7");
    fireEvent.click(row("r5")!);
    flush();
    expect(document.activeElement).toBe(row("r5"));
    row("r5")!.blur();
    flush();
    change(keys(1000).filter((k) => !["r3", "r4", "r5"].includes(k)));
    expect(stops().map((r) => r.dataset.key)).toEqual(["r7"]);
    expect(document.activeElement).toBe(document.body);
  });

  it("draws rows after the list shrinks below the scroll position", () => {
    const { scrollTo, change, rows } = setup();
    scrollTo(20000);
    change(keys(5));
    expect(rows().map((r) => r.dataset.key)).toEqual(keys(5));
  });

  it("reveals a row by key", () => {
    const { handle, scroller, row } = setup();
    handle().reveal("r500");
    flush();
    expect(scroller.scrollTop).toBe(scrollToShow(500, at(0)));
    expect(row("r500")).toBeDefined();
    handle().reveal("nope");
    flush();
    expect(scroller.scrollTop).toBe(scrollToShow(500, at(0)));
  });

  it("reveals the selected row after keeping the rows in view still, when revealSelectedOn changes", () => {
    const { scrollTo, change, filter, scroller } = setup(1000, "r900");
    scrollTo(2400); // r100 is the top row, r900 far below
    const rest = keys(1000).slice(50);
    filter(rest); // 50 rows leave above the view: r100 stays on top (1200), then r900 comes into view
    expect(scroller.scrollTop).toBe(scrollToShow(850, at(1200)));
    expect(scroller.scrollTop).toBe(19824);
    scrollTo(0);
    change([...rest]); // rows that change without the cue never scroll to the selection
    expect(scroller.scrollTop).toBe(0);
  });

  it("moves to a row by key from the handle, as an arrow key does", () => {
    const { handle, row, rows, stops, scroller, onSelect } = setup();
    handle().moveTo("r500");
    flush();
    expect(document.activeElement).toBe(row("r500"));
    expect(stops()).toEqual([row("r500")]);
    expect(onSelect).toHaveBeenLastCalledWith("r500");
    expect(rows().filter((r) => r.querySelector("[data-keyboard=yes]"))).toEqual([row("r500")]);
    expect(scroller.scrollTop).toBe(scrollToShow(500, at(0)));
    handle().moveTo("nope");
    flush();
    expect(onSelect).toHaveBeenCalledTimes(1);
    expect(document.activeElement).toBe(row("r500"));
  });

  it("tells a row when it has keyboard focus", () => {
    const { focus, row, rows, key } = setup();
    focus("r0");
    key("ArrowDown");
    const told = rows().filter((r) => r.querySelector("[data-keyboard=yes]"));
    expect(told).toEqual([row("r1")]);
  });

  it("keeps a drawn row's element through a scroll, and updates its attributes", () => {
    const { row, scrollTo, setSelectedKey } = setup();
    const r20 = row("r20");
    scrollTo(24);
    setSelectedKey("r20");
    flush();
    expect(row("r20")).toBe(r20);
    expect(r20?.getAttribute("aria-selected")).toBe("true");
  });
});

describe("shared", () => {
  it("is made once per drawn row for its attributes and content, not again as the focus moves", () => {
    const shared = vi.fn((item: () => string) => ({ upper: () => item().toUpperCase() }));
    const onSelect = vi.fn();
    render(() => (
      <VirtualList
        items={keys(3)}
        keyOf={(k) => k}
        rowHeight={24}
        role="listbox"
        itemRole="option"
        label="Rows"
        testid="list"
        shared={shared}
        attrs={(_k, _i, s) => ({ "aria-label": s.upper() })}
        selectedKey={null}
        onSelect={onSelect}
      >
        {(_k, _i, _keyboard, s) => <span>{s.upper()}</span>}
      </VirtualList>
    ));
    const rows = () => [
      ...screen.getByTestId("list").querySelectorAll<HTMLElement>("[role=option]"),
    ];
    expect(rows().map((r) => [r.getAttribute("aria-label"), r.textContent])).toEqual([
      ["R0", "R0"],
      ["R1", "R1"],
      ["R2", "R2"],
    ]);
    expect(shared).toHaveBeenCalledTimes(3);
    rows()[0].focus();
    flush();
    rows()[0].dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }));
    flush();
    expect(onSelect).toHaveBeenLastCalledWith("r1");
    expect(shared).toHaveBeenCalledTimes(3);
  });
});
