import type { JSX } from "@solidjs/web";
import { createEffect, createSignal, createUniqueId, flush, onCleanup, Show } from "solid-js";
import { placeTip } from "./placeTip";

const DELAY = 500; // the pointer must rest this long on the target

/** The hide of the tooltip that is open: only one at a time. */
let openTip: (() => void) | null = null;

/**
 * Where a target was last pressed, until the pointer leaves a tooltip's wrapper. A press often redraws
 * the row under a pointer that has not moved, and the new wrapper gets a pointerenter there: it must not
 * start a pause, or a tooltip would come up over the row below, which the person is about to click.
 */
let pressedAt: { x: number; y: number } | null = null;
const SAME_SPOT = 2; // px

// A fade over --duration-fast; the global reduced-motion rule turns it off. `open:` is :popover-open.
// The ::before bridges the 6 px gap to the target, so the pointer can cross onto the tooltip.
// The tooltip sits inside rows that may truncate, centre or shout: it resets what text inherits.
const tipClass = [
  "m-0 inset-auto w-max max-w-80 overflow-visible rounded-md border border-line bg-raised px-2 py-1",
  "font-sans text-xs font-normal break-words whitespace-normal normal-case not-italic tracking-normal text-left text-ink shadow-float",
  "opacity-0 transition-[opacity,display,overlay] transition-discrete duration-(--duration-fast) ease-standard",
  "open:opacity-100 starting:open:opacity-0",
  "before:absolute before:inset-x-0 before:-top-2 before:h-2",
  "data-above:before:top-auto data-above:before:-bottom-2",
].join(" ");

/** Whether the focus that just arrived came from the keyboard. An engine that cannot say shows it. */
function keyboardFocus(el: EventTarget | null) {
  try {
    return (el as Element).matches(":focus-visible");
  } catch {
    return true;
  }
}

/** The tooltip is a child of the target's wrapper in the DOM: a click on it must not reach a row or a toolbar. */
const keepHere = (e: Event) => e.stopPropagation();

/**
 * A styled tooltip for the first element of `children` (spec R9.4); it replaces a native `title`.
 * The wrapper, not the target, listens: a disabled button gets no pointer events. The tooltip is in the
 * DOM only while open, as a child of the wrapper, so the pointer moving from the target onto it never
 * leaves the wrapper. No `text` (a disabled control with no reason) means no tooltip.
 * `class` goes on the wrapper, after `inline-flex`: a row that truncates needs `min-w-0` there.
 * `describe` adds the tooltip to the target's `aria-describedby`: leave it off when the text is the
 * target's name.
 */
export function Tooltip(props: {
  text: string | undefined;
  children: JSX.Element;
  describe?: boolean;
  class?: string;
}) {
  const id = createUniqueId();
  const [open, setOpen] = createSignal(false);
  let wrap!: HTMLSpanElement;
  let tip: HTMLDivElement | undefined;
  let shown = false; // the signal, readable at once
  let timer: ReturnType<typeof setTimeout> | undefined;

  // Capture phase and only while open, so it takes the one Escape that closes the tooltip and
  // nothing else: not a dialog under it, not a page that goes back on Escape.
  const onKey = (e: KeyboardEvent) => {
    if (e.key !== "Escape") return;
    e.stopPropagation();
    e.preventDefault();
    hide();
  };

  // Only a scroll that moves the target hides the tooltip: the page, or a box that holds it. A console
  // following its tail leaves the tooltip of a disabled button alone, as a run goes on.
  const onScroll = (e: Event) => {
    if (e.target instanceof Node && e.target.contains(wrap)) hide();
  };

  // Taking the element out of the DOM hides the popover.
  const hide = () => {
    clearTimeout(timer);
    if (!shown) return;
    shown = false;
    document.removeEventListener("keydown", onKey, true);
    document.removeEventListener("scroll", onScroll, true);
    if (openTip === hide) openTip = null;
    setOpen(false);
  };

  const show = () => {
    clearTimeout(timer);
    if (shown || !props.text) return;
    openTip?.();
    openTip = hide;
    shown = true;
    setOpen(true);
    flush(); // the tooltip must exist to be shown and measured
    tip!.showPopover?.(); // jsdom has no Popover API; shown first: it has no size while display is none
    const target = (wrap.firstElementChild ?? wrap).getBoundingClientRect();
    const { width, height } = tip!.getBoundingClientRect();
    const at = placeTip(target, { width, height }, { width: innerWidth, height: innerHeight });
    tip!.style.top = `${at.top}px`;
    tip!.style.left = `${at.left}px`;
    tip!.toggleAttribute("data-above", at.above);
    document.addEventListener("keydown", onKey, true);
    document.addEventListener("scroll", onScroll, true);
  };

  // Added to the target's ids, not set over them: an input may already point at its error message.
  createEffect(
    () => props.describe,
    (on) => {
      const target = wrap.firstElementChild;
      if (!on || !target) return;
      const ids = target.getAttribute("aria-describedby");
      target.setAttribute("aria-describedby", ids ? `${ids} ${id}` : id);
      return () => {
        const rest = (target.getAttribute("aria-describedby") ?? "")
          .split(" ")
          .filter((x) => x && x !== id);
        if (rest.length) target.setAttribute("aria-describedby", rest.join(" "));
        else target.removeAttribute("aria-describedby");
      };
    },
  );
  onCleanup(hide);

  return (
    <span
      ref={(e) => (wrap = e)}
      class={props.class ? `inline-flex ${props.class}` : "inline-flex"}
      onPointerEnter={(e) => {
        if (pressedAt && Math.hypot(e.clientX - pressedAt.x, e.clientY - pressedAt.y) <= SAME_SPOT)
          return;
        pressedAt = null;
        clearTimeout(timer);
        timer = setTimeout(show, DELAY);
      }}
      onPointerLeave={() => {
        pressedAt = null;
        hide();
      }}
      // A press of the target: the person acted on it. The tooltip is hoverable, so it would stay up over
      // whatever lies below and take the next click. A press inside the tooltip leaves it open.
      onPointerDown={(e) => {
        if (tip?.contains(e.target as Node)) return;
        pressedAt = { x: e.clientX, y: e.clientY };
        hide();
      }}
      onFocusIn={(e) => keyboardFocus(e.target) && show()}
      onFocusOut={hide}
    >
      {props.children}
      <Show when={open()}>
        <div
          ref={(e) => {
            tip = e;
            // Native listeners: Solid's own run at the document, after a row's native ones.
            for (const type of ["click", "dblclick", "contextmenu"])
              e.addEventListener(type, keepHere);
          }}
          popover="manual"
          role="tooltip"
          id={id}
          class={tipClass}
        >
          {props.text}
        </div>
      </Show>
    </span>
  );
}
