import { Check, Globe } from "lucide";
import { createMemo, flush, For, Show } from "solid-js";
import { HTTP_GROUPS, httpGroup, type HttpGroup } from "../shared/http";
import { groupCounts, shownCalls, totalCalls, type CallRef, type ListedCall } from "./http";
import { Cut } from "./Cut";
import { EmptyState } from "./EmptyState";
import { Icon } from "./Icon";
import { findNode, formatMillis, nodeName } from "./nodes";
import type { AppState } from "./reduce";
import { Tooltip } from "./Tooltip";
import { badge, chip, chipPressed, TINT } from "./ui";
import { VirtualList, type VirtualListHandle } from "./VirtualList";

const CHIPS: readonly { group: HttpGroup | "all"; label: string }[] = [
  { group: "all", label: "All" },
  ...HTTP_GROUPS,
];

/** A call's accessible name: what its row reads, status first (R5.4). */
export const callName = (l: ListedCall): string =>
  [
    l.call.status ?? "no status",
    l.call.method,
    l.call.url ?? l.call.error?.split("\n")[0] ?? "",
    nodeName(l.test),
  ].join(", ");

/** The tint of each status group's badge: the colour that group has everywhere else (R1.3, R7.2). */
export const GROUP_TINT: Record<HttpGroup, keyof typeof TINT> = {
  "1xx": "skipped",
  "2xx": "passed",
  "3xx": "running",
  "4xx": "cancelled",
  "5xx": "failed",
  none: "failed",
};

/** The tint of a status; a code outside 100–599 is neutral. */
export const statusTint = (status: number | null): keyof typeof TINT => {
  const g = httpGroup(status);
  return g ? GROUP_TINT[g] : "skipped";
};

/** The group of a status in words, for a tooltip; a code outside 100–599 has no group, so it names itself. */
export const groupLabel = (status: number | null): string => {
  const g = httpGroup(status);
  return g ? HTTP_GROUPS.find((x) => x.group === g)!.label : `Status ${status}`;
};

// A badge is the code in tabular mono on its group's solid tint, never white on a fill. A fixed width keeps the
// methods in line down the list.
const BADGE = `${badge} w-11 shrink-0 justify-center font-mono tabular-nums`;
// A row's tooltips stay on the row: beside the badge, over the cut text.
const ROW =
  "flex h-7 items-center gap-3 border-b border-line px-3 text-xs transition-colors duration-(--duration-fast) ease-standard";
// The rows scroll, so a focus ring goes inside them (R10.1).
const ITEM = "focus-visible:-outline-offset-2";
const SELECTED = "bg-selected shadow-[inset_2px_0_0_var(--color-accent)]";

// A place, not an object: a row (and the focus on it) outlives the list being rebuilt.
const callKey = ({ key, index }: CallRef) => `${key}\n${index}`;

/** What a row draws; the list around it makes it an option, and a key on the row selects it too. */
function Row(props: { item: ListedCall; selected: boolean; onSelect(): void }) {
  const call = () => props.item.call;
  const time = () => (call().millis === null ? "—" : formatMillis(call().millis!));
  return (
    <div class={`${ROW} ${props.selected ? SELECTED : "hover:bg-hover"}`} onClick={props.onSelect}>
      <Tooltip text={groupLabel(call().status)} describe class="shrink-0" side="right">
        <span data-testid="http-call-status" class={`${BADGE} ${TINT[statusTint(call().status)]}`}>
          {call().status ?? "—"}
        </span>
      </Tooltip>
      <span data-testid="http-call-method" class="w-14 shrink-0 font-mono font-medium">
        {call().method}
      </span>
      <Cut
        full={call().url ?? call().error ?? ""}
        testid="http-call-url"
        class="font-mono"
        wrap="flex-1"
        side="over"
      >
        {call().url ?? call().error?.split("\n")[0]}
      </Cut>
      <Cut
        full={nodeName(props.item.test)}
        testid="http-call-test"
        class="max-w-40 text-muted"
        wrap="shrink-0"
        side="over"
      >
        {nodeName(props.item.test)}
      </Cut>
      <Show when={call().attempt}>
        {(a) => (
          <span data-testid="http-call-attempt" class="shrink-0 text-muted tabular-nums">
            {`attempt ${a().n} of ${a().of}`}
          </span>
        )}
      </Show>
      <Show when={call().via}>
        {(via) => (
          <Cut
            full={`via ${via()}`}
            testid="http-call-via"
            class="max-w-56 text-muted"
            wrap="shrink-0"
            side="over"
          >
            {`via ${via()}`}
          </Cut>
        )}
      </Show>
      <span data-testid="http-call-time" class="w-16 shrink-0 text-right font-mono tabular-nums">
        {time()}
      </span>
    </div>
  );
}

