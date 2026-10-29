import { createEffect } from "solid-js";
import type { Api } from "../shared/ipc";
import { MAX_THREADS } from "../shared/threads";
import type { AppState } from "./reduce";
import { input } from "./ui";

/**
 * The project's thread count: a number committed through main. The box only shows what main last
 * said; main echoes the count in force, also when it refused a change.
 */
export function Threads(props: { state: AppState; api: Api }) {
  const locked = () => props.state.run.active || props.state.busy;
  let box!: HTMLInputElement;

  // Every new view resets the box, so a refused count never stays in it. Set by hand: a `value=`
  // binding would not change when main re-sends the count it held. The compute returns a fresh
  // object, so a count re-sent unchanged still resets the box.
  createEffect(
    () => ({ count: props.state.threads?.count ?? 1 }),
    ({ count }) => {
      box.value = String(count);
    },
  );

  return (
    <label class="flex items-center gap-2 text-sm text-muted">
      Threads
      <input
        type="number"
        data-testid="threads-input"
        ref={(e) => (box = e)}
        class={`${input} w-14`}
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
