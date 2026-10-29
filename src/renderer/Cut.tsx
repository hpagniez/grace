import type { JSX } from "@solidjs/web";
import { Tooltip } from "./Tooltip";

/**
 * One line of text that is cut with an ellipsis when it does not fit. `full` shows in a tooltip, and only
 * when the line is cut (R9.4): measured when the tooltip would show, so it follows the window.
 * `class` goes on the line; `wrap` on the tooltip's wrapper, which must be able to shrink.
 */
export function Cut(props: {
  full: string;
  class?: string;
  wrap?: string;
  testid?: string;
  children: JSX.Element;
}) {
  let line!: HTMLSpanElement;
  const cut = () => line.scrollWidth > line.clientWidth;
  return (
    <Tooltip text={cut() ? props.full : undefined} class={`min-w-0 ${props.wrap ?? ""}`}>
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
