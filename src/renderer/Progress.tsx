import { CircleAlert, CircleCheck, CircleSlash, LoaderCircle, type IconNode } from "lucide";
import { createEffect, createSignal, Show } from "solid-js";
import { Icon } from "./Icon";
import { leaves } from "./nodes";
import type { AppState } from "./reduce";
import { StatusMark } from "./StatusMark";
import { Tooltip } from "./Tooltip";

/** The outcome of the run, or what the runner is doing, with its icon (spec R14.2). */
interface Outcome {
  text: string;
  icon: IconNode;
  tone: string;
  turning?: true;
}
const WORKING = { icon: LoaderCircle, tone: "text-running", turning: true } as const;
const RUNNING: Outcome = { text: "Running…", ...WORKING };
const FINDING: Outcome = { text: "Finding tests…", ...WORKING };
const OUTCOME = {
  finished: { text: "Finished", icon: CircleCheck, tone: "text-muted" },
  cancelled: { text: "Cancelled", icon: CircleSlash, tone: "text-cancelled" },
  failed: { text: "Run failed", icon: CircleAlert, tone: "text-failed" },
} as const satisfies Record<string, Outcome>;

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
  const outcome = (): Outcome | null =>
    run().active
      ? RUNNING
      : props.state.busy
        ? FINDING
        : run().outcome
          ? OUTCOME[run().outcome!]
          : null;
  // finding tests, or a run whose first test has not finished: nothing in the bar moves otherwise
  const waiting = () => (props.state.busy && !run().active) || (run().active && run().done === 0);
  const value = (n: number) => (waiting() ? undefined : n);
  // No run in this view yet (the reducer starts a view with none on a new project or environment)
  const idle = () => !run().active && run().runId === null && run().outcome === null;
  // "Not run yet" only for a project that never ran: with runs in its history the line just counts
  const neverRan = () => idle() && props.state.history.length === 0;
  // the run view's total is 0 until a run starts: the line counts the tests of the tree
  const tests = () => (props.state.tree ? leaves(props.state.tree).length : 0);
  const fill = "flex-none transition-[width] duration-300 ease-out motion-reduce:transition-none";

  return (
    <section class="border-b border-line px-3 py-2">
      {/* flex-none segments: with a 0-wide one the two gaps would otherwise shorten a full bar; the track clips the 4 px */}
      <div
        data-testid="progress-bar"
        role="progressbar"
        aria-label="Run progress"
        aria-busy={waiting() ? "true" : "false"}
        aria-valuemin={value(0)}
        aria-valuemax={value(run().total)}
        aria-valuenow={value(run().done)}
        class="relative flex h-1.5 gap-0.5 overflow-hidden rounded-full bg-line"
      >
        <div class={`${fill} bg-passed`} style={`width:${pct(run().passed)}%`} />
        <div class={`${fill} bg-failed`} style={`width:${pct(run().failed)}%`} />
        <div class={`${fill} bg-cancelled`} style={`width:${pct(run().interrupted)}%`} />
        {/* over the segments: during a Refresh they still show the last run, often a full bar */}
        <Show when={waiting()}>
          <div
            data-testid="progress-stripe"
            aria-hidden="true"
            class="absolute inset-y-0 left-0 w-[30%] animate-slide rounded-full bg-running motion-reduce:animate-none"
          />
        </Show>
      </div>
      <div class="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-sm">
        <span
          data-testid="progress-text"
          class={idle() ? "text-muted tabular-nums" : "font-mono tabular-nums"}
        >
          {idle()
            ? `${tests()} ${tests() === 1 ? "test" : "tests"}${neverRan() ? " · Not run yet" : ""}`
            : `${run().done} / ${run().total}`}
        </span>
        <Show when={!idle()}>
          <span class="inline-flex items-center gap-1.5">
            <StatusMark status="passed" decorative />
            <span data-testid="progress-passed" class="font-mono font-medium tabular-nums">
              {run().passed}
            </span>{" "}
            passed
          </span>
          <span class="inline-flex items-center gap-1.5">
            <StatusMark status="failed" decorative />
            <span data-testid="progress-failed" class="font-mono font-medium tabular-nums">
              {run().failed}
            </span>{" "}
            failed
          </span>
          <span data-testid="progress-elapsed" class="font-mono tabular-nums">
            {mmss(elapsed())}
          </span>
        </Show>
        {/* no environment: the wrapper goes too, or its gap would stay */}
        <Tooltip
          text={run().env ? "Environment of the run" : undefined}
          class="has-[>:empty]:hidden"
        >
          <span
            data-testid="run-env"
            class="inline-flex h-5 items-center rounded-sm bg-selected px-1.5 text-2xs font-medium text-muted"
          >
            {run().env ?? ""}
          </span>
        </Tooltip>
        <span data-testid="run-outcome" class="inline-flex items-center gap-1.5 font-medium">
          <Show when={outcome()}>
            {(o) => (
              <>
                <Icon icon={o().icon} class={`${o().tone}${o().turning ? " animate-spin" : ""}`} />
                {o().text}
              </>
            )}
          </Show>
        </span>
      </div>
    </section>
  );
}
