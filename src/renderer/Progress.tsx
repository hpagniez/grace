import { createEffect, createSignal, Show } from "solid-js";
import type { AppState } from "./reduce";

const OUTCOME_TEXT = {
  finished: "Finished",
  cancelled: "Cancelled",
  failed: "Run failed",
} as const;

const pad = (n: number) => String(n).padStart(2, "0");
const mmss = (ms: number) => {
  const s = Math.floor(ms / 1000);
  return `${pad(Math.floor(s / 60))}:${pad(s % 60)}`;
};

export function Progress(props: { state: AppState }) {
  const run = () => props.state.run;
  const [tickedMillis, setTickedMillis] = createSignal(0);

  createEffect(
    () => (run().active ? run().startedAt : null),
    (startedAt) => {
      if (startedAt === null) return;
      const tick = () => setTickedMillis(Math.max(0, Date.now() - startedAt));
      tick();
      const id = setInterval(tick, 1000);
      return () => clearInterval(id);
    },
  );

  const elapsed = () =>
    run().active ? tickedMillis() : (run().summary?.durationMillis ?? tickedMillis());
  const pct = (n: number) => (run().total > 0 ? Math.min(100, (n / run().total) * 100) : 0);
  const outcome = () =>
    run().active
      ? "Running…"
      : props.state.busy
        ? "Finding tests…"
        : run().outcome
          ? OUTCOME_TEXT[run().outcome!]
          : "";
  // finding tests, or a run whose first test has not finished: nothing in the bar moves otherwise
  const waiting = () => (props.state.busy && !run().active) || (run().active && run().done === 0);
  const value = (n: number) => (waiting() ? undefined : n);
  const fill = "transition-[width] duration-300 ease-out motion-reduce:transition-none";

  return (
    <section class="border-b border-zinc-200 px-3 py-2 dark:border-zinc-700">
      <div
        data-testid="progress-bar"
        role="progressbar"
        aria-label="Run progress"
        aria-busy={waiting() ? "true" : "false"}
        aria-valuemin={value(0)}
        aria-valuemax={value(run().total)}
        aria-valuenow={value(run().done)}
        class="relative flex h-2 overflow-hidden rounded bg-zinc-200 dark:bg-zinc-700"
      >
        <div class={`${fill} bg-green-500`} style={`width:${pct(run().passed)}%`} />
        <div class={`${fill} bg-red-500`} style={`width:${pct(run().failed)}%`} />
        {/* over the segments: during a Refresh they still show the last run, often a full bar */}
        <Show when={waiting()}>
          <div
            data-testid="progress-stripe"
            aria-hidden="true"
            class="absolute inset-y-0 left-0 w-[30%] animate-slide rounded bg-sky-600 motion-reduce:animate-none dark:bg-sky-500"
          />
        </Show>
      </div>
      <div class="mt-1 flex gap-4 text-sm">
        <span data-testid="progress-text">{`${run().done} / ${run().total}`}</span>
        <span class="text-green-700 dark:text-green-400">
          Passed <span data-testid="progress-passed">{run().passed}</span>
        </span>
        <span class="text-red-700 dark:text-red-400">
          Failed <span data-testid="progress-failed">{run().failed}</span>
        </span>
        <span data-testid="progress-elapsed" class="font-mono">
          {mmss(elapsed())}
        </span>
        <span
          data-testid="run-env"
          title="Environment of the run"
          class="rounded bg-zinc-200 px-1.5 text-xs empty:hidden dark:bg-zinc-700"
        >
          {run().env ?? ""}
        </span>
        <span data-testid="run-outcome" class="font-medium">
          {outcome()}
        </span>
      </div>
    </section>
  );
}
