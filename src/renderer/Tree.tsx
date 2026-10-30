import {
  ChevronDown,
  ChevronRight,
  FileSearch,
  FileText,
  Folder,
  FolderOpen,
  LoaderCircle,
  Play,
  Shuffle,
} from "lucide";
import { createMemo, createSignal, For, Match, Show, Switch } from "solid-js";
import { FLAKY_TITLE } from "../shared/flaky";
import type { Api } from "../shared/ipc";
import type { Status, TestNode } from "../shared/model";
import { deriveStatus } from "../shared/status";
import { filterOn, matchingLeaves, type FilterState } from "../shared/tags";
import { EmptyState } from "./EmptyState";
import { Icon } from "./Icon";
import { leaves, nodeName, STATUS_LABEL } from "./nodes";
import { Notice } from "./Notice";
import type { AppState } from "./reduce";
import { StatusMark } from "./StatusMark";
import { Tooltip } from "./Tooltip";
import { rowName, treeRows, type TreeRow } from "./treeRows";
import { badge, tagChip, TINT } from "./ui";
import { VirtualList, type VirtualListHandle } from "./VirtualList";

interface Ctx {
  state: AppState;
  select(key: string): void;
  api: Api;
}

// The tree scrolls, so a focus ring goes inside its row (spec R10.1). A row's tooltips stay on the row
// (beside their target, or over the cut label): one below would take the click meant for the next row.
const INSET = "focus-visible:-outline-offset-2";
// The row element takes the focus; 8 px in from the tree's sides, where the rows always sat.
const ITEM = `group mx-2 rounded-sm ${INSET}`;
const MESSAGE = "min-h-0 flex-1 overflow-auto p-2";
const TAG = `${tagChip} shrink-0`;
const FLAKY = `${badge} ${TINT.cancelled} shrink-0`;
// 24 x 24 (R10.3). Muted until its row is hovered, focused or selected (the row sets `group`).
const RUN = `inline-flex size-6 shrink-0 items-center justify-center rounded-sm transition-colors duration-(--duration-fast) ease-standard enabled:group-hover:text-passed enabled:group-focus-within:text-passed disabled:cursor-not-allowed disabled:opacity-40 ${INSET}`;

const isTest = (n: TestNode) => n.kind === "scenario" || n.kind === "row";
// Users navigate by file: a feature shows its file name; its Feature title follows, muted.
const labelOf = (n: TestNode) => (n.kind === "feature" ? n.file.split("/").pop()! : nodeName(n));
// A folder, feature or outline matches when a test under it does; with the filter off all match.
const matches = (n: TestNode, f: FilterState) => !filterOn(f) || matchingLeaves(n, f).length > 0;
/** What a row's flaky chip says, or null: it counts the tests the tag filter hides too, as flakiness is not about tags. */
const flakyChip = (n: TestNode, flaky: AppState["flaky"]) => {
  const count = leaves(n).filter((l) => l.key in flaky).length;
  return count === 0 ? null : isTest(n) ? "flaky" : `${count} flaky`;
};

