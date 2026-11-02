import { Braces, ChevronRight } from "lucide";
import { createSignal, For, Show } from "solid-js";
import type { DebugVar } from "../shared/events";
import { ghostSmall } from "./buttons";
import { Cut } from "./Cut";
import { EmptyState } from "./EmptyState";
import { Icon } from "./Icon";
import type { AppState } from "./reduce";
import { box, tagChip } from "./ui";

/** JSON indented to read; a value Java cut short is no longer JSON, so it shows as sent. */
const full = (v: DebugVar) => {
  if (v.kind !== "json" || v.cut) return v.value;
  try {
    return JSON.stringify(JSON.parse(v.value), null, 2);
  } catch {
    return v.value;
  }
};

function VariableRow(props: { v: DebugVar }) {
  const [open, setOpen] = createSignal(false);
  const opens = () => props.v.kind === "json" || props.v.kind === "xml";
  return (
    <li data-testid="variable" data-name={props.v.name} class="py-1">
      <div class="flex min-h-6 items-center gap-2 text-sm">
        <span class="shrink-0 font-mono text-xs font-medium">{props.v.name}</span>
        <span class={`${tagChip} shrink-0`}>{props.v.type ?? props.v.kind}</span>
        <Cut full={props.v.value} wrap="min-w-0 flex-1" side="over">
          <span class="font-mono text-xs">{props.v.value}</span>
        </Cut>
        <Show when={props.v.cut}>
          {(cut) => (
            <span data-testid="variable-cut" class="shrink-0 text-xs text-muted tabular-nums">
              {`${cut().toLocaleString("en-US")} more characters`}
            </span>
          )}
        </Show>
        <Show when={opens()}>
          <button
            type="button"
            data-testid="variable-toggle"
            class={`${ghostSmall} shrink-0`}
            aria-expanded={open() ? "true" : "false"}
            aria-label={`${open() ? "Hide" : "Show"} ${props.v.name}`}
            onClick={() => setOpen(!open())}
          >
            <Icon
              icon={ChevronRight}
              size={14}
              class={`transition-transform duration-(--duration-base) ease-standard ${open() ? "rotate-90" : ""}`}
            />
            Value
          </button>
        </Show>
      </div>
      <Show when={open()}>
        <pre data-testid="variable-value" class={`${box} mt-1 max-h-60 overflow-auto`}>
          {full(props.v)}
        </pre>
      </Show>
    </li>
  );
}

/** The paused scenario's variables (step debugger R7.4); the last pause's, dimmed, while the run goes on. */
export function Variables(props: { state: AppState }) {
  const sorted = () => [...(props.state.vars ?? [])].sort((a, b) => a.name.localeCompare(b.name));
  const running = () => !props.state.paused;
  return (
    <Show
      when={props.state.vars}
      fallback={
        <EmptyState icon={Braces} text="Variables show when the run pauses" testid="variables" />
      }
    >
      <div
        data-testid="variables"
        class={`min-h-0 flex-1 overflow-auto px-3 py-1 ${running() ? "opacity-60" : ""}`}
      >
        <Show when={running()}>
          <p class="py-1 text-xs text-muted">Running…</p>
        </Show>
        <ul class="divide-y divide-line">
          <For each={sorted()}>{(v) => <VariableRow v={v} />}</For>
        </ul>
      </div>
    </Show>
  );
}
