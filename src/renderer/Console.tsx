import { createEffect, For } from "solid-js";

const NEAR_BOTTOM_PX = 20;

const lineClass = (line: string) =>
  line.includes(" ERROR ") ? "text-red-400" : line.includes(" WARN ") ? "text-amber-400" : "";

export function Console(props: { lines: string[] }) {
  let el!: HTMLDivElement;
  let pinned = true;
  const onScroll = () => {
    pinned = el.scrollHeight - el.scrollTop - el.clientHeight <= NEAR_BOTTOM_PX;
  };
  createEffect(
    () => props.lines.length,
    () => {
      if (pinned) el.scrollTop = el.scrollHeight;
    },
  );
  return (
    <div
      ref={(e) => (el = e)}
      data-testid="console"
      onScroll={onScroll}
      class="h-48 shrink-0 overflow-auto whitespace-pre-wrap bg-zinc-900 px-3 py-2 font-mono text-xs text-zinc-200"
    >
      <For each={props.lines}>{(line) => <div class={lineClass(line)}>{line}</div>}</For>
    </div>
  );
}
