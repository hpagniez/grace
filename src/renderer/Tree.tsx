import {
  Bug,
  ChevronDown,
  ChevronRight,
  FileSearch,
  FileText,
  Folder,
  FolderOpen,
  LoaderCircle,
  Play,
  Shuffle,
  type IconNode,
} from "lucide";
import {
  createEffect,
  createMemo,
  createSignal,
  For,
  Match,
  Show,
  Switch,
  untrack,
} from "solid-js";
import { FLAKY_TITLE } from "../shared/flaky";
import type { Api } from "../shared/ipc";
import type { Status, TestNode } from "../shared/model";
import { deriveStatus } from "../shared/status";
import { filterOn, matchingLeaves, type FilterState } from "../shared/tags";
import { findingText } from "./bannerText";
import { btnSmall } from "./buttons";
import { EmptyState } from "./EmptyState";
import { Icon } from "./Icon";
import {
  ancestors,
  breakpointKeys,
  failedAt,
  featureOf,
  leaves,
  nodeName,
  STATUS_LABEL,
} from "./nodes";
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
const DEBUG = RUN.replaceAll("text-passed", "text-running");
// Bug's 11 strokes drawn as one path, the same lines: 10 fewer nodes on every row (page.nodes). Its one
// path that starts with a relative moveto gets it absolute, with the pairs after it still relative.
const BUG: IconNode = [
  ["path", { d: Bug.map(([, a]) => String(a.d).replace(/^m(\S+ \S+) /, "M$1 l")).join(" ") }],
];

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
/**
 * Where "Open in VS Code" opens a row (open-in-editor-from-tree R2): a failed test where Karate says it failed, any
 * other test or outline at its line, a feature at its top; null for a folder.
 */
const editorPlace = (n: TestNode, state: AppState) => {
  if (n.kind === "folder") return null;
  const place =
    isTest(n) && state.statuses[n.key] === "failed"
      ? failedAt(state.results[n.key]?.error ?? "")
      : null;
  return place ?? { file: n.file, line: n.line ?? 1 };
};

/** What a tree row draws; the list around it makes it a treeitem, named from the same status and flaky chip. */
function Row(props: {
  node: TestNode;
  open: boolean;
  toggle(): void;
  keyboardFocused: boolean;
  status: Status;
  flaky: string | null;
  /** A breakpoint is set on one of its lines (step debugger R5.2). */
  breakpoint: boolean;
  ctx: Ctx;
}) {
  const hasKids = () => props.node.children.length > 0;
  const status = () => props.status;
  // the test a debug run waits in shows the pause mark, still `running` underneath (R7.2)
  const mark = () => (props.ctx.state.paused?.key === props.node.key ? "paused" : status());
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
  // A getter, not `&&` in the prop: the compiler makes that a new memo on every read, and the tooltip
  // reads `pinned` on every scroll.
  const pinned = () => props.keyboardFocused && cut();
  // The Debug button only on the row in use: hovered, focused from the keyboard or selected (step debugger R5.1).
  // Every other row keeps its place empty, so nothing moves under the pointer; 8 fewer nodes a row (page.nodes).
  const [hovered, setHovered] = createSignal(false);
  const debugShown = () => hovered() || props.keyboardFocused || selected();
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
      onPointerEnter={() => setHovered(true)}
      onPointerLeave={() => setHovered(false)}
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
      {/* One insert, not one per child: each dynamic child beside another leaves a comment node in the DOM (page.nodes). */}
      {[
        <Tooltip text={mark() === "paused" ? "Paused" : STATUS_LABEL[status()]} side="right">
          <StatusMark status={mark()} />
        </Tooltip>,
        <Show when={kindIcon()}>
          <Icon icon={kindIcon()!} size={14} class="text-muted" />
        </Show>,
        // A keyboard user reads a cut label as a mouse user does (R4.10).
        <Tooltip
          text={cut() ? full() : undefined}
          class="min-w-0 flex-1"
          side="over"
          pinned={pinned()}
        >
          <button
            ref={(e) => (labelEl = e)}
            type="button"
            tabindex={-1}
            class={`h-6 min-w-0 flex-1 truncate rounded-sm text-left text-sm ${INSET}`}
            onClick={() => {
              props.ctx.select(props.node.key);
              if (isFeature() && !props.open) props.toggle(); // a click opens a feature, never closes it (#224 R2.1)
            }}
          >
            {[
              label(),
              <Show when={title()}>
                <span class="ml-2 text-xs text-muted">{title()}</span>
              </Show>,
            ]}
          </button>
        </Tooltip>,
        <Show when={props.breakpoint}>
          <span
            data-testid="breakpoint-dot"
            role="img"
            aria-label="Has breakpoints"
            class="size-2 shrink-0 rounded-full bg-failed"
          />
        </Show>,
        <For each={props.node.tags}>{(tag) => <span class={TAG}>{tag}</span>}</For>,
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
        </Show>,
        <Show when={match()}>
          <Show when={debugShown()} fallback={<span class="size-6 shrink-0" />}>
            <Tooltip text={`Debug ${label()} (Ctrl+Shift+Enter)`} class="shrink-0" side="left">
              <button
                type="button"
                tabindex={-1}
                data-testid="debug-node"
                aria-label={`Debug ${label()}`}
                class={`${DEBUG} ${selected() ? "text-running" : "text-muted"}`}
                disabled={props.ctx.state.run.active || props.ctx.state.busy}
                onClick={() => void props.ctx.api.debug(props.node.key)}
              >
                <Icon icon={BUG} size={14} />
              </button>
            </Tooltip>
          </Show>
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
        </Show>,
      ]}
    </div>
  );
}

