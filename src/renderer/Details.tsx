import { createSignal, For, Show } from "solid-js";
import type { Step } from "../shared/events";
import type { Status, TestNode } from "../shared/model";
import { deriveStatus } from "../shared/status";
import { findNode, formatMillis, leaves, nodeName, STATUS_DOT, STATUS_LABEL } from "./nodes";
import type { AppState } from "./reduce";

const STEP_DOT: Record<string, string> = {
  passed: "bg-green-500",
  failed: "bg-red-500",
  skipped: "bg-zinc-400",
};

function StepRow(props: { step: Step }) {
  const [open, setOpen] = createSignal(false);
  return (
    <li data-testid="step" data-status={props.step.status} class="py-0.5">
      <div class="flex items-center gap-2 text-sm">
        <span
          class={`size-2 shrink-0 rounded-full ${STEP_DOT[props.step.status] ?? "bg-amber-500"}`}
          title={props.step.status}
        />
        <Show when={props.step.background}>
          <span class="shrink-0 rounded bg-zinc-200 px-1 text-[10px] dark:bg-zinc-700">
            Background
          </span>
        </Show>
        <span class="min-w-0 flex-1 truncate font-mono">{`${props.step.keyword} ${props.step.text}`}</span>
        <span class="shrink-0 text-xs text-zinc-500">{formatMillis(props.step.millis)}</span>
        <Show when={props.step.log}>
          <button
            type="button"
            data-testid="step-toggle"
            class="shrink-0 rounded px-1 text-xs text-sky-700 hover:bg-zinc-100 dark:text-sky-400 dark:hover:bg-zinc-800"
            aria-expanded={open() ? "true" : "false"}
            aria-label={`${open() ? "Hide" : "Show"} log of ${props.step.text}`}
            onClick={() => setOpen(!open())}
          >
            {open() ? "Hide log" : "Log"}
          </button>
        </Show>
      </div>
      <Show when={open() && props.step.log}>
        <pre
          data-testid="step-log"
          class="mt-1 max-h-60 overflow-auto rounded bg-zinc-100 p-2 text-xs dark:bg-zinc-800"
        >
          {props.step.log}
        </pre>
      </Show>
    </li>
  );
}

function Leaf(props: { node: TestNode; state: AppState }) {
  const status = () => props.state.statuses[props.node.key] ?? "notRun";
  const result = () => props.state.results[props.node.key];
  const steps = () => props.state.steps[props.node.key];
  return (
    <>
      <h2 class="text-lg font-semibold">{nodeName(props.node)}</h2>
      <p class="flex items-center gap-2 text-sm">
        <span class={`size-2.5 rounded-full ${STATUS_DOT[status()]}`} />
        <span data-testid="details-status">{STATUS_LABEL[status()]}</span>
      </p>
      <p class="font-mono text-sm text-zinc-500" data-testid="details-location">
        {props.node.line === null ? props.node.file : `${props.node.file}:${props.node.line}`}
      </p>
      <Show when={result()?.durationMillis !== undefined}>
        <p class="text-sm" data-testid="details-duration">
          {formatMillis(result()!.durationMillis!)}
        </p>
      </Show>
      <Show when={props.node.tags.length > 0}>
        <p class="flex gap-1">
          <For each={props.node.tags}>
            {(t) => <span class="rounded bg-zinc-200 px-1 text-xs dark:bg-zinc-700">{t}</span>}
          </For>
        </p>
      </Show>
      <Show when={status() === "failed" && result()?.error}>
        <pre
          data-testid="details-error"
          class="whitespace-pre-wrap rounded border border-red-300 bg-red-50 p-2 text-xs text-red-800 dark:border-red-800 dark:bg-red-950 dark:text-red-200"
        >
          {result()!.error}
        </pre>
      </Show>
      <Show
        when={steps()}
        fallback={
          <p class="text-sm text-zinc-500">Steps are available when the feature finishes</p>
        }
      >
        <ul class="divide-y divide-zinc-100 dark:divide-zinc-800">
          <For each={steps()!}>{(step) => <StepRow step={step} />}</For>
        </ul>
      </Show>
    </>
  );
}

function Group(props: { node: TestNode; state: AppState }) {
  const counts = () => {
    const out: Partial<Record<Status, number>> = {};
    for (const l of leaves(props.node)) {
      const s = props.state.statuses[l.key] ?? "notRun";
      out[s] = (out[s] ?? 0) + 1;
    }
    return Object.entries(out) as [Status, number][];
  };
  return (
    <>
      <h2 class="text-lg font-semibold">{props.node.name}</h2>
      <p class="flex items-center gap-2 text-sm">
        <span
          class={`size-2.5 rounded-full ${STATUS_DOT[deriveStatus(props.node, props.state.statuses)]}`}
        />
        {STATUS_LABEL[deriveStatus(props.node, props.state.statuses)]}
      </p>
      <p class="font-mono text-sm text-zinc-500" data-testid="details-location">
        {props.node.file || "/"}
      </p>
      <p class="text-sm" data-testid="details-counts">
        {`${leaves(props.node).length} scenarios`}
        <For each={counts()}>{([s, n]) => ` · ${STATUS_LABEL[s]} ${n}`}</For>
      </p>
    </>
  );
}

export function Details(props: { state: AppState }) {
  const node = () =>
    props.state.selected && props.state.tree
      ? findNode(props.state.tree, props.state.selected)
      : null;
  return (
    <aside
      data-testid="details"
      class="min-h-0 flex-1 space-y-2 overflow-auto border-l border-zinc-200 p-3 dark:border-zinc-700"
    >
      <Show
        when={node()}
        fallback={<p class="text-sm text-zinc-500">Select a test to see its details</p>}
      >
        {(n) => (
          <Show
            when={n().kind === "scenario" || n().kind === "row"}
            fallback={<Group node={n()} state={props.state} />}
          >
            <Leaf node={n()} state={props.state} />
          </Show>
        )}
      </Show>
    </aside>
  );
}
