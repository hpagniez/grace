import { createMemo, createSignal, For, Show, untrack } from "solid-js";
import { MAX_ROWS, PREFIX, type ImportEntry } from "../shared/env";
import { btn, primary } from "./buttons";
import { FieldMessage } from "./FieldMessage";
import { importPlan } from "./importPlan";
import { input } from "./ui";

/**
 * The Properties dialog's import step (docs/specs/2026-11-04-env-var-properties.md R5): where the
 * file's keys go, as which kind and under which names, and what that adds, replaces and skips. It
 * shows names and reasons, never a value.
 */
export function ImportStep(props: {
  file: string;
  entries: ImportEntry[];
  scopes: string[];
  scope: string;
  existing: (scope: string) => { name: string; envVar: boolean }[];
  /** False when the scope has no list and no other one fits. */
  canAddList: (scope: string) => boolean;
  onImport: (scope: string, envVar: boolean, rows: { name: string; value: string }[]) => void;
  onBack: () => void;
}) {
  const [target, setTarget] = createSignal(untrack(() => props.scope));
  const [envVar, setEnvVar] = createSignal(true);
  const [prefix, setPrefix] = createSignal("");

  const plan = createMemo(() =>
    importPlan(props.entries, {
      envVar: envVar(),
      prefix: envVar() ? prefix() : "",
      existing: props.existing(target()),
    }),
  );
  const prefixError = () =>
    envVar() && !PREFIX.test(prefix()) ? "Letters, digits and _ only" : null;
  const limit = () =>
    !props.canAddList(target())
      ? "At most 100 environments"
      : props.existing(target()).length + plan().adds > MAX_ROWS
        ? `At most ${MAX_ROWS} properties per list`
        : null;
  const preview = () => {
    const first = plan().rows[0];
    return first && `${first.key} → ${first.name}`;
  };
  const blocked = () => prefixError() !== null || limit() !== null || plan().rows.length === 0;

  return (
    <div data-testid="import-step" class="flex flex-col gap-3">
      <h3 class="text-sm font-semibold">Import {props.file}</h3>
      <label class="flex items-center gap-2 text-sm">
        Environment
        <select
          data-testid="import-scope"
          class={`${input} min-w-0 flex-1`}
          onChange={(e) => setTarget(e.currentTarget.value)}
        >
          <For each={props.scopes}>
            {(s) => (
              <option value={s} selected={s === target()}>
                {s === "" ? "All environments" : s}
              </option>
            )}
          </For>
        </select>
      </label>
      <label class="flex h-6 items-center gap-1 text-sm">
        <input
          type="checkbox"
          data-testid="import-envvar"
          class="accent-accent"
          checked={envVar()}
          onChange={(e) => setEnvVar(e.currentTarget.checked)}
        />
        Environment variables
      </label>
      <Show when={envVar()}>
        <label class="flex items-center gap-2 text-sm">
          Prefix
          <input
            data-testid="import-prefix"
            autocomplete="off"
            spellcheck={false}
            class={`${input} w-40 min-w-0`}
            value={prefix()}
            aria-invalid={prefixError() ? "true" : undefined}
            aria-describedby={prefixError() ? "import-prefix-error" : undefined}
            onInput={(e) => setPrefix(e.currentTarget.value)}
          />
        </label>
        <Show when={prefixError()}>
          <FieldMessage kind="error" id="import-prefix-error" testid="import-prefix-error">
            {prefixError()}
          </FieldMessage>
        </Show>
      </Show>
      <Show when={!prefixError() && preview()}>
        <p data-testid="import-preview" class="font-mono text-xs text-muted">
          {preview()}
        </p>
      </Show>
      <div data-testid="import-summary" class="flex flex-col gap-1 text-sm">
        <p>Adds {plan().adds}</p>
        <Show when={plan().replaces.length}>
          <p>
            Replaces {plan().replaces.length}: {plan().replaces.join(", ")}
          </p>
        </Show>
        <Show when={plan().skips.length}>
          <p>Skips {plan().skips.length}:</p>
          <ul class="max-h-40 overflow-y-auto text-xs text-muted">
            <For each={plan().skips}>
              {(s) => (
                <li data-testid="import-skip">
                  {s.key}: {s.reason}
                </li>
              )}
            </For>
          </ul>
        </Show>
      </div>
      <Show when={limit()}>
        <FieldMessage kind="error" testid="import-limit">
          {limit()}
        </FieldMessage>
      </Show>
      <div class="flex gap-2">
        <button
          type="button"
          data-testid="import-confirm"
          class={primary}
          disabled={blocked()}
          onClick={() => props.onImport(target(), envVar(), plan().rows)}
        >
          Import
        </button>
        <button type="button" data-testid="import-back" class={btn} onClick={() => props.onBack()}>
          Back
        </button>
      </div>
    </div>
  );
}
