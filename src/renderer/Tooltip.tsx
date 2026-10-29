import type { JSX } from "@solidjs/web";
import { createUniqueId, onCleanup, onSettled } from "solid-js";
import { placeTip } from "./placeTip";

const DELAY = 500; // the pointer must rest this long on the target

/** The hide of the tooltip that is open: only one at a time. */
let openTip: (() => void) | null = null;

// A fade over --duration-fast; the global reduced-motion rule turns it off. `open:` is :popover-open.
// The ::before bridges the 6 px gap to the target, so the pointer can cross onto the tooltip.
const tipClass = [
  "m-0 inset-auto w-max max-w-80 overflow-visible rounded-md border border-line bg-raised px-2 py-1",
  "text-xs break-words text-ink shadow-float",
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

/**
 * A styled tooltip for the first element of `children` (spec R9.4); it replaces a native `title`.
 * The tooltip is a child of the wrapper, so the pointer moving from the target onto it never leaves
 * the wrapper. The wrapper, not the target, listens: a disabled button gets no pointer events.
 * `describe` adds `aria-describedby` to the target: leave it off when the text is the target's name.
 */
export function Tooltip(props: { text: string; children: JSX.Element; describe?: boolean }) {
  const id = createUniqueId();
  let wrap!: HTMLSpanElement;
  let tip!: HTMLDivElement;
  let timer: ReturnType<typeof setTimeout> | undefined;

  // Capture phase and only while open, so it takes the one Escape that closes the tooltip and
  // nothing else: not a dialog under it, not a page that goes back on Escape.
  const onKey = (e: KeyboardEvent) => {
    if (e.key !== "Escape") return;
    e.stopPropagation();
    e.preventDefault();
    hide();
  };

  const hide = () => {
    clearTimeout(timer);
    tip.hidePopover?.(); // jsdom has no Popover API
    tip.removeAttribute("data-open");
    document.removeEventListener("keydown", onKey, true);
    document.removeEventListener("scroll", hide, true);
    if (openTip === hide) openTip = null;
  };

  const show = () => {
    clearTimeout(timer);
    if (openTip && openTip !== hide) openTip();
    openTip = hide;
    tip.showPopover?.(); // shown first: it has no size while display is none
    const target = (wrap.firstElementChild ?? wrap).getBoundingClientRect();
    const { width, height } = tip.getBoundingClientRect();
    const at = placeTip(target, { width, height }, { width: innerWidth, height: innerHeight });
    tip.style.top = `${at.top}px`;
    tip.style.left = `${at.left}px`;
    tip.toggleAttribute("data-above", at.above);
    tip.setAttribute("data-open", "");
    document.addEventListener("keydown", onKey, true);
    document.addEventListener("scroll", hide, true);
  };

  onSettled(() => {
    if (props.describe) wrap.firstElementChild?.setAttribute("aria-describedby", id);
  });
  onCleanup(hide);

  return (
    <span
      ref={(e) => (wrap = e)}
      class="inline-flex"
      onPointerEnter={() => {
        clearTimeout(timer);
        timer = setTimeout(show, DELAY);
      }}
      onPointerLeave={hide}
      onFocusIn={(e) => keyboardFocus(e.target) && show()}
      onFocusOut={hide}
    >
      {props.children}
      <div ref={(e) => (tip = e)} popover="manual" role="tooltip" id={id} class={tipClass}>
        {props.text}
      </div>
    </span>
  );
}