/** What a tree row draws; the list around it makes it a treeitem, named from the same status and flaky chip. */
function Row(props: {
  node: TestNode;
  level: number;
  open: boolean;
  toggle(): void;
  keyboardFocused: boolean;
  status: Status;
  flaky: string | null;
  ctx: Ctx;
}) {
  const hasKids = () => props.node.children.length > 0;
  const status = () => props.status;
  const selected = () => props.ctx.state.selected === props.node.key;
  const isFeature = () => props.node.kind === "feature";
  const match = createMemo(() => matches(props.node, props.ctx.state.filter));
  const flaky = () => props.flaky;
  const label = () => labelOf(props.node);
  const kindIcon = () =>
    isFeature()
      ? FileText
      : props.node.kind === "folder"
        ? props.open
          ? FolderOpen
          : Folder
        : null;
  const title = () => (isFeature() && props.node.name !== label() ? props.node.name : null);
  // Cut: the label shows less than it holds. Measured when the tooltip would show, so it follows the window.
  let labelEl: HTMLButtonElement | undefined;
  const cut = () => !!labelEl && labelEl.scrollWidth > labelEl.clientWidth;
  const full = () => (title() ? `${label()} · ${title()}` : label());
  // Getters, not a ternary in the prop: the compiler makes that a memo, which the tooltip's pinned
  // effect may not create.
  const tip = () => (cut() ? full() : undefined);
  const pinned = () => props.keyboardFocused && cut();
  // The buttons are out of the tab order: the row is the tab stop, its keys reach them (R3.2).
  return (
    <div
      data-testid="tree-node"
      data-key={props.node.key}
      data-status={status()}
      data-kind={props.node.kind}
      data-match={match() ? "true" : "false"}
      // A non-matching row is muted, not faded (R15.2): its status mark too, which sets its own colour.
      class={`group flex h-6 items-center gap-1 rounded-sm pr-1 ${selected() ? "bg-selected shadow-[inset_2px_0_0_var(--color-accent)]" : "hover:bg-hover"} ${match() ? "" : "text-muted [&_[data-status]]:text-muted"}`}
      style={{ "margin-left": `${(props.level - 1) * 16}px` }}
    >
      <button
        type="button"
        tabindex={-1}
        class={`inline-flex size-6 shrink-0 items-center justify-center rounded-sm text-muted ${INSET} ${hasKids() ? "" : "invisible"}`}
        aria-label={`${props.open ? "Collapse" : "Expand"} ${label()}`}
        onClick={() => props.toggle()}
      >
        {/* A swap, not a rotate: a transform on every open row makes each hit test undo it. */}
        <Icon icon={props.open ? ChevronDown : ChevronRight} size={14} />
      </button>
      <Tooltip text={STATUS_LABEL[status()]} side="right">
        <StatusMark status={status()} />
      </Tooltip>
      <Show when={kindIcon()}>
        <Icon icon={kindIcon()!} size={14} class="text-muted" />
      </Show>
      {/* A keyboard user reads a cut label as a mouse user does (R4.10). */}
      <Tooltip text={tip()} class="min-w-0 flex-1" side="over" pinned={pinned()}>
        <button
          ref={(e) => (labelEl = e)}
          type="button"
          tabindex={-1}
          class={`h-6 min-w-0 flex-1 truncate rounded-sm text-left text-sm ${INSET}`}
          onClick={() => props.ctx.select(props.node.key)}
        >
          {label()}
          <Show when={title()}>
            <span class="ml-2 text-xs text-muted">{title()}</span>
          </Show>
        </button>
      </Tooltip>
      <For each={props.node.tags}>{(tag) => <span class={TAG}>{tag}</span>}</For>
      <Show when={flaky()}>
        <Show
          when={isTest(props.node)}
          fallback={
            <span data-testid="flaky-count" class={FLAKY}>
              <Icon icon={Shuffle} size={14} />
              {flaky()}
            </span>
          }
        >
          <Tooltip text={FLAKY_TITLE} class="shrink-0" side="left">
            <span
              data-testid="flaky-mark"
              role="img" // aria-label names only an element with a role
              aria-label={FLAKY_TITLE}
              class={FLAKY}
            >
              <Icon icon={Shuffle} size={14} />
              flaky
            </span>
          </Tooltip>
        </Show>
      </Show>
      <Show when={match()}>
        <Tooltip text={`Run ${label()} (Ctrl+Enter)`} class="shrink-0" side="left">
          <button
            type="button"
            tabindex={-1}
            data-testid="run-node"
            aria-label={`Run ${label()}`}
            class={`${RUN} ${selected() ? "text-passed" : "text-muted"}`}
            disabled={props.ctx.state.run.active || props.ctx.state.busy}
            onClick={() => void props.ctx.api.run(props.node.key)}
          >
            <Icon icon={Play} size={14} />
          </button>
        </Tooltip>
      </Show>
    </div>
  );
}

