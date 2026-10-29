import { ChevronRight, CircleAlert, LoaderCircle, MousePointerClick } from "lucide";
import { createSignal, For, Show } from "solid-js";
import type { Step } from "../shared/events";
import type { Api } from "../shared/ipc";
import type { Status, TestNode } from "../shared/model";
import { deriveStatus } from "../shared/status";
import { ghostSmall } from "./buttons";
import { CallDetails } from "./CallDetails";
import { Cut } from "./Cut";
import { EmptyState } from "./EmptyState";
import { FlakyProof } from "./FlakyProof";
import { findCall } from "./http";
import { Icon } from "./Icon";
import { findNode, formatMillis, leaves, nodeName, STATUS_LABEL } from "./nodes";
import type { AppState } from "./reduce";
import { MARK, StatusMark, type MarkStatus } from "./StatusMark";
import { Tooltip } from "./Tooltip";
import { badge, box, tagChip, TINT } from "./ui";

/** The tint a test's status takes in its badge: not run and pending are the grey of a skipped step. */
const BADGE: Record<Status, keyof typeof TINT> = {
  notRun: "skipped",
  pending: "skipped",
  running: "running",
  passed: "passed",
  failed: "failed",
  interrupted: "cancelled",
};

/** The status as a badge: its icon and its word, which names it (R7.1), so the icon is decorative. */
function StatusBadge(props: { status: Status }) {
  return (
    <span data-testid="details-status" class={`${badge} ${TINT[BADGE[props.status]]}`}>
      <StatusMark status={props.status} decorative />
      {STATUS_LABEL[props.status]}
    </span>
  );
}

/** A step status Karate has not shown us (the fixtures hold passed, failed and skipped) takes the amber mark. */
const stepMark = (status: string): MarkStatus =>
  Object.hasOwn(MARK, status) ? (status as MarkStatus) : "interrupted";

function StepRow(props: { step: Step }) {
  const [open, setOpen] = createSignal(false);
  const mark = () => stepMark(props.step.status);
  return (
    <li data-testid="step" data-status={props.step.status} class="py-1">
      <div class="flex min-h-6 items-center gap-2 text-sm">
        <Tooltip text={MARK[mark()].label}>
          <StatusMark status={mark()} />
        </Tooltip>
        <Show when={props.step.background}>
          <span class={`${tagChip} shrink-0`}>Background</span>
        </Show>
        <Cut full={`${props.step.keyword} ${props.step.text}`} wrap="flex-1">
          <span class="font-medium">{props.step.keyword}</span>{" "}
          <span class="font-mono text-xs">{props.step.text}</span>
        </Cut>
        <span class="shrink-0 font-mono text-xs text-muted tabular-nums">
          {formatMillis(props.step.millis)}
        </span>
        <Show when={props.step.log}>
          <button
            type="button"
            data-testid="step-toggle"
            class={`${ghostSmall} shrink-0`}
            aria-expanded={open() ? "true" : "false"}
            aria-label={`${open() ? "Hide" : "Show"} log of ${props.step.text}`}
            onClick={() => setOpen(!open())}
          >
            <Icon
              icon={ChevronRight}
              size={14}
              class={`transition-transform duration-(--duration-base) ease-standard ${open() ? "rotate-90" : ""}`}
            />
            Log
          </button>
        </Show>
      </div>
      <Show when={open() && props.step.log}>
        <pre data-testid="step-log" class={`${box} mt-1 ml-6 max-h-60 overflow-auto`}>
          {props.step.log}
        </pre>
      </Show>
    </li>
  );
}

function Leaf(props: { node: TestNode; state: AppState; api: Api }) {
  const status = () => props.state.statuses[props.node.key] ?? "notRun";
  const result = () => props.state.results[props.node.key];
  const steps = () => props.state.steps[props.node.key];
  const location = () =>
    props.node.line === null ? props.node.file : `${props.node.file}:${props.node.line}`;
  return (
    <>
      <h2 class="text-lg font-semibold break-words">{nodeName(props.node)}</h2>
      <p>
        <StatusBadge status={status()} />
      </p>
      <div class="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted">
        <Cut full={location()} testid="details-location" class="font-mono" wrap="max-w-full">
          {location()}
        </Cut>
        <Show when={result()?.durationMillis !== undefined}>
          <span data-testid="details-duration" class="font-mono tabular-nums">
            {formatMillis(result()!.durationMillis!)}
          </span>
        </Show>
        <For each={props.node.tags}>{(t) => <span class={tagChip}>{t}</span>}</For>
      </div>
      <Show when={props.state.flaky[props.node.key]}>
        {(envs) => (
          <FlakyProof
            envs={envs()}
            current={props.state.env?.env ?? null}
            locked={props.state.run.active || props.state.busy}
            api={props.api}
          />
        )}
      </Show>
      <Show when={status() === "failed" && result()?.error}>
        <div class={`rounded-lg p-3 ${TINT.failed}`}>
          <p class="flex items-center gap-1.5 text-sm font-semibold">
            <Icon icon={CircleAlert} size={14} />
            Error
          </p>
          <pre
            data-testid="details-error"
            class="mt-1 font-mono text-xs break-words whitespace-pre-wrap text-ink"
          >
            {result()!.error}
          </pre>
        </div>
      </Show>
      <Show
        when={steps()}
        fallback={
          <p class="flex items-center gap-2 text-sm text-muted">
            <Show when={props.state.run.active && status() !== "notRun"}>
              <Icon icon={LoaderCircle} class="animate-spin" />
            </Show>
            Steps are available when the feature finishes
          </p>
        }
      >
        <ul class="divide-y divide-line">
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
  const location = () => props.node.file || "/";
  return (
    <>
      <h2 class="text-lg font-semibold break-words">{props.node.name}</h2>
      <p>
        <StatusBadge status={deriveStatus(props.node, props.state.statuses)} />
      </p>
      <div class="flex text-xs text-muted">
        <Cut full={location()} testid="details-location" class="font-mono" wrap="max-w-full">
          {location()}
        </Cut>
      </div>
      <p class="flex flex-wrap items-center gap-x-4 gap-y-1 text-sm" data-testid="details-counts">
        <span>{`${leaves(props.node).length} scenarios`}</span>
        <For each={counts()}>
          {([s, n]) => (
            <span class="inline-flex items-center gap-1.5">
              <StatusMark status={s} decorative />
              <span class="tabular-nums">{n}</span> {STATUS_LABEL[s]}
            </span>
          )}
        </For>
      </p>
    </>
  );
}

export function Details(props: { state: AppState; api: Api }) {
  const node = () =>
    props.state.selected && props.state.tree
      ? findNode(props.state.tree, props.state.selected)
      : null;
  // The call replaces the node while the HTTP tab is up; findCall is null once the call is gone.
  const call = () => (props.state.panel === "http" ? findCall(props.state) : null);
  return (
    <aside
      data-testid="details"
      class="min-h-0 flex-1 space-y-3 overflow-auto border-l border-line p-4"
    >
      <Show
        when={call()}
        fallback={
          <Show
            when={node()}
            fallback={
              <EmptyState icon={MousePointerClick} text="Select a test to see its details" />
            }
          >
            {(n) => (
              <Show
                when={n().kind === "scenario" || n().kind === "row"}
                fallback={<Group node={n()} state={props.state} />}
              >
                <Leaf node={n()} state={props.state} api={props.api} />
              </Show>
            )}
          </Show>
        }
      >
        {(c) => <CallDetails listed={c()} />}
      </Show>
    </aside>
  );
}
