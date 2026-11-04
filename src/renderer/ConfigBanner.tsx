import { CircleAlert, ExternalLink } from "lucide";
import { createMemo, Show } from "solid-js";
import type { Api } from "../shared/ipc";
import { ghostSmall } from "./buttons";
import { configFailure } from "./failures";
import { Icon } from "./Icon";
import { leaves } from "./nodes";
import type { AppState } from "./reduce";
import { TINT } from "./ui";

/** Says once, after a run, that karate-config.js (or an overlay) failed its tests (#229 R6). */
export function ConfigBanner(props: { state: AppState; api: Api }) {
  // Found when a run ends, from the last run's failed tests; nothing during a run or while idle.
  const failure = createMemo(() =>
    props.state.run.active || !props.state.tree
      ? null
      : configFailure(leaves(props.state.tree), props.state.run.failedKeys, props.state.results),
  );
  return (
    <Show when={failure()}>
      {(f) => (
        <div class="px-3 py-2">
          <div role="alert" data-testid="config-failure" class={`rounded-lg p-3 ${TINT.failed}`}>
            <p class="flex flex-wrap items-center gap-x-1.5 gap-y-1 text-sm font-semibold">
              <Icon icon={CircleAlert} size={14} />
              <span class="text-ink">
                {`${f().file} failed for ${f().count} ${f().count === 1 ? "test" : "tests"}`}
              </span>
              <button
                type="button"
                class={ghostSmall}
                aria-label={`Open ${f().file} in VS Code`}
                onClick={() => void props.api.openInEditor(f().file, f().line)}
              >
                <Icon icon={ExternalLink} size={14} />
                Open in VS Code
              </button>
            </p>
            <pre
              data-testid="config-failure-reason"
              class="mt-1 max-h-60 overflow-auto font-mono text-xs break-words whitespace-pre-wrap text-ink"
            >
              {f().reason}
            </pre>
          </div>
        </div>
      )}
    </Show>
  );
}
