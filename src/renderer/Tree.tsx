import { createMemo, createSignal, For, Show } from "solid-js";
import { FLAKY_TITLE } from "../shared/flaky";
import type { Api } from "../shared/ipc";
import type { TestNode } from "../shared/model";
import { deriveStatus } from "../shared/status";
import { filterOn, matchingLeaves } from "../shared/tags";
import { FLAKY_TONE, leaves, nodeName, STATUS_DOT, STATUS_LABEL } from "./nodes";
import type { AppState } from "./reduce";

interface Ctx {
  state: AppState;
  select(key: string): void;
  api: Api;
}

const FLAKY_CHIP = `shrink-0 rounded border px-1 text-xs ${FLAKY_TONE}`;

function Node(props: { node: TestNode; ctx: Ctx }) {
  const [open, setOpen] = createSignal(true);
  const hasKids = () => props.node.children.length > 0;
  const status = () => deriveStatus(props.node, props.ctx.state.statuses);
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
          class={`flex items-center gap-1 rounded px-1 py-0.5 hover:bg-zinc-100 dark:hover:bg-zinc-800 ${selected() ? "bg-sky-100 dark:bg-sky-900/40" : ""} ${match() ? "" : "opacity-40"}`}
        >
          <button
            type="button"
            class="w-4 shrink-0 text-xs text-zinc-500"
            aria-label={`${open() ? "Collapse" : "Expand"} ${label()}`}
            tabindex={hasKids() ? 0 : -1}
            style={hasKids() ? "" : "visibility:hidden"}
            onClick={() => setOpen(!open())}
          >
            {open() ? "▾" : "▸"}
          </button>
          <span
            class={`size-2.5 shrink-0 rounded-full ${STATUS_DOT[status()]}`}
            title={STATUS_LABEL[status()]}
          />
          <button
            type="button"
            class="min-w-0 flex-1 truncate text-left text-sm"
            onClick={() => props.ctx.select(props.node.key)}
          >
            {label()}
            <Show when={isFeature() && props.node.name !== label()}>
              <span class="ml-2 text-xs text-zinc-500 dark:text-zinc-400">{props.node.name}</span>
            </Show>
          </button>
          <For each={props.node.tags}>
            {(tag) => (
              <span class="shrink-0 rounded bg-zinc-200 px-1 text-[10px] dark:bg-zinc-700">
                {tag}
              </span>
            )}
          </For>
          <Show when={flakyCount() > 0}>
            <Show
              when={isTest()}
              fallback={
                <span data-testid="flaky-count" class={FLAKY_CHIP}>
                  {`${flakyCount()} flaky`}
                </span>
              }
            >
              <span
                data-testid="flaky-mark"
                role="img" // aria-label names only an element with a role
                title={FLAKY_TITLE}
                aria-label={FLAKY_TITLE}
                class={FLAKY_CHIP}
              >
                flaky
              </span>
            </Show>
          </Show>
          <Show when={match()}>
            <button
              type="button"
              data-testid="run-node"
              aria-label={`Run ${label()}`}
              class="shrink-0 rounded px-1.5 text-xs text-green-700 hover:bg-green-100 disabled:cursor-not-allowed disabled:opacity-30 dark:text-green-400 dark:hover:bg-green-900/40"
              disabled={props.ctx.state.run.active || props.ctx.state.busy}
              onClick={() => void props.ctx.api.run(props.node.key)}
            >
              ▶
            </button>
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
  const message = () => {
    const { error, project, tree } = props.state;
    if (error) return error;
    if (!project || !tree) return "Finding tests…";
    if (leaves(tree).length === 0) return "No feature files found";
    return null;
  };
  return (
    <div class="min-h-0 flex-1 overflow-auto p-2">
      <Show
        when={message() === null}
        fallback={
          <p data-testid="tree-message" class="p-2 text-sm text-zinc-500">
            {message()}
          </p>
        }
      >
        <ul role="tree" data-testid="tree">
          <For each={props.state.tree!.children}>{(n) => <Node node={n} ctx={props} />}</For>
        </ul>
      </Show>
    </div>
  );
}
