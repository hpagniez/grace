import { cleanup, fireEvent, render, screen } from "@solidjs/testing-library";
import { flush } from "solid-js";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
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

  it("hides on scroll anywhere", () => {
    const { wrapper } = one();
    hover(wrapper);
    fireEvent.scroll(document.body);
    flush();
    expect(tips()).toHaveLength(0);
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
      for (const type of ["click", "dblclick", "contextmenu"])
        tip.dispatchEvent(new MouseEvent(type, { bubbles: true, cancelable: true }));
      expect(delegated).not.toHaveBeenCalled();
      expect(native).not.toHaveBeenCalled();

      fireEvent.click(screen.getByRole("button")); // the target itself still reaches the row
      expect(delegated).toHaveBeenCalledTimes(1);
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
