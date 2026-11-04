import { ChevronRight, CircleAlert, ExternalLink, LoaderCircle, MousePointerClick } from "lucide";
import { createEffect, createMemo, createSignal, flush, For, Show } from "solid-js";
import type { FileStep, Step } from "../shared/events";
import type { Api } from "../shared/ipc";
import type { Status, TestNode } from "../shared/model";
import { deriveStatus } from "../shared/status";
import { ghostSmall } from "./buttons";
import { CallDetails } from "./CallDetails";
import { Cut } from "./Cut";
import { EmptyState } from "./EmptyState";
import { Examples } from "./Examples";
import { FlakyProof } from "./FlakyProof";
import { findCall } from "./http";
import { Icon } from "./Icon";
import { failedAt, findNode, formatMillis, leaves, nodeName, STATUS_LABEL } from "./nodes";
import type { AppState } from "./reduce";
import { MARK, StatusMark, type MarkStatus } from "./StatusMark";
import { Tooltip } from "./Tooltip";
import { badge, box, link, tagChip, TINT } from "./ui";

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

/** A step of the run, or one read from the feature file before any run (no status, time or log). */
type ShownStep = FileStep & Partial<Pick<Step, "status" | "millis" | "log">>;

function StepRow(props: { step: ShownStep; on: boolean; toggle: () => void; paused: boolean }) {
  const [open, setOpen] = createSignal(false);
  const breakpoint = () => `${props.on ? "Remove" : "Add"} breakpoint at line ${props.step.line}`;
  return (
    <li
      data-testid="step"
      data-status={props.step.status}
      data-paused={props.paused ? "" : undefined}
      class={props.paused ? `-mx-2 rounded-md px-2 py-1 ${TINT.running}` : "py-1"}
    >
      <Show when={props.step.comments}>
        {(comments) => (
          <div data-testid="step-comments" class="ml-6 font-mono text-xs break-words text-muted">
            <For each={comments()}>{(line) => <p>{line}</p>}</For>
          </div>
        )}
      </Show>
      <div class="flex min-h-6 items-center gap-2 text-sm">
        <Tooltip text={breakpoint()} side="right">
          <button
            type="button"
            data-testid="breakpoint"
            aria-label={breakpoint()}
            class="group flex size-4 shrink-0 items-center justify-center rounded-full"
            onClick={() => props.toggle()}
          >
            <span
              aria-hidden="true"
              class={`size-2.5 rounded-full border border-failed ${props.on ? "bg-failed" : "opacity-0 group-hover:opacity-100 group-focus-visible:opacity-100"}`}
            />
          </button>
        </Tooltip>
        <Show when={props.step.status}>
          {(status) => (
            <Tooltip text={MARK[stepMark(status())].label}>
              <StatusMark status={stepMark(status())} />
            </Tooltip>
          )}
        </Show>
        <Show when={props.step.background}>
          <span class={`${tagChip} shrink-0`}>Background</span>
        </Show>
        <Cut full={`${props.step.keyword} ${props.step.text}`} wrap="flex-1" side="over">
          <span class="font-medium">{props.step.keyword}</span>{" "}
          <span class="font-mono text-xs">{props.step.text}</span>
        </Cut>
        <Show when={props.step.millis !== undefined}>
          <span class="shrink-0 font-mono text-xs text-muted tabular-nums">
            {formatMillis(props.step.millis!)}
          </span>
        </Show>
        <Show when={props.paused}>
          <span class="shrink-0 text-xs font-medium">Paused</span>
        </Show>
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

/** A failed test's error; the place on its first line opens in VS Code (open-in-editor R2). */
function ErrorText(props: { error: string; api: Api }) {
  const first = () => props.error.split("\n", 1)[0];
  return (
    <Show when={failedAt(props.error)} fallback={props.error}>
      {(place) => (
        <>
          <button
            type="button"
            data-testid="error-place"
            aria-label={`Open ${first()} in VS Code`}
            class={link}
            onClick={() => void props.api.openInEditor(place().file, place().line)}
          >
            {first()}
          </button>
          {props.error.slice(first().length)}
        </>
      )}
    </Show>
  );
}

function Leaf(props: { node: TestNode; state: AppState; api: Api }) {
  const status = () => props.state.statuses[props.node.key] ?? "notRun";
  const result = () => props.state.results[props.node.key];
  const runSteps = () => props.state.steps[props.node.key];
  // Before its feature finishes, a test shows its steps as the file has them, to set breakpoints on (R4.1)
  const [fileSteps, setFileSteps] = createSignal<FileStep[] | null>(null);
  createEffect(
    () => (runSteps() ? null : props.node.key),
    (key) => {
      setFileSteps(null);
      if (key === null) return;
      let live = true;
      void props.api.steps(key).then((s) => live && setFileSteps(s));
      return () => {
        live = false;
      };
    },
  );
  const steps = (): ShownStep[] | null => runSteps() ?? fileSteps();
  const paused = () => (props.state.paused?.key === props.node.key ? props.state.paused : null);
  const pausedAt = (s: ShownStep) => {
    const p = paused();
    return !!p && p.depth === 0 && p.file === props.node.file && p.line === s.line;
  };
  const on = (line: number) =>
    props.state.breakpoints.some((b) => b.file === props.node.file && b.line === line);
  const location = () =>
    props.node.line === null ? props.node.file : `${props.node.file}:${props.node.line}`;
  return (
    <>
      {/* Focus lands here when a row is opened from the Examples table with the keyboard */}
      <h2 tabindex="-1" class="text-lg font-semibold break-words outline-none">
        {nodeName(props.node)}
      </h2>
      <p>
        <StatusBadge status={status()} />
      </p>
      <div class="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted">
        <Cut full={location()} testid="details-location" class="font-mono" wrap="max-w-full">
          {location()}
        </Cut>
        <Show when={props.node.line}>
          {(line) => (
            <button
              type="button"
              data-testid="open-in-editor"
              class={ghostSmall}
              onClick={() => void props.api.openInEditor(props.node.file, line())}
            >
              <Icon icon={ExternalLink} size={14} />
              Open in VS Code
            </button>
          )}
        </Show>
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
            <ErrorText error={result()!.error!} api={props.api} />
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
          <For each={steps()!}>
            {(step) => (
              <StepRow
                step={step}
                on={on(step.line)}
                toggle={() => void props.api.toggleBreakpoint(props.node.file, step.line)}
                paused={pausedAt(step)}
              />
            )}
          </For>
        </ul>
        <Show when={paused()?.depth}>
          <p data-testid="paused-in" class={`rounded-md px-2 py-1 text-sm ${TINT.running}`}>
            {`Paused in ${paused()!.file}:${paused()!.line}`}
          </p>
        </Show>
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
  // A memo: the badge reads it in several places, and each read would walk the whole group again.
  const status = createMemo(() => deriveStatus(props.node, props.state.statuses));
  const location = () => props.node.file || "/";
  return (
    <>
      <h2 class="text-lg font-semibold break-words">{props.node.name}</h2>
      <p>
        <StatusBadge status={status()} />
      </p>
      <div class="flex text-xs text-muted">
        <Cut full={location()} testid="details-location" class="font-mono" wrap="max-w-full">
          {location()}
        </Cut>
      </div>
      <p class="flex flex-wrap items-center gap-x-4 gap-y-1 text-sm" data-testid="details-counts">
        <span class="tabular-nums">{`${leaves(props.node).length} scenarios`}</span>
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

export function Details(props: { state: AppState; api: Api; select(key: string): void }) {
  const node = () =>
    props.state.selected && props.state.tree
      ? findNode(props.state.tree, props.state.selected)
      : null;
  // The call replaces the node while the HTTP tab is up; findCall is null once the call is gone.
  const call = () => (props.state.panel === "http" ? findCall(props.state) : null);
  let aside!: HTMLElement;
  // Opening a row unmounts the table: focus inside it moves to the row's heading, not the page.
  const open = (key: string) => {
    const focused = aside.contains(document.activeElement);
    props.select(key);
    if (!focused) return;
    flush();
    aside.querySelector<HTMLElement>("h2")?.focus();
  };
  return (
    <aside
      ref={(e) => (aside = e)}
      data-testid="details"
      class="flex min-h-0 flex-1 flex-col gap-3 overflow-auto border-l border-line p-4"
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
                fallback={
                  <>
                    <Group node={n()} state={props.state} />
                    {/* Keyed: another outline gets a new table, at its top; a refresh keeps the place */}
                    <Show when={n().kind === "outline" && n().key} keyed>
                      {(_key) => <Examples outline={n()} state={props.state} select={open} />}
                    </Show>
                  </>
                }
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