export function Tree(props: Ctx) {
  const loading = () => !props.state.project || !props.state.tree;
  const root = createMemo(() => props.state.project?.root);
  // The closed rows by key: a refresh keeps them, another project starts with everything open (R2.2).
  const [closed, setClosed] = createSignal<ReadonlySet<string>>(() => {
    root();
    return new Set<string>();
  });
  const toggle = (key: string) =>
    setClosed((c) => {
      const next = new Set(c);
      if (!next.delete(key)) next.add(key);
      return next;
    });
  // Reads no status, so a run never redraws a row (R2.3).
  const rows = createMemo(() =>
    props.state.tree ? treeRows(props.state.tree, closed(), props.state.filter) : [],
  );
  let list: VirtualListHandle | undefined;

  // Right and Left move to a child or the parent as the arrows move (R3.3).
  const onKey = (e: KeyboardEvent, row: TreeRow, index: number) => {
    const { key } = row.node;
    if (e.ctrlKey && e.key === "Enter") {
      // as the run button does: nothing where it is missing or disabled (R3.3)
      const busy = props.state.run.active || props.state.busy;
      if (!busy && matches(row.node, props.state.filter)) void props.api.run(key);
      return true;
    }
    if (e.ctrlKey || e.metaKey || e.altKey) return false;
    if (e.key !== "ArrowRight" && e.key !== "ArrowLeft") return false;
    const kids = row.node.children.length > 0;
    const open = kids && !closed().has(key);
    if (e.key === "ArrowRight") {
      const child = rows()[index + 1]; // an open row's first child follows it
      if (kids && !open) toggle(key);
      else if (child?.parentKey === key) list?.moveTo(child.node.key);
    } else if (open) toggle(key);
    else if (row.parentKey !== null) list?.moveTo(row.parentKey);
    return true;
  };

  return (
    <Switch
      fallback={
        <VirtualList
          items={rows()}
          keyOf={(r) => r.node.key}
          rowHeight={24}
          pad={8}
          role="tree"
          itemRole="treeitem"
          label="Tests"
          testid="tree"
          itemClass={ITEM}
          // Each walks the row's subtree: once per change for the row and its name, not on every attrs run.
          shared={(r) => ({
            status: createMemo(() => deriveStatus(r().node, props.state.statuses)),
            flaky: createMemo(() => flakyChip(r().node, props.state.flaky)),
          })}
          attrs={(r, _index, row) => {
            const n = r.node;
            return {
              "aria-level": r.level,
              "aria-setsize": r.setSize,
              "aria-posinset": r.posInSet,
              "aria-expanded":
                n.children.length > 0 ? (closed().has(n.key) ? "false" : "true") : undefined,
              "aria-selected": props.state.selected === n.key ? "true" : "false",
              "aria-label": rowName(labelOf(n), row.status(), row.flaky(), n.tags),
              "aria-keyshortcuts": "Control+Enter",
            };
          }}
          selectedKey={props.state.selected}
          onSelect={(r) => props.select(r.node.key)}
          onKey={onKey}
          // By value: main sends the filter in force again after every refresh, which must not scroll (R4.7).
          revealSelectedOn={JSON.stringify(props.state.filter)}
          ref={(h) => (list = h)}
        >
          {(r, _index, keyboardFocused, row) => (
            <Row
              node={r.node}
              level={r.level}
              open={!closed().has(r.node.key)}
              toggle={() => toggle(r.node.key)}
              keyboardFocused={keyboardFocused()}
              status={row.status()}
              flaky={row.flaky()}
              ctx={props}
            />
          )}
        </VirtualList>
      }
    >
      <Match when={props.state.error}>
        <div class={MESSAGE}>
          <Notice kind="error" text={props.state.error!} testid="tree-message" />
        </div>
      </Match>
      <Match when={loading()}>
        <div class={MESSAGE}>
          <div class="flex items-center gap-2 p-2 text-sm text-muted">
            <Icon icon={LoaderCircle} class="animate-spin" />
            <p data-testid="tree-message">Finding tests…</p>
          </div>
        </div>
      </Match>
      <Match when={leaves(props.state.tree!).length === 0}>
        <div class={MESSAGE}>
          <EmptyState icon={FileSearch} text="No feature files found" testid="tree-message">
            <p class="text-xs text-muted">
              Grace looks for .feature files under the project folder.
            </p>
          </EmptyState>
        </div>
      </Match>
    </Switch>
  );
}
