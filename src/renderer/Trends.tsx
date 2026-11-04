import { createEffect, createSignal, For, onCleanup, Show } from "solid-js";
import type { Api } from "../shared/ipc";
import type { TrendPoint } from "../shared/trends";
import { Cut } from "./Cut";
import { formatMillis } from "./nodes";
import type { AppState } from "./reduce";
import { Tooltip } from "./Tooltip";
import { passRateLabel, trendCaption, trendTip } from "./trendLabels";

/**
 * The trend strip at the top of the History tab: per stored run of the picked environment, the pass rate
 * and the time per test of the tree selection (docs/specs/2026-11-04-trends.md). Mounted only while the
 * tab shows; it asks main one request at a time (R5).
 */
export function Trends(props: { state: AppState; api: Api }) {
  const [points, setPoints] = createSignal<TrendPoint[] | null>(null);
  let live = true;
  let busy = false;
  let again = false;
  /** The selection the strip should show, as the effect last saw it. */
  let wanted: string | null = null;
  const ask = () => {
    if (busy) {
      again = true; // asked once this answer is in, for the selection then current
      return;
    }
    busy = true;
    void props.api
      .trend(wanted)
      .catch(() => null)
      .then((p) => {
        busy = false;
        if (!live) return;
        if (again) {
          again = false;
          ask(); // this answer is for an older request: dropped
        } else if (p) setPoints(p);
      });
  };
  createEffect(
    () =>
      [props.state.selected, props.state.history, props.state.tree, props.state.env?.env] as const,
    ([selected]) => {
      wanted = selected;
      ask();
    },
  );
  onCleanup(() => (live = false));

  const caption = () => trendCaption(props.state.tree, props.state.selected);
  const newest = () => points()?.at(-1);
  const maxMillis = () => Math.max(0, ...(points() ?? []).map((p) => p.meanMillis ?? 0));
  const scroll = (id: string) =>
    document
      .querySelector(`[data-testid="history-row"][data-id="${id}"]`)
      ?.scrollIntoView({ block: "nearest" });

  return (
    <section
      data-testid="trends"
      role="group"
      aria-label={`Trend of ${caption()}`}
      class="px-3 py-2 text-xs"
    >
      <Cut full={caption()} testid="trends-caption" class="block text-muted" wrap="max-w-full">
        {caption()}
      </Cut>
      <div class="mt-1 flex gap-3">
        <div class="flex w-36 shrink-0 flex-col gap-1 tabular-nums">
          <span data-testid="trends-pass-label" class="flex h-6 items-center">
            <span class="text-muted">{"Pass rate "}</span>
            {newest() ? passRateLabel(newest()!) : ""}
          </span>
          <span data-testid="trends-time-label" class="flex h-6 items-center">
            <span class="text-muted">{"Time per test "}</span>
            {newest()?.meanMillis != null ? formatMillis(newest()!.meanMillis!) : ""}
          </span>
        </div>
        <Show
          when={points()?.length !== 0}
          fallback={
            <p data-testid="trends-none" class="flex h-13 items-center text-muted">
              No stored run ran these tests
            </p>
          }
        >
          {/* the two baselines run under every column; the bars sit on them */}
          <div class="relative flex h-13 gap-1">
            <span aria-hidden="true" class="absolute inset-x-0 top-6 h-px bg-line" />
            <span aria-hidden="true" class="absolute inset-x-0 top-13 h-px bg-line" />
            <For each={points() ?? []}>
              {(p) => (
                <Tooltip text={trendTip(p, Date.now())}>
                  <button
                    type="button"
                    data-testid="trend-run"
                    data-id={p.id}
                    aria-label={trendTip(p, Date.now())}
                    class="flex w-3 flex-col gap-1"
                    onClick={() => scroll(p.id)}
                  >
                    <span class="flex h-6 w-full flex-col gap-0.5 overflow-hidden rounded-t">
                      <Show when={p.failed > 0}>
                        <span
                          data-testid="trend-failed"
                          class="basis-0 bg-failed"
                          style={{ "flex-grow": p.failed }}
                        />
                      </Show>
                      <Show when={p.passed > 0}>
                        <span
                          data-testid="trend-passed"
                          class="basis-0 bg-passed"
                          style={{ "flex-grow": p.passed }}
                        />
                      </Show>
                    </span>
                    <span class="flex h-6 w-full items-end">
                      <Show when={p.meanMillis !== null}>
                        <span
                          data-testid="trend-time"
                          class={`w-full rounded-t bg-muted ${p.meanMillis! > 0 ? "min-h-0.5" : ""}`}
                          style={{ height: `${(p.meanMillis! / maxMillis()) * 100}%` }}
                        />
                      </Show>
                    </span>
                  </button>
                </Tooltip>
              )}
            </For>
          </div>
          <div class="flex flex-col gap-1 tabular-nums text-muted">
            <span class="h-6" />
            <span data-testid="trends-time-max" class="h-6">
              {maxMillis() > 0 ? formatMillis(maxMillis()) : ""}
            </span>
          </div>
        </Show>
      </div>
    </section>
  );
}
