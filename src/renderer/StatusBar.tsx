import { Show } from "solid-js";
import type { Api } from "../shared/ipc";
import { latestOn } from "../shared/updates";
import { compareVersions } from "../shared/version";
import type { AppState } from "./reduce";

const item = "rounded px-1 hover:text-zinc-900 dark:hover:text-zinc-100";

/** The bar along the bottom of every view (R3): the update marker, the running version and Settings. */
export function StatusBar(props: { state: AppState; api: Api; openSettings(): void }) {
  // R3.2: out of date on the channel, the skipped version included
  const latest = () => {
    const u = props.state.updates;
    const l = u && latestOn(u.releases ?? [], u.settings.channel);
    return l && compareVersions(l.version, u.running) > 0 ? l.version : null;
  };
  return (
    <footer
      data-testid="status-bar"
      class="flex shrink-0 items-center justify-end gap-2 border-t border-zinc-200 bg-zinc-50 px-2 py-0.5 text-xs text-zinc-600 dark:border-zinc-700 dark:bg-zinc-800 dark:text-zinc-400"
    >
      <Show when={props.state.updates}>
        {(u) => (
          <>
            <Show when={latest()}>
              {(v) => (
                <>
                  <button
                    type="button"
                    data-testid="update-marker"
                    aria-label={`Grace ${u().running}, ${v()} available`}
                    class={`${item} flex items-center gap-1 text-amber-800 dark:text-amber-300`}
                    onClick={() => props.openSettings()}
                  >
                    <span aria-hidden="true" class="size-2 rounded-full bg-amber-500" />
                    {v()} available
                  </button>
                  <button
                    type="button"
                    data-testid="update-link"
                    class="rounded px-1 text-sky-700 underline hover:text-sky-900 dark:text-sky-400 dark:hover:text-sky-300"
                    onClick={() => void props.api.installVersion(v(), false)}
                  >
                    Update
                  </button>
                </>
              )}
            </Show>
            <button
              type="button"
              data-testid="status-version"
              class={item}
              onClick={() => props.openSettings()}
            >
              Grace {u().running}
            </button>
          </>
        )}
      </Show>
      <button
        type="button"
        data-testid="settings-button"
        class={item}
        onClick={() => props.openSettings()}
      >
        <span aria-hidden="true">⚙</span> Settings
      </button>
    </footer>
  );
}
