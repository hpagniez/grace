import { cleanup, fireEvent, render, screen } from "@solidjs/testing-library";
import { createSignal, flush } from "solid-js";
import { afterEach, beforeEach, describe, expect, it, type Mock, vi } from "vitest";
import { Tooltip } from "./Tooltip";

// Only the timers: a full fake would stop Solid's microtask flush.
beforeEach(() => vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] }));
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

/** The open tooltips: one is in the DOM only while it is open. */
const tips = () => [...document.querySelectorAll<HTMLElement>('[role="tooltip"]')];
const tipOf = () => tips()[0];
const wrapperOf = (el: HTMLElement) => el.closest("span.inline-flex") as HTMLElement;

/** Events that Solid applies at the next flush, then flushed. */
function leave(el: HTMLElement) {
  fireEvent.pointerLeave(el);
  flush();
}

/** jsdom has no focus ring logic: say what `:focus-visible` answers. */
function focusVisible(answer: boolean | "throws") {
  const real = Element.prototype.matches;
  vi.spyOn(Element.prototype, "matches").mockImplementation(function (this: Element, sel: string) {
    if (sel !== ":focus-visible") return real.call(this, sel);
    if (answer === "throws") throw new SyntaxError("not a valid selector");
    return answer;
  });
}

function one(describe = false) {
  render(() => (
    <Tooltip text="Refresh the tree" describe={describe}>
      <button type="button">Refresh</button>
    </Tooltip>
  ));
  const button = screen.getByRole("button");
  return { button, wrapper: wrapperOf(button) };
}

/** Rests the pointer on the wrapper for the whole delay. */
function hover(wrapper: HTMLElement) {
  fireEvent.pointerEnter(wrapper);
  vi.advanceTimersByTime(500);
}

