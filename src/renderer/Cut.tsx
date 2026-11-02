import type { JSX } from "@solidjs/web";
import type { Side } from "./placeTip";
import { Tooltip } from "./Tooltip";

/**
 * One line of text that is cut with an ellipsis when it does not fit. `full` shows in a tooltip, and only
 * when the line, or a part of a `Middle` in it, is cut (R9.4): measured when the tooltip would show, so it
 * follows the window. `class` goes on the line; `wrap` on the tooltip's wrapper, which must be able to
 * shrink. In a row, `side` is `over`: the tooltip lies on its own line, not on the next row.
 */
export function Cut(props: {
  full: string;
  class?: string;
  wrap?: string;
  testid?: string;
  side?: Side;
  children: JSX.Element;
}) {
  let line!: HTMLSpanElement;
  const cut = () => [line, ...line.children].some((e) => e.scrollWidth > e.clientWidth);
  return (
    <Tooltip
      text={cut() ? props.full : undefined}
      class={`min-w-0 ${props.wrap ?? ""}`}
      side={props.side}
    >
      <span
        ref={(e) => (line = e)}
        data-testid={props.testid}
        class={`min-w-0 truncate ${props.class ?? ""}`}
      >
        {props.children}
      </span>
    </Tooltip>
  );
}

/**
 * A line cut in the middle, inside a `Cut` with `class="flex"`: `head` gives way first behind an
 * ellipsis, then `tail` from its end. The factors are far apart, so a tail that fits loses no fraction
 * of a pixel, which would already show an ellipsis.
 */
export function Middle(props: { head: string; tail: string }) {
  return (
    <>
      <span class="min-w-0 shrink-100000 truncate">{props.head}</span>
      <span class="min-w-0 truncate">{props.tail}</span>
    </>
  );
}
