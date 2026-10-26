import { createMemo, For, Show } from "solid-js";
import { HTTP_GROUPS, httpGroup, type HttpGroup } from "../shared/http";
import {
  groupCounts,
  narrowedCalls,
  shownCalls,
  totalCalls,
  type CallRef,
  type ListedCall,
} from "./http";
import { findNode, formatMillis, nodeName } from "./nodes";
import type { AppState } from "./reduce";

const CHIPS: readonly { group: HttpGroup | "all"; label: string }[] = [
  { group: "all", label: "All" },
  ...HTTP_GROUPS,
];

const BADGE: Record<HttpGroup, string> = {
  "1xx": "bg-zinc-600",
  "2xx": "bg-green-700",
  "3xx": "bg-sky-700",
  "4xx": "bg-amber-700",
  "5xx": "bg-red-600",
  none: "bg-red-900",
};
const badge = (status: number | null) => {
  const g = httpGroup(status);
  return g ? BADGE[g] : "bg-zinc-600"; // a code outside 100–599
};

const CHIP = "rounded border px-2 py-0.5 text-xs";
const CHIP_ON = `${CHIP} border-sky-600 bg-sky-100 text-sky-900 dark:bg-sky-900/40 dark:text-sky-200`;
const CHIP_OFF = `${CHIP} border-zinc-300 hover:bg-zinc-100 dark:border-zinc-600 dark:hover:bg-zinc-800`;

function Row(props: { item: ListedCall; selected: boolean; onSelect(): void }) {
  const call = () => props.item.call;
  const time = () => (call().millis === null ? "—" : formatMillis(call().millis!));
  return (
    <button
      type="button"
      data-testid="http-call"
      data-status={call().status ?? "none"}
      aria-pressed={props.selected ? "true" : "false"}
      class={`flex w-full items-center gap-3 border-b border-zinc-100 px-3 py-1 text-left text-xs hover:bg-zinc-100 dark:border-zinc-800 dark:hover:bg-zinc-800 ${props.selected ? "bg-sky-100 dark:bg-sky-900/40" : ""}`}
      onClick={props.onSelect}
    >
      <span
        data-testid="http-call-status"
        title={call().status === null ? "No response" : undefined}
        class={`w-10 shrink-0 rounded px-1 text-center font-mono text-white ${badge(call().status)}`}
      >
        {call().status ?? "—"}
      </span>
      <span data-testid="http-call-method" class="w-14 shrink-0 font-mono font-medium">
        {call().method}
      </span>
      <span
        data-testid="http-call-url"
        title={call().url ?? call().error ?? undefined}
        class="min-w-0 flex-1 truncate font-mono"
      >
        {call().url ?? call().error?.split("\n")[0]}
      </span>
      <span
        data-testid="http-call-test"
        class="max-w-40 shrink-0 truncate text-zinc-600 dark:text-zinc-400"
      >
        {nodeName(props.item.test)}
      </span>
      <Show when={call().attempt}>
        {(a) => (
          <span data-testid="http-call-attempt" class="shrink-0">
            {`attempt ${a().n} of ${a().of}`}
          </span>
        )}
      </Show>
      <Show when={call().via}>
        {(via) => (
          <span
            data-testid="http-call-via"
            class="max-w-56 shrink-0 truncate text-zinc-600 dark:text-zinc-400"
          >
            {`via ${via()}`}
          </span>
        )}
      </Show>
      <span data-testid="http-call-time" class="w-16 shrink-0 text-right">
        {time()}
      </span>
    </button>
  );
}

/** The HTTP tab: status-group chips over one row per call of the tests under the tree selection. */
export function HttpCalls(props: {
  state: AppState;
  selectCall(ref: CallRef): void;
  toggleGroup(g: HttpGroup | "all"): void;
}) {
  const narrowed = createMemo(() => narrowedCalls(props.state));
  const counts = createMemo(() => groupCounts(narrowed()));
  const shown = createMemo(() => shownCalls(narrowed(), props.state.httpGroups));
  const pressed = (g: HttpGroup | "all") =>
    g === "all" ? props.state.httpGroups.length === 0 : props.state.httpGroups.includes(g);
  const isSelected = ({ key, index }: CallRef) =>
    props.state.selectedCall?.key === key && props.state.selectedCall.index === index;
  const empty = () => {
    if (totalCalls(props.state.calls) === 0)
      return "No HTTP calls yet. Calls appear when a feature finishes.";
    if (narrowed().length > 0) return "No calls in the chosen groups";
    const { tree, selected } = props.state;
    const scope = (selected && tree && findNode(tree, selected)) || tree;
    return `No HTTP calls in ${scope ? nodeName(scope) : "the project"}`;
  };
  return (
    <div class="flex min-h-0 flex-1 flex-col">
      <div class="flex shrink-0 flex-wrap gap-1 border-b border-zinc-100 px-3 py-1 dark:border-zinc-800">
        <For each={CHIPS}>
          {({ group, label }) => (
            <button
              type="button"
              data-testid="http-group"
              data-group={group}
              aria-pressed={pressed(group) ? "true" : "false"}
              class={pressed(group) ? CHIP_ON : CHIP_OFF}
              onClick={() => props.toggleGroup(group)}
            >
              {`${label} ${counts()[group]}`}
            </button>
          )}
        </For>
      </div>
      <div data-testid="http-calls" class="min-h-0 flex-1 overflow-auto">
        <Show
          when={shown().length > 0}
          fallback={
            <p data-testid="http-empty" class="px-3 py-2 text-sm text-zinc-600 dark:text-zinc-400">
              {empty()}
            </p>
          }
        >
          {/* keyed by place, not object: rows (and the focus on one) outlive the list being rebuilt */}
          <For each={shown()} keyed={(l) => `${l.ref.key}\n${l.ref.index}`}>
            {(l) => (
              <Row
                item={l()}
                selected={isSelected(l().ref)}
                onSelect={() => props.selectCall(l().ref)}
              />
            )}
          </For>
        </Show>
      </div>
    </div>
  );
}