describe("Tooltip", () => {
  it("is in the DOM only while open: a manual popover with the text, inside the wrapper", () => {
    const { wrapper } = one();
    expect(tips()).toHaveLength(0);
    expect(wrapper.textContent).toBe("Refresh");
    expect(screen.getAllByText("Refresh")).toHaveLength(1);

    hover(wrapper);
    const tip = tipOf()!;
    expect(tip.getAttribute("popover")).toBe("manual");
    expect(tip.textContent).toBe("Refresh the tree");
    expect(wrapper.contains(tip)).toBe(true);

    leave(wrapper);
    expect(tips()).toHaveLength(0);
  });

  it("puts `class` on the wrapper after inline-flex", () => {
    render(() => (
      <Tooltip text="A long name" class="min-w-0 flex-1">
        <span class="truncate">A long name</span>
      </Tooltip>
    ));
    const wrapper = screen.getByText("A long name").parentElement!;
    expect(wrapper.className).toBe("inline-flex min-w-0 flex-1");
  });

  it("opens nothing for an empty or missing text", () => {
    render(() => (
      <>
        <Tooltip text="">
          <button type="button">a</button>
        </Tooltip>
        <Tooltip text={undefined}>
          <button type="button">b</button>
        </Tooltip>
      </>
    ));
    focusVisible(true);
    for (const button of screen.getAllByRole("button")) {
      hover(wrapperOf(button));
      fireEvent.focusIn(button);
      flush();
    }
    expect(tips()).toHaveLength(0);
  });

  it("resets the text styles a row would pass down", () => {
    const { wrapper } = one();
    hover(wrapper);
    const classes = tipOf()!.className.split(" ");
    for (const c of [
      "whitespace-normal",
      "text-left",
      "font-normal",
      "normal-case",
      "tracking-normal",
    ])
      expect(classes).toContain(c);
  });

  it("shows 500 ms after the pointer enters, not at 499", () => {
    const { wrapper } = one();
    fireEvent.pointerEnter(wrapper);
    vi.advanceTimersByTime(499);
    expect(tips()).toHaveLength(0);
    vi.advanceTimersByTime(1);
    expect(tips()).toHaveLength(1);
  });

  it("does not show when the pointer leaves first, and hides when it leaves after", () => {
    const { wrapper } = one();
    fireEvent.pointerEnter(wrapper);
    vi.advanceTimersByTime(300);
    leave(wrapper);
    vi.advanceTimersByTime(1000);
    expect(tips()).toHaveLength(0);

    hover(wrapper);
    expect(tips()).toHaveLength(1);
    leave(wrapper);
    expect(tips()).toHaveLength(0);
  });

  it("keeps open while the pointer moves from the target onto the tooltip", () => {
    const { button, wrapper } = one();
    hover(wrapper);
    // The tooltip is a DOM child of the wrapper, so the browser fires no pointerleave on the wrapper;
    // pointerleave does not bubble, so the one on the button does not reach it either.
    leave(button);
    fireEvent.pointerEnter(tipOf()!);
    vi.advanceTimersByTime(1000);
    expect(tips()).toHaveLength(1);
    leave(wrapper);
    expect(tips()).toHaveLength(0);
  });

  it("shows at once on keyboard focus and hides on blur", () => {
    focusVisible(true);
    const { button } = one();
    fireEvent.focusIn(button);
    expect(tips()).toHaveLength(1);
    fireEvent.focusOut(button);
    flush();
    expect(tips()).toHaveLength(0);
  });

  it("ignores focus that is not :focus-visible (a mouse click)", () => {
    focusVisible(false);
    const { button } = one();
    fireEvent.focusIn(button);
    expect(tips()).toHaveLength(0);
  });

  it("shows on focus when the engine cannot tell :focus-visible", () => {
    focusVisible("throws");
    const { button } = one();
    fireEvent.focusIn(button);
    expect(tips()).toHaveLength(1);
  });

  it("shows even over a disabled button, which gets no pointer events itself", () => {
    render(() => (
      <Tooltip text="Pick a project first">
        <button type="button" disabled>
          Run
        </button>
      </Tooltip>
    ));
    hover(wrapperOf(screen.getByRole("button")));
    expect(tips()).toHaveLength(1);
  });

  it("closes the other tooltip when a second one opens", () => {
    focusVisible(true);
    render(() => (
      <>
        <Tooltip text="First">
          <button type="button">a</button>
        </Tooltip>
        <Tooltip text="Second">
          <button type="button">b</button>
        </Tooltip>
      </>
    ));
    const [a, b] = screen.getAllByRole("button");
    hover(wrapperOf(a!));
    expect(tips().map((t) => t.textContent)).toEqual(["First"]);
    fireEvent.focusIn(b!);
    flush();
    expect(tips().map((t) => t.textContent)).toEqual(["Second"]);
  });

  it("hides when a scroll moves its target: the page's, or that of a box that holds it", () => {
    const { wrapper } = one();
    hover(wrapper);
    fireEvent.scroll(document.body);
    flush();
    expect(tips()).toHaveLength(0);

    cleanup();
    render(() => (
      <div data-testid="list">
        <Tooltip text="In a list">
          <button type="button">Row</button>
        </Tooltip>
      </div>
    ));
    hover(wrapperOf(screen.getByRole("button")));
    fireEvent.scroll(screen.getByTestId("list"));
    flush();
    expect(tips()).toHaveLength(0);
  });

  it("stays open when a box that does not hold its target scrolls: a console following its tail", () => {
    render(() => (
      <>
        <div data-testid="console" />
        <Tooltip text="Refresh">
          <button type="button" disabled>
            Refresh
          </button>
        </Tooltip>
      </>
    ));
    hover(wrapperOf(screen.getByRole("button")));
    fireEvent.scroll(screen.getByTestId("console"));
    flush();
    expect(tips().map((x) => x.textContent)).toEqual(["Refresh"]);
  });

  it("hides on a press of its target", () => {
    const { button, wrapper } = one();
    hover(wrapper);
    expect(tips()).toHaveLength(1);
    fireEvent.pointerDown(button);
    flush();
    expect(tips()).toHaveLength(0);
    leave(wrapper);
  });

  it("hides on a click inside it, not on the press that starts one", () => {
    const { wrapper } = one();
    hover(wrapper);
    fireEvent.pointerDown(tipOf()!); // a press may start a text selection
    flush();
    expect(tips()).toHaveLength(1);
    fireEvent.click(tipOf()!);
    flush();
    expect(tips()).toHaveLength(0);
  });

  it("does not come back under a pointer that has not moved since the press", () => {
    const at = (x: number) => ({ clientX: x, clientY: 20 });
    const { button, wrapper } = one();
    fireEvent.pointerEnter(wrapper, at(10));
    fireEvent.pointerDown(button, at(10));
    flush();
    vi.advanceTimersByTime(500); // a press ends the pause that was running
    expect(tips()).toHaveLength(0);

    // the row is redrawn under the pointer: its new wrapper gets a pointerenter where the press was
    cleanup();
    const again = one();
    fireEvent.pointerEnter(again.wrapper, at(11));
    vi.advanceTimersByTime(500);
    expect(tips()).toHaveLength(0);

    // the pointer moves on, and comes back somewhere else: a tooltip again
    fireEvent.pointerEnter(again.wrapper, at(40));
    vi.advanceTimersByTime(500);
    expect(tips()).toHaveLength(1);
    leave(again.wrapper);

    // leaving ends it: the same spot shows a tooltip after the pointer was elsewhere
    fireEvent.pointerDown(again.button, at(10));
    leave(again.wrapper);
    fireEvent.pointerEnter(again.wrapper, at(10));
    vi.advanceTimersByTime(500);
    expect(tips()).toHaveLength(1);
  });

  it("calls showPopover once the tooltip exists, when the engine has it", () => {
    const show = vi.fn(function (this: HTMLElement) {
      expect(this.isConnected).toBe(true);
    });
    HTMLElement.prototype.showPopover = show;
    try {
      const { wrapper } = one();
      hover(wrapper);
      expect(show).toHaveBeenCalledTimes(1);
    } finally {
      // @ts-expect-error jsdom has none; put it back as it was
      delete HTMLElement.prototype.showPopover;
    }
  });

  describe("pinned", () => {
    const TEXT = "Refresh the tree";

    /** A tooltip that its owner opens: a row that has the keyboard focus and a cut label. */
    function pin(initial: boolean, text: string | undefined) {
      const [pinned, setPinned] = createSignal(initial);
      const [label, setLabel] = createSignal(text);
      render(() => (
        <div data-testid="list">
          <Tooltip text={label()} pinned={pinned()}>
            <button type="button">Refresh</button>
          </Tooltip>
        </div>
      ));
      flush();
      return {
        wrapper: wrapperOf(screen.getByRole("button")),
        set: (on: boolean) => {
          setPinned(on);
          flush();
        },
        setText: (t: string | undefined) => {
          setLabel(t);
          flush();
        },
      };
    }

    it("shows at once while true, with the text, and hides when it turns false", () => {
      const { set } = pin(false, TEXT);
      expect(tips()).toHaveLength(0);
      set(true);
      expect(tips().map((t) => t.textContent)).toEqual([TEXT]);
      set(false);
      expect(tips()).toHaveLength(0);
    });

    it("shows from the start when it is true on mount", () => {
      pin(true, TEXT);
      expect(tips().map((t) => t.textContent)).toEqual([TEXT]);
    });

    it("shows nothing without a text, and shows once the text comes", () => {
      const { set, setText } = pin(false, undefined);
      set(true);
      expect(tips()).toHaveLength(0);
      setText(TEXT);
      expect(tips()).toHaveLength(1);
      setText(undefined);
      expect(tips()).toHaveLength(0);
    });

    it("leaves hover as it was: a pointer resting on the target opens it, leaving closes it", () => {
      const { wrapper } = pin(false, TEXT);
      hover(wrapper);
      expect(tips()).toHaveLength(1);
      leave(wrapper);
      expect(tips()).toHaveLength(0);
    });

    it("hides at the cleanup of its owner", () => {
      const { set } = pin(false, TEXT);
      set(true);
      cleanup();
      expect(tips()).toHaveLength(0);
    });

    describe("placed", () => {
      let target: DOMRect;
      let box: DOMRect | undefined; // the scrolling box's own, when a test gives it one
      let show: Mock<(this: HTMLElement) => void>;
      beforeEach(() => {
        target = DOMRect.fromRect({ x: 500, y: 300, width: 24, height: 24 });
        box = undefined;
        const tip = DOMRect.fromRect({ x: 0, y: 0, width: 24, height: 24 });
        vi.spyOn(Element.prototype, "getBoundingClientRect").mockImplementation(function (
          this: Element,
        ) {
          if (box && this.getAttribute("data-testid") === "list") return box;
          return this.getAttribute("role") === "tooltip" ? tip : target;
        });
        show = vi.fn(function (this: HTMLElement) {
          expect(this.isConnected).toBe(true);
        });
        HTMLElement.prototype.showPopover = show;
      });
      afterEach(() => {
        // @ts-expect-error jsdom has none; put it back as it was
        delete HTMLElement.prototype.showPopover;
      });

      it("shows the popover once and places it below the target when pinned turns true", () => {
        const { set } = pin(false, TEXT);
        set(true);
        expect(show).toHaveBeenCalledTimes(1);
        expect(tipOf()!.dataset.side).toBe("below");
        expect(tipOf()!.style.top).toBe("330px");
        expect(tipOf()!.style.left).toBe("500px");
      });

      it("does the same when pinned is true on mount", () => {
        pin(true, TEXT);
        expect(show).toHaveBeenCalledTimes(1);
        expect(tipOf()!.dataset.side).toBe("below");
        expect(tipOf()!.style.top).toBe("330px");
      });

      it("stays up and is placed again when a box that holds it scrolls, while pinned", () => {
        const { set } = pin(true, TEXT);
        target = DOMRect.fromRect({ x: 500, y: 400, width: 24, height: 24 }); // the scroll moved it
        fireEvent.scroll(screen.getByTestId("list"));
        flush();
        expect(tips()).toHaveLength(1);
        expect(tipOf()!.style.top).toBe("430px");
        expect(show).toHaveBeenCalledTimes(1); // placed again, not shown again

        set(false); // the scroll listener did not outlive the close
        expect(tips()).toHaveLength(0);
      });

      it("hides, though pinned, once that scroll takes its target out of the box", () => {
        box = DOMRect.fromRect({ x: 0, y: 0, width: 1000, height: 400 });
        pin(true, TEXT);
        target = DOMRect.fromRect({ x: 500, y: 350, width: 24, height: 24 }); // still in the box
        fireEvent.scroll(screen.getByTestId("list"));
        flush();
        expect(tipOf()!.style.top).toBe("380px");
        target = DOMRect.fromRect({ x: 500, y: 400, width: 24, height: 24 }); // just below it
        fireEvent.scroll(screen.getByTestId("list"));
        flush();
        expect(tips()).toHaveLength(0);
      });

      it("still hides on that scroll when it is not pinned", () => {
        const { wrapper } = pin(false, TEXT);
        hover(wrapper);
        expect(tips()).toHaveLength(1);
        fireEvent.scroll(screen.getByTestId("list"));
        flush();
        expect(tips()).toHaveLength(0);
      });

      it("hides on Escape and on a pointer leave even while pinned", () => {
        const { wrapper } = pin(true, TEXT);
        fireEvent.keyDown(wrapper, { key: "Escape" });
        flush();
        expect(tips()).toHaveLength(0);

        cleanup();
        const again = pin(true, TEXT);
        leave(again.wrapper);
        expect(tips()).toHaveLength(0);
      });
    });
  });

  describe("clicks on the tooltip", () => {
    it("do not reach a handler on an ancestor", () => {
      const delegated = vi.fn(); // Solid's onClick, run at the document
      const native = vi.fn();
      render(() => (
        <div data-testid="row" onClick={delegated} ref={(e) => e.addEventListener("click", native)}>
          <Tooltip text="Hint">
            <button type="button">x</button>
          </Tooltip>
        </div>
      ));
      hover(wrapperOf(screen.getByRole("button")));
      const tip = tipOf()!;
      for (const type of ["dblclick", "contextmenu", "click"])
        tip.dispatchEvent(new MouseEvent(type, { bubbles: true, cancelable: true }));
      expect(delegated).not.toHaveBeenCalled();
      expect(native).not.toHaveBeenCalled();
      flush();
      expect(tips()).toHaveLength(0); // the click closed it

      fireEvent.click(screen.getByRole("button")); // the target itself still reaches the row
      expect(delegated).toHaveBeenCalledTimes(1);
    });

    /** A tooltip with `side` whose target is a 200 x 24 label at (100, 300); a click on the tooltip at x. */
    function clickAt(side: "over" | "left", x: number) {
      const rect = (e: Element) =>
        e.getAttribute("role") === "tooltip"
          ? DOMRect.fromRect({ x: 100, y: 300, width: 320, height: 26 })
          : DOMRect.fromRect({ x: 100, y: 300, width: 200, height: 24 });
      vi.spyOn(Element.prototype, "getBoundingClientRect").mockImplementation(function (
        this: Element,
      ) {
        return rect(this);
      });
      const onTarget = vi.fn();
      render(() => (
        <div onClick={onTarget}>
          <Tooltip text="A long label, whole" side={side}>
            <button type="button">A long la…</button>
          </Tooltip>
        </div>
      ));
      hover(wrapperOf(screen.getByRole("button")));
      fireEvent.click(tipOf()!, { clientX: x, clientY: 310 });
      flush();
      expect(tips()).toHaveLength(0);
      return onTarget;
    }

    it("on an `over` tooltip, inside its target's box, reach the target: the tooltip lies on it", () => {
      expect(clickAt("over", 150)).toHaveBeenCalledTimes(1);
    });

    it("on an `over` tooltip, beyond its target's box, only close it", () => {
      expect(clickAt("over", 350)).not.toHaveBeenCalled();
    });

    it("on a tooltip beside its target only close it: what lies under is not the target", () => {
      expect(clickAt("left", 150)).not.toHaveBeenCalled();
    });
  });

  describe("side", () => {
    // every box 24 x 24 in the middle of the window, so no side runs out of room
    beforeEach(() => {
      vi.spyOn(Element.prototype, "getBoundingClientRect").mockReturnValue(
        DOMRect.fromRect({ x: 500, y: 300, width: 24, height: 24 }),
      );
    });

    const at = (side?: "below" | "left" | "right" | "over") => {
      render(() => (
        <Tooltip text="Hint" side={side}>
          <button type="button">x</button>
        </Tooltip>
      ));
      hover(wrapperOf(screen.getByRole("button")));
      const tip = tipOf()!;
      return { side: tip.dataset.side, top: tip.style.top, left: tip.style.left };
    };

    it("is below by default", () => {
      expect(at()).toEqual({ side: "below", top: "330px", left: "500px" });
    });

    it("goes where it is asked and says where it went", () => {
      expect(at("left")).toEqual({ side: "left", top: "300px", left: "470px" });
      cleanup();
      expect(at("right")).toEqual({ side: "right", top: "300px", left: "530px" });
      cleanup();
      expect(at("over")).toEqual({ side: "over", top: "300px", left: "500px" });
    });

    it("bridges the gap to its target on the side it went, so the pointer can cross onto it", () => {
      at("left");
      const classes = tipOf()!.className.split(" ");
      for (const c of [
        "data-[side=below]:before:-top-2",
        "data-[side=above]:before:-bottom-2",
        "data-[side=left]:before:-right-2",
        "data-[side=right]:before:-left-2",
      ])
        expect(classes).toContain(c);
    });
  });

  describe("Escape", () => {
    function inDialog() {
      const cancel = vi.fn();
      render(() => (
        <dialog open>
          <Tooltip text="Close it">
            <button type="button">x</button>
          </Tooltip>
        </dialog>
      ));
      const dialog = document.querySelector("dialog")!;
      // What the browser does: an Escape nobody cancelled asks the dialog to close.
      dialog.addEventListener("keydown", (e) => {
        if (e.key === "Escape" && !e.defaultPrevented) dialog.dispatchEvent(new Event("cancel"));
      });
      dialog.addEventListener("cancel", cancel);
      const button = screen.getByRole("button");
      return { cancel, button, wrapper: wrapperOf(button) };
    }

    it("closes only the tooltip, then the next Escape reaches the dialog", () => {
      const { cancel, button, wrapper } = inDialog();
      hover(wrapper);

      expect(fireEvent.keyDown(button, { key: "Escape" })).toBe(false); // default prevented
      flush();
      expect(tips()).toHaveLength(0);
      expect(cancel).not.toHaveBeenCalled();

      expect(fireEvent.keyDown(button, { key: "Escape" })).toBe(true);
      expect(cancel).toHaveBeenCalledTimes(1);
    });

    it("leaves other keys alone while open", () => {
      const { button, wrapper } = inDialog();
      hover(wrapper);
      expect(fireEvent.keyDown(button, { key: "Enter" })).toBe(true);
      expect(tips()).toHaveLength(1);
    });

    it("stops listening once the component is gone", () => {
      const { cancel, wrapper } = inDialog();
      hover(wrapper);
      cleanup();
      expect(fireEvent.keyDown(document.body, { key: "Escape" })).toBe(true);
      expect(cancel).not.toHaveBeenCalled();
    });
  });

  describe("describe", () => {
    it("adds the tooltip's id to the target, the one the open tooltip has", () => {
      focusVisible(true);
      const { button } = one(true);
      fireEvent.focusIn(button);
      expect(button.getAttribute("aria-describedby")).toBe(tipOf()!.id);
    });

    it("leaves the target alone by default", () => {
      const { button } = one();
      expect(button.hasAttribute("aria-describedby")).toBe(false);
    });

    it("keeps an id the target already points at", () => {
      render(() => (
        <Tooltip text="Hint" describe>
          <input aria-describedby="name-error" />
        </Tooltip>
      ));
      const [error, own] = screen.getByRole("textbox").getAttribute("aria-describedby")!.split(" ");
      expect(error).toBe("name-error");
      expect(own).toMatch(/\S/);
    });

    it("gives two tooltips two ids", () => {
      render(() => (
        <>
          <Tooltip text="a" describe>
            <button type="button">a</button>
          </Tooltip>
          <Tooltip text="b" describe>
            <button type="button">b</button>
          </Tooltip>
        </>
      ));
      const [a, b] = screen.getAllByRole("button").map((x) => x.getAttribute("aria-describedby"));
      expect(a).not.toBe(b);
    });
  });
});
