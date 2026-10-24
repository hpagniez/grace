import { createSignal, For, Show } from "solid-js";
import type { Api } from "../shared/ipc";
import type { TestNode } from "../shared/model";
import { deriveStatus } from "../shared/status";
import { leaves, STATUS_DOT, STATUS_LABEL } from "./nodes";
import type { AppState } from "./reduce";

interface Ctx {
  state: AppState;
  select(key: string): void;
  api: Api;
}

function Node(props: { node: TestNode; ctx: Ctx }) {
  const [open, setOpen] = createSignal(true);
  const hasKids = () => props.node.children.length > 0;
  const status = () => deriveStatus(props.node, props.ctx.state.statuses);
  const selected = () => props.ctx.state.selected === props.node.key;
  return (
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
        class={`flex items-center gap-1 rounded px-1 py-0.5 hover:bg-zinc-100 dark:hover:bg-zinc-800 ${selected() ? "bg-sky-100 dark:bg-sky-900/40" : ""}`}
      >
        <button
          type="button"
          class="w-4 shrink-0 text-xs text-zinc-500"
          aria-label={`${open() ? "Collapse" : "Expand"} ${props.node.name}`}
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
          {props.node.name}
        </button>
        <For each={props.node.tags}>
          {(tag) => (
            <span class="shrink-0 rounded bg-zinc-200 px-1 text-[10px] dark:bg-zinc-700">
              {tag}
            </span>
          )}
        </For>
        <button
          type="button"
          data-testid="run-node"
          aria-label={`Run ${props.node.name}`}
          class="shrink-0 rounded px-1.5 text-xs text-green-700 hover:bg-green-100 disabled:cursor-not-allowed disabled:opacity-30 dark:text-green-400 dark:hover:bg-green-900/40"
          disabled={props.ctx.state.run.active}
          onClick={() => void props.ctx.api.run(props.node.key)}
        >
          ▶
        </button>
      </div>
      <Show when={hasKids() && open()}>
        <ul role="group" class="pl-4">
          <For each={props.node.children}>{(child) => <Node node={child} ctx={props.ctx} />}</For>
        </ul>
      </Show>
    </li>
  );
}

export function Tree(props: Ctx) {
  const message = () => {
    const { error, project, tree } = props.state;
    if (error) return error;
    if (!project || !tree) return "Open a Karate project folder";
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