export function Tree(props: Ctx) {
  const loading = () => !props.state.project || !props.state.tree;
  const root = createMemo(() => props.state.project?.root);
  // The rows turned from where they start, by key: a feature starts closed, every other row open. A refresh keeps
  // them; another project starts afresh (#224 R1).
  const [turned, setTurned] = createSignal<ReadonlySet<string>>(() => {
    root();
    return new Set<string>();
  });
  const startsOpen = (key: string) => !key.startsWith("f:");
  const isOpen = (key: string) => startsOpen(key) !== turned().has(key);
  const setOpen = (key: string, open: boolean) =>
    setTurned((t) => {
      const next = new Set(t);
      if (open === startsOpen(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  // The run in progress, or the last: the features it may no longer open, and how many failed keys it has read.
  let run = { id: null as string | null, done: new Set<string>(), read: 0 };
  const toggle = (key: string) => {
    // a feature the person closes during a run stays closed until it ends (#224 R3.5)
    if (isOpen(key) && props.state.run.active) run.done.add(key);
    setOpen(key, !isOpen(key));
  };
  // Reads no status, so a status update never redraws a row (R2.3). A row that stays as it was keeps its object, so
  // a feature that opens draws only its own rows: a run opens features while it goes (#224 R4.3).
  let built = new Map<string, TreeRow>();
  const rows = createMemo(() => {
    const next = props.state.tree ? treeRows(props.state.tree, isOpen, props.state.filter) : [];
    const kept = next.map((r) => {
      const old = built.get(r.node.key);
      return old &&
        old.node === r.node &&
        old.level === r.level &&
        old.setSize === r.setSize &&
        old.posInSet === r.posInSet &&
        old.parentKey === r.parentKey
        ? old
        : r;
    });
    built = new Map(kept.map((r) => [r.node.key, r]));
    return kept;
  });
  let list: VirtualListHandle | undefined;
  const withBreakpoints = createMemo(() =>
    breakpointKeys(props.state.tree, props.state.breakpoints),
  );
  // A pause, or a step through a group in Details (#229 R4.3), opens the rows above the test; the list then
  // scrolls to it, the selected row (R7.1). The cue changes with the rows it opens, not with the pause: the list
  // would look for the row before it is there.
  const [shown, setShown] = createSignal(0);
  const show = (key: string | null) => {
    const tree = props.state.tree;
    if (key === null || !tree) return;
    for (const a of ancestors(tree, key)) if (!isOpen(a)) setOpen(a, true);
    setShown((n) => n + 1);
  };
  createEffect(
    () => [props.state.paused?.key ?? null, props.state.run.pauses] as const,
    ([key]) => show(key),
  );
  createEffect(
    () => props.state.reveals,
    () => show(untrack(() => props.state.selected)),
  );
  // A run opens the feature it started in, and each feature where one of its tests fails, once (#224 R3). The rows
  // still read no status: they change only when a feature opens (R4.3).
  createEffect(
    () => [props.state.run.runId, props.state.run.failedKeys.length] as const,
    ([id, failed]) => {
      if (id === null) return;
      const open = (feature: string | null) => {
        if (feature === null || run.done.has(feature)) return;
        run.done.add(feature);
        if (!isOpen(feature)) setOpen(feature, true);
      };
      if (id !== run.id) {
        run = { id, done: new Set(), read: 0 };
        open(featureOf(props.state.run.selection));
      }
      for (; run.read < failed; run.read++) open(featureOf(props.state.run.failedKeys[run.read]));
    },
  );

  // Right and Left move to a child or the parent as the arrows move (R3.3).
  const onKey = (e: KeyboardEvent, row: TreeRow, index: number) => {
    const { key } = row.node;
    if (e.ctrlKey && e.key === "Enter") {
      // as the run and debug buttons do: nothing where they are missing or disabled (R3.3)
      const busy = props.state.run.active || props.state.busy;
      if (!busy && matches(row.node, props.state.filter))
        void (e.shiftKey ? props.api.debug(key) : props.api.run(key));
      return true;
    }
    if (e.ctrlKey || e.metaKey || e.altKey) return false;
    if (e.key !== "ArrowRight" && e.key !== "ArrowLeft") return false;
    const kids = row.node.children.length > 0;
    const open = kids && isOpen(key);
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
          indent={(row) => (row.level - 1) * 16}
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
                n.children.length > 0 ? (isOpen(n.key) ? "true" : "false") : undefined,
              "aria-selected": props.state.selected === n.key ? "true" : "false",
              "aria-label": rowName(labelOf(n), row.status(), row.flaky(), n.tags),
              "aria-keyshortcuts": "Control+Enter Control+Shift+Enter",
            };
          }}
          selectedKey={props.state.selected}
          onSelect={(r) => props.select(r.node.key)}
          onKey={onKey}
          onContextMenu={(e, r) => {
            const place = editorPlace(r.node, props.state);
            if (!place) return;
            e.preventDefault();
            void props.api.editorMenu(place.file, place.line, e.clientX, e.clientY);
          }}
          // By value: main sends the filter in force again after every refresh, which must not scroll (R4.7).
          revealSelectedOn={`${JSON.stringify(props.state.filter)} ${shown()}`}
          ref={(h) => (list = h)}
        >
          {(r, _index, keyboardFocused, row) => (
            <Row
              node={r.node}
              open={isOpen(r.node.key)}
              toggle={() => toggle(r.node.key)}
              keyboardFocused={keyboardFocused()}
              status={row.status()}
              flaky={row.flaky()}
              breakpoint={withBreakpoints().has(r.node.key)}
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
            <p data-testid="tree-message">{findingText(props.state)}</p>
          </div>
        </div>
      </Match>
      <Match when={leaves(props.state.tree!).length === 0}>
        <div class={MESSAGE}>
          <EmptyState icon={FileSearch} text="No feature files found" testid="tree-message">
            <p class="text-xs text-muted">
              Grace looks for .feature files under the project folder.
            </p>
            <Show when={props.state.subfolders.length > 0}>
              <div class="flex flex-wrap justify-center gap-2">
                <For each={props.state.subfolders}>
                  {(folder) => (
                    <button
                      type="button"
                      class={btnSmall}
                      disabled={props.state.busy}
                      onClick={() => void props.api.openSubfolder(folder)}
                    >
                      Open {folder}/
                    </button>
                  )}
                </For>
              </div>
            </Show>
          </EmptyState>
        </div>
      </Match>
    </Switch>
  );
}
