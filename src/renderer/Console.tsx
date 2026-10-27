import type { JSX } from "@solidjs/web";
import { createEffect, For } from "solid-js";

const NEAR_BOTTOM_PX = 20;

const lineClass = (line: string) =>
  line.includes(" ERROR ") ? "text-red-400" : line.includes(" WARN ") ? "text-amber-400" : "";

/** Karate's output; `children` (the banner) come first, in the same scroller. */
export function Console(props: { lines: string[]; children?: JSX.Element }) {
  let el!: HTMLDivElement;
  let pinned = true;
  const onScroll = () => {
    pinned = el.scrollHeight - el.scrollTop - el.clientHeight <= NEAR_BOTTOM_PX;
  };
  createEffect(
    // the array is replaced on every append; its length stops changing at the line cap
    () => props.lines,
    () => {
      if (pinned) el.scrollTop = el.scrollHeight;
    },
  );
  return (
    <div
      ref={(e) => (el = e)}
      data-testid="console"
      onScroll={onScroll}
      class="min-h-0 flex-1 overflow-auto whitespace-pre-wrap bg-zinc-900 [scrollbar-color:var(--color-zinc-500)_transparent] px-3 py-2 font-mono text-xs text-zinc-200 dark:bg-zinc-950 dark:shadow-[inset_0_2px_6px_rgb(0_0_0/0.6)]"
    >
      {props.children}
      <For each={props.lines}>{(line) => <div class={lineClass(line)}>{line}</div>}</For>
    </div>
  );
}
