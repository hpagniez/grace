import { createMemo, For } from "solid-js";
import type { TestNode } from "../shared/model";
import { Cut } from "./Cut";
import { formatMillis, STATUS_LABEL } from "./nodes";
import type { AppState } from "./reduce";
import { StatusMark } from "./StatusMark";
import { Tooltip } from "./Tooltip";
import { VirtualList } from "./VirtualList";

const ROW_HEIGHT = 24;
/** The fewest lines the table shows before the details pane scrolls instead (data-driven view R2.7). */
const MIN_LINES = 8;
const LINE = "grid h-6 items-center gap-2 px-2 text-xs";
// The lines scroll, so a focus ring goes inside them.
const ITEM = "focus-visible:-outline-offset-2";

/** The columns of all the rows, in the order first met (R2.2). */
export function exampleColumns(rows: TestNode[]): string[] {
  const seen = new Set<string>();
  for (const r of rows) for (const c of Object.keys(r.example ?? {})) seen.add(c);
  return [...seen];
}

/** An outline's Examples as a table: a line per row with its values, status and time; a click or Enter opens the row (R2, R3). */
export function Examples(props: { outline: TestNode; state: AppState; select(key: string): void }) {
  const rows = () => props.outline.children;
  const columns = createMemo(() => exampleColumns(rows()));
  // Status, number, the values sharing what is left, time: the header and every line, so they line up.
  const grid = () => {
    const n = columns().length;
    return {
      "grid-template-columns": `1.5rem 2.5rem ${n ? `repeat(${n}, minmax(0, 1fr)) ` : ""}4.5rem`,
    };
  };
  const status = (key: string) => props.state.statuses[key] ?? "notRun";
  const time = (key: string) => {
    const ms = props.state.results[key]?.durationMillis;
    return ms === undefined ? "" : formatMillis(ms);
  };
  /** A line's accessible name: number, values by column, status, time (R3.3). */
  const name = (r: TestNode, index: number) =>
    `Row ${index + 1}: ${[
      ...columns()
        .filter((c) => r.example?.[c])
        .map((c) => `${c} ${r.example![c]}`),
      STATUS_LABEL[status(r.key)],
      time(r.key),
    ]
      .filter(Boolean)
      .join(", ")}`;
  return (
    <div
      data-testid="examples"
      class="flex flex-col overflow-hidden rounded-lg border border-line"
      style={{ "min-height": `${(Math.min(MIN_LINES, rows().length) + 1) * ROW_HEIGHT + 2}px` }}
    >
      <div
        data-testid="examples-head"
        class={`${LINE} shrink-0 border-b border-line font-medium text-muted`}
        style={grid()}
      >
        <span />
        <span>#</span>
        <For each={columns()}>
          {(c) => (
            <Cut full={c} testid="example-column" side="over">
              {c}
            </Cut>
          )}
        </For>
        <span class="text-right">Time</span>
      </div>
      <VirtualList
        items={rows()}
        keyOf={(r) => r.key}
        rowHeight={ROW_HEIGHT}
        role="listbox"
        itemRole="option"
        label="Examples"
        testid="examples-list"
        itemClass={ITEM}
        attrs={(r, index) => ({
          "data-testid": "example",
          "data-status": status(r.key),
          "aria-label": name(r, index),
          "aria-selected": "false",
        })}
        selectedKey={null}
        selectOnMove={false}
        onSelect={(r) => props.select(r.key)}
      >
        {(r, index) => (
          <div class={`${LINE} hover:bg-hover`} style={grid()} onClick={() => props.select(r.key)}>
            <Tooltip text={STATUS_LABEL[status(r.key)]} side="right">
              <StatusMark status={status(r.key)} />
            </Tooltip>
            <span class="text-muted tabular-nums">{index + 1}</span>
            <For each={columns()}>
              {(c) => (
                <Cut
                  full={r.example?.[c] ?? ""}
                  testid="example-value"
                  class="font-mono"
                  side="over"
                >
                  {r.example?.[c] ?? ""}
                </Cut>
              )}
            </For>
            <span data-testid="example-time" class="text-right font-mono text-muted tabular-nums">
              {time(r.key)}
            </span>
          </div>
        )}
      </VirtualList>
    </div>
  );
}
