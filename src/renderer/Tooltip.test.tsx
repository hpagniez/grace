import { cleanup, fireEvent, render, screen } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Tooltip } from "./Tooltip";

// Only the timers: a full fake would stop Solid's microtask flush.
beforeEach(() => vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] }));
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

const isOpen = (tip: HTMLElement) => tip.hasAttribute("data-open");
const wrapperOf = (el: HTMLElement) => el.closest("span.inline-flex") as HTMLElement;

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
  return { button, wrapper: wrapperOf(button), tip: screen.getByRole("tooltip", { hidden: true }) };
}

describe("Tooltip", () => {
  it("is a manual popover with the text, inside the wrapper", () => {
    const { wrapper, tip } = one();
    expect(tip.getAttribute("popover")).toBe("manual");
    expect(tip.textContent).toBe("Refresh the tree");
    expect(wrapper.contains(tip)).toBe(true);
    expect(isOpen(tip)).toBe(false);
  });

  it("shows 500 ms after the pointer enters, not at 499", () => {
    const { wrapper, tip } = one();
    fireEvent.pointerEnter(wrapper);
    vi.advanceTimersByTime(499);
    expect(isOpen(tip)).toBe(false);
    vi.advanceTimersByTime(1);
    expect(isOpen(tip)).toBe(true);
  });

  it("does not show when the pointer leaves first, and hides when it leaves after", () => {
    const { wrapper, tip } = one();
    fireEvent.pointerEnter(wrapper);
    vi.advanceTimersByTime(300);
    fireEvent.pointerLeave(wrapper);
    vi.advanceTimersByTime(1000);
    expect(isOpen(tip)).toBe(false);

    fireEvent.pointerEnter(wrapper);
    vi.advanceTimersByTime(500);
    expect(isOpen(tip)).toBe(true);
    fireEvent.pointerLeave(wrapper);
    expect(isOpen(tip)).toBe(false);
  });

  it("keeps open while the pointer moves from the target onto the tooltip", () => {
    const { button, wrapper, tip } = one();
    fireEvent.pointerEnter(wrapper);
    vi.advanceTimersByTime(500);
    // The tooltip is a DOM child of the wrapper, so the browser fires no pointerleave on the wrapper;
    // pointerleave does not bubble, so the one on the button does not reach it either.
    fireEvent.pointerLeave(button);
    fireEvent.pointerEnter(tip);
    vi.advanceTimersByTime(1000);
    expect(isOpen(tip)).toBe(true);
    fireEvent.pointerLeave(wrapper);
    expect(isOpen(tip)).toBe(false);
  });

  it("shows at once on keyboard focus and hides on blur", () => {
    focusVisible(true);
    const { button, tip } = one();
    fireEvent.focusIn(button);
    expect(isOpen(tip)).toBe(true);
    fireEvent.focusOut(button);
    expect(isOpen(tip)).toBe(false);
  });

  it("ignores focus that is not :focus-visible (a mouse click)", () => {
    focusVisible(false);
    const { button, tip } = one();
    fireEvent.focusIn(button);
    expect(isOpen(tip)).toBe(false);
  });

  it("shows on focus when the engine cannot tell :focus-visible", () => {
    focusVisible("throws");
    const { button, tip } = one();
    fireEvent.focusIn(button);
    expect(isOpen(tip)).toBe(true);
  });

  it("shows even over a disabled button, which gets no pointer events itself", () => {
    render(() => (
      <Tooltip text="Pick a project first">
        <button type="button" disabled>
          Run
        </button>
      </Tooltip>
    ));
    const tip = screen.getByRole("tooltip", { hidden: true });
    fireEvent.pointerEnter(wrapperOf(screen.getByRole("button")));
    vi.advanceTimersByTime(500);
    expect(isOpen(tip)).toBe(true);
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
    const [first, second] = screen.getAllByRole("tooltip", { hidden: true });
    const [a, b] = screen.getAllByRole("button");
    fireEvent.pointerEnter(wrapperOf(a!));
    vi.advanceTimersByTime(500);
    expect(isOpen(first!)).toBe(true);
    fireEvent.focusIn(b!);
    expect(isOpen(first!)).toBe(false);
    expect(isOpen(second!)).toBe(true);
  });

  it("hides on scroll anywhere", () => {
    const { wrapper, tip } = one();
    fireEvent.pointerEnter(wrapper);
    vi.advanceTimersByTime(500);
    fireEvent.scroll(document.body);
    expect(isOpen(tip)).toBe(false);
  });

  it("calls showPopover and hidePopover when the engine has them", () => {
    const show = vi.fn();
    const hide = vi.fn();
    HTMLElement.prototype.showPopover = show;
    HTMLElement.prototype.hidePopover = hide;
    try {
      const { wrapper } = one();
      fireEvent.pointerEnter(wrapper);
      vi.advanceTimersByTime(500);
      expect(show).toHaveBeenCalledTimes(1);
      fireEvent.pointerLeave(wrapper);
      expect(hide).toHaveBeenCalled();
    } finally {
      // @ts-expect-error jsdom has neither; put it back as it was
      delete HTMLElement.prototype.showPopover;
      // @ts-expect-error jsdom has neither
      delete HTMLElement.prototype.hidePopover;
    }
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
      return {
        cancel,
        button,
        wrapper: wrapperOf(button),
        tip: screen.getByRole("tooltip", { hidden: true }),
      };
    }

    it("closes only the tooltip, then the next Escape reaches the dialog", () => {
      const { cancel, button, wrapper, tip } = inDialog();
      fireEvent.pointerEnter(wrapper);
      vi.advanceTimersByTime(500);

      expect(fireEvent.keyDown(button, { key: "Escape" })).toBe(false); // default prevented
      expect(isOpen(tip)).toBe(false);
      expect(cancel).not.toHaveBeenCalled();

      expect(fireEvent.keyDown(button, { key: "Escape" })).toBe(true);
      expect(cancel).toHaveBeenCalledTimes(1);
    });

    it("leaves other keys alone while open", () => {
      const { button, wrapper, tip } = inDialog();
      fireEvent.pointerEnter(wrapper);
      vi.advanceTimersByTime(500);
      expect(fireEvent.keyDown(button, { key: "Enter" })).toBe(true);
      expect(isOpen(tip)).toBe(true);
    });

    it("stops listening once the component is gone", () => {
      const { cancel, wrapper } = inDialog();
      fireEvent.pointerEnter(wrapper);
      vi.advanceTimersByTime(500);
      cleanup();
      expect(fireEvent.keyDown(document.body, { key: "Escape" })).toBe(true);
      expect(cancel).not.toHaveBeenCalled();
    });
  });

  describe("describe", () => {
    it("points the focusable child at the tooltip", () => {
      const { button, tip } = one(true);
      expect(tip.id).not.toBe("");
      expect(button.getAttribute("aria-describedby")).toBe(tip.id);
    });

    it("leaves the child alone by default", () => {
      const { button } = one();
      expect(button.hasAttribute("aria-describedby")).toBe(false);
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
      const [a, b] = screen.getAllByRole("tooltip", { hidden: true });
      expect(a!.id).not.toBe(b!.id);
    });
  });
});
