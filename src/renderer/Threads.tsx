import { createEffect } from "solid-js";
import type { Api } from "../shared/ipc";
import { MAX_THREADS } from "../shared/threads";
import type { AppState } from "./reduce";

/**
 * The project's thread count: a number committed through main. The box only shows what main last
 * said; main echoes the count in force, also when it refused a change.
 */
export function Threads(props: { state: AppState; api: Api }) {
  const locked = () => props.state.run.active || props.state.busy;
  let box!: HTMLInputElement;

  // Every new view resets the box, so a refused count never stays in it. Set by hand: a `value=`
  // binding would not change when main re-sends the count it held.
  createEffect(
    () => props.state.threads,
    (t) => {
      box.value = String(t?.count ?? 1);
    },
  );

  return (
    <label class="flex items-center gap-1 text-sm">
      Threads
      <input
        type="number"
        data-testid="threads-input"
        ref={(e) => (box = e)}
        class="w-16 rounded border border-zinc-300 bg-white px-2 py-1 text-sm disabled:cursor-not-allowed disabled:opacity-40 dark:border-zinc-600 dark:bg-zinc-800"
        min="1"
        max={MAX_THREADS}
        step="1"
        disabled={locked() || !props.state.threads}
        // NaN for an empty box: main checks every value
        onChange={(e) => void props.api.setThreads(e.currentTarget.valueAsNumber)}
      />
    </label>
  );
}
