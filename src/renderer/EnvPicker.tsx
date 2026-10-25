import { createEffect, createSignal, For, Show } from "solid-js";
import { validEnvName } from "../shared/env";
import type { Api } from "../shared/ipc";
import type { AppState } from "./reduce";

const NONE = "(none)";
export const INVALID_ENV =
  "Use letters, digits, '.', '_' or '-' (up to 64), starting with a letter or digit";

/**
 * The project's environment: a name typed or picked, committed through main. The box only shows
 * what main last said; main echoes the environment in force, also when it refused a change.
 */
export function EnvPicker(props: { state: AppState; api: Api }) {
  const view = () => props.state.env; // the reducer keeps it null with no project
  const locked = () => props.state.run.active || props.state.busy;
  // No environment is an empty box with a placeholder: Chromium narrows the datalist popup to the
  // options that match the text in the box, so "(none)" as a value would hide every suggestion.
  const shown = () => view()?.env ?? "";
  const [text, setText] = createSignal("");
  const [error, setError] = createSignal<string | null>(null);

  // Every new view resets the box, so a refused change never leaves a name the runner is not using.
  createEffect(
    () => [view(), shown()] as const,
    ([, value]) => {
      setText(value);
      setError(null);
    },
  );

  const hint = () => {
    const env = view()?.env;
    return env && !view()!.overlays.includes(env)
      ? `No karate-config-${env}.js found. Fine if karate-config.js handles it.`
      : null;
  };

  const commit = (value: string) => {
    if (locked()) return; // Electron blurs the box it disables: that change was sent already
    const name = value.trim();
    const next = name === "" || name === NONE ? null : name;
    if (next !== null && !validEnvName(next)) setError(INVALID_ENV);
    else if (next !== (view()?.env ?? null)) void props.api.setEnv(next);
    else setText(shown()); // nothing sent, so no view comes back to reset the box
  };

  return (
    <div class="flex flex-col">
      <label class="flex items-center gap-1 text-sm">
        Env
        <input
          type="text"
          data-testid="env-input"
          list="env-suggestions"
          class="w-36 rounded border border-zinc-300 bg-white px-2 py-1 text-sm disabled:cursor-not-allowed disabled:opacity-40 dark:border-zinc-600 dark:bg-zinc-800"
          autocomplete="off"
          spellcheck={false}
          disabled={locked() || !view()}
          placeholder={view() ? NONE : undefined}
          value={text()}
          aria-invalid={error() ? "true" : undefined}
          aria-describedby={error() ? "env-error" : hint() ? "env-hint" : undefined}
          onFocus={(e) => e.currentTarget.select()} // so typing replaces, and the list shows every name
          onInput={(e) => {
            setText(e.currentTarget.value);
            setError(null);
            // A pick from the list arrives as a replacement; typing alone changes nothing.
            if (e.inputType === "insertReplacementText") commit(e.currentTarget.value);
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter") commit(e.currentTarget.value);
          }}
          onBlur={(e) => commit(e.currentTarget.value)}
        />
      </label>
      <datalist id="env-suggestions">
        <option value={NONE} />
        <For each={view()?.suggestions ?? []}>{(name) => <option value={name} />}</For>
      </datalist>
      <Show when={error()}>
        <span id="env-error" data-testid="env-error" class="text-xs text-red-700 dark:text-red-400">
          {error()}
        </span>
      </Show>
      <Show when={hint()}>
        <span id="env-hint" data-testid="env-hint" class="text-xs text-zinc-600 dark:text-zinc-400">
          {hint()}
        </span>
      </Show>
    </div>
  );
}
