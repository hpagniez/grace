import { Show } from "solid-js";
import type { Api } from "../shared/ipc";
import type { AppState } from "./reduce";
import { latestAvailable, updateBlocked } from "./updateState";

const item = "rounded px-1 hover:text-zinc-900 dark:hover:text-zinc-100";

/** The bar along the bottom of every view (R3): the update marker, the running version and Settings. */
export function StatusBar(props: { state: AppState; api: Api; openSettings(): void }) {
  const latest = () => latestAvailable(props.state.updates);
  const blocked = () => updateBlocked(props.state);
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
                    disabled={!!blocked()}
                    title={blocked()}
                    class="rounded px-1 text-sky-700 underline hover:text-sky-900 disabled:cursor-not-allowed disabled:opacity-40 dark:text-sky-400 dark:hover:text-sky-300"
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
