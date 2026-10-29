import { Shuffle } from "lucide";
import { For } from "solid-js";
import { flakyCounts, flakyHeading, orderEnvs, threadsLabel, type FlakyEnv } from "../shared/flaky";
import { whenLabel } from "../shared/history";
import type { Api } from "../shared/ipc";
import { btnSmall } from "./buttons";
import { Icon } from "./Icon";
import { STATUS_LABEL } from "./nodes";
import { StatusMark } from "./StatusMark";
import { TINT } from "./ui";

/** The runs that prove a test flaky, one box per environment, the current one first (F15). */
export function FlakyProof(props: {
  envs: FlakyEnv[];
  current: string | null;
  locked: boolean;
  api: Api;
}) {
  return (
    <For each={orderEnvs(props.envs, props.current)}>
      {(e) => (
        <section data-testid="flaky-proof" class={`space-y-1 rounded-lg p-3 ${TINT.cancelled}`}>
          <h3 class="flex items-center gap-1.5 text-sm font-semibold">
            <Icon icon={Shuffle} size={14} />
            {flakyHeading(e.env)}
          </h3>
          <p class="text-xs text-ink">{flakyCounts(e.passed, e.failed)}</p>
          <ul class="space-y-1">
            <For each={e.runs}>
              {(r) => (
                <li
                  data-testid="flaky-run"
                  data-id={r.id}
                  data-status={r.status}
                  class="flex items-center gap-3 text-xs text-ink"
                >
                  <span>{whenLabel(r.startedAt, Date.now())}</span>
                  <span class="inline-flex items-center gap-1">
                    <StatusMark status={r.status} decorative />
                    {STATUS_LABEL[r.status]}
                  </span>
                  <span>{threadsLabel(r.threads)}</span>
                  <button
                    type="button"
                    data-testid="flaky-run-report"
                    class={btnSmall}
                    disabled={props.locked || !r.reportAvailable}
                    onClick={() => void props.api.openRunReport(r.id)}
                  >
                    Report
                  </button>
                </li>
              )}
            </For>
          </ul>
        </section>
      )}
    </For>
  );
}
