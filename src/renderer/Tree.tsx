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
import type { TestNode } from "../shared/model";
import { deriveStatus } from "../shared/status";
import { filterOn, matchingLeaves } from "../shared/tags";
import { EmptyState } from "./EmptyState";
import { Icon } from "./Icon";
import { leaves, nodeName, STATUS_LABEL } from "./nodes";
import { Notice } from "./Notice";
import type { AppState } from "./reduce";
import { StatusMark } from "./StatusMark";
import { Tooltip } from "./Tooltip";
import { badge, tagChip, TINT } from "./ui";

interface Ctx {
  state: AppState;
  select(key: string): void;
  api: Api;
}

// The tree scrolls, so a focus ring goes inside its row (spec R10.1). A row's tooltips stay on the row
// (beside their target, or over the cut label): one below would take the click meant for the next row.
const INSET = "focus-visible:-outline-offset-2";
const TAG = `${tagChip} shrink-0`;
const FLAKY = `${badge} ${TINT.cancelled} shrink-0`;
// 24 x 24 (R10.3). Muted until its row is hovered, focused or selected (the row sets `group`).
const RUN = `inline-flex size-6 shrink-0 items-center justify-center rounded-sm transition-colors duration-(--duration-fast) ease-standard enabled:group-hover:text-passed enabled:group-focus-within:text-passed disabled:cursor-not-allowed disabled:opacity-40 ${INSET}`;

function Node(props: { node: TestNode; ctx: Ctx }) {
  const [open, setOpen] = createSignal(true);
  const hasKids = () => props.node.children.length > 0;
  const status = createMemo(() => deriveStatus(props.node, props.ctx.state.statuses));
  const selected = () => props.ctx.state.selected === props.node.key;
  // Users navigate by file: a feature shows its file name; its Feature title follows, muted.
  const isFeature = () => props.node.kind === "feature";
  const filter = () => props.ctx.state.filter;
  // A folder, feature or outline matches when a test under it does; with the filter off all match.
  const match = createMemo(
    () => !filterOn(filter()) || matchingLeaves(props.node, filter()).length > 0,
  );
  const hidden = () => !match() && filter().hideNonMatching;
  // Counts the tests the tag filter hides too: flakiness is not about tags.
  const flakyCount = createMemo(
    () => leaves(props.node).filter((l) => l.key in props.ctx.state.flaky).length,
  );
  const isTest = () => props.node.kind === "scenario" || props.node.kind === "row";
  const label = () => (isFeature() ? props.node.file.split("/").pop()! : nodeName(props.node));
  const kindIcon = () =>
    isFeature() ? FileText : props.node.kind === "folder" ? (open() ? FolderOpen : Folder) : null;
  const title = () => (isFeature() && props.node.name !== label() ? props.node.name : null);
  // Cut: the label shows less than it holds. Measured when the tooltip would show, so it follows the window.
  let labelEl!: HTMLButtonElement;
  const cut = () => labelEl.scrollWidth > labelEl.clientWidth;
  const full = () => (title() ? `${label()} · ${title()}` : label());
  return (
    <Show when={!hidden()}>
      <li
        role="treeitem"
        aria-expanded={hasKids() ? (open() ? "true" : "false") : undefined}
        aria-selected={selected() ? "true" : "false"}
      >
        <div
          data-testid="tree-node"
          data-key={props.node.key}
          data-status={status()}
          data-kind={props.node.kind}
          data-match={match() ? "true" : "false"}
          // A non-matching row is muted, not faded (R15.2): its status mark too, which sets its own colour.
          // content-visibility: off-screen rows skip paint, so a hover transition or a status change repaints only what shows.
          class={`group flex h-6 items-center gap-1 rounded-sm pr-1 [content-visibility:auto] [contain-intrinsic-size:auto_1.5rem] ${selected() ? "bg-selected shadow-[inset_2px_0_0_var(--color-accent)]" : "hover:bg-hover"} ${match() ? "" : "text-muted [&_[data-status]]:text-muted"}`}
        >
          <button
            type="button"
            class={`inline-flex size-6 shrink-0 items-center justify-center rounded-sm text-muted ${INSET} ${hasKids() ? "" : "invisible"}`}
            aria-label={`${open() ? "Collapse" : "Expand"} ${label()}`}
            onClick={() => setOpen(!open())}
          >
            {/* A swap, not a rotate: a transform on every open row makes each hit test undo it. */}
            <Icon icon={open() ? ChevronDown : ChevronRight} size={14} />
          </button>
          <Tooltip text={STATUS_LABEL[status()]} side="right">
            <StatusMark status={status()} />
          </Tooltip>
          <Show when={kindIcon()}>
            <Icon icon={kindIcon()!} size={14} class="text-muted" />
          </Show>
          <Tooltip text={cut() ? full() : undefined} class="min-w-0 flex-1" side="over">
            <button
              ref={(e) => (labelEl = e)}
              type="button"
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
          <Show when={flakyCount() > 0}>
            <Show
              when={isTest()}
              fallback={
                <span data-testid="flaky-count" class={FLAKY}>
                  <Icon icon={Shuffle} size={14} />
                  {`${flakyCount()} flaky`}
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
            <Tooltip text={`Run ${label()}`} class="shrink-0" side="left">
              <button
                type="button"
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
        <Show when={hasKids() && open()}>
          <ul role="group" class="pl-4">
            <For each={props.node.children}>{(child) => <Node node={child} ctx={props.ctx} />}</For>
          </ul>
        </Show>
      </li>
    </Show>
  );
}

export function Tree(props: Ctx) {
  const loading = () => !props.state.project || !props.state.tree;
  return (
    <div class="min-h-0 flex-1 overflow-auto p-2">
      <Switch
        fallback={
          <ul role="tree" data-testid="tree">
            <For each={props.state.tree!.children}>{(n) => <Node node={n} ctx={props} />}</For>
          </ul>
        }
      >
        <Match when={props.state.error}>
          <Notice kind="error" text={props.state.error!} testid="tree-message" />
        </Match>
        <Match when={loading()}>
          <div class="flex items-center gap-2 p-2 text-sm text-muted">
            <Icon icon={LoaderCircle} class="animate-spin" />
            <p data-testid="tree-message">Finding tests…</p>
          </div>
        </Match>
        <Match when={leaves(props.state.tree!).length === 0}>
          <EmptyState icon={FileSearch} text="No feature files found" testid="tree-message">
            <p class="text-xs text-muted">
              Grace looks for .feature files under the project folder.
            </p>
          </EmptyState>
        </Match>
      </Switch>
    </div>
  );
}