/** The HTTP tab: status-group chips over one row per call of the tests under the tree selection. */
export function HttpCalls(props: {
  state: AppState;
  /** The calls of the tests under the tree selection, from the panel, which counts them too. */
  narrowed: ListedCall[];
  selectCall(ref: CallRef): void;
  toggleGroup(g: HttpGroup | "all"): void;
}) {
  const narrowed = () => props.narrowed;
  const counts = createMemo(() => groupCounts(narrowed()));
  const shown = createMemo(() => shownCalls(narrowed(), props.state.httpGroups));
  const pressed = (g: HttpGroup | "all") =>
    g === "all" ? props.state.httpGroups.length === 0 : props.state.httpGroups.includes(g);
  const selectedKey = () => {
    const c = props.state.selectedCall;
    return c ? callKey(c) : null;
  };
  let list: VirtualListHandle | undefined;
  // A chip change the person made shows the selected call again (R4.7); new calls never scroll. The list
  // settles first (it keeps the rows in view still when rows leave above, R4.6): an effect on the chips would
  // run before that and lose to it. Not `revealSelectedOn`: the `<Show>` below remounts the list when the
  // chips empty it and fill it again, and a fresh list skips it.
  const toggle = (group: HttpGroup | "all") => {
    props.toggleGroup(group);
    flush();
    const key = selectedKey();
    if (key !== null) list?.reveal(key);
  };
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
      <div class="flex shrink-0 flex-wrap gap-2 border-b border-line px-3 py-2">
        <For each={CHIPS}>
          {({ group, label }) => (
            <button
              type="button"
              data-testid="http-group"
              data-group={group}
              aria-pressed={pressed(group) ? "true" : "false"}
              class={`${pressed(group) ? chipPressed : chip} tabular-nums`}
              onClick={() => toggle(group)}
            >
              {/* the check shows a pressed chip by shape, not only by fill (R7) */}
              <Show when={pressed(group)}>
                <Icon icon={Check} size={14} />
              </Show>
              {`${label} ${counts()[group]}`}
            </button>
          )}
        </For>
      </div>
      <Show
        when={shown().length > 0}
        fallback={
          <div data-testid="http-calls" class="min-h-0 flex-1 overflow-auto">
            <EmptyState icon={Globe} text={empty()} testid="http-empty" />
          </div>
        }
      >
        <VirtualList
          items={shown()}
          keyOf={(l) => callKey(l.ref)}
          rowHeight={28}
          role="listbox"
          itemRole="option"
          label="HTTP calls"
          scrollerTestid="http-calls"
          testid="http-list"
          itemClass={ITEM}
          attrs={(l, i) => ({
            "data-testid": "http-call",
            "data-status": l.call.status ?? "none",
            "aria-selected": callKey(l.ref) === selectedKey() ? "true" : "false",
            "aria-label": callName(l),
            "aria-setsize": shown().length,
            "aria-posinset": i + 1,
          })}
          selectedKey={selectedKey()}
          onSelect={(l) => props.selectCall(l.ref)}
          ref={(h) => (list = h)}
        >
          {(l) => (
            <Row
              item={l}
              selected={callKey(l.ref) === selectedKey()}
              onSelect={() => props.selectCall(l.ref)}
            />
          )}
        </VirtualList>
      </Show>
    </div>
  );
}
