import { For, Show } from "solid-js";
import type { Api } from "../shared/ipc";
import { primary } from "./buttons";
import type { AppState } from "./reduce";

const name = (path: string) => path.split(/[\\/]/).pop() || path;

const ring =
  "outline-none focus-visible:ring-2 focus-visible:ring-sky-600 focus-visible:ring-inset";
const rowHover = "enabled:hover:bg-zinc-100 dark:enabled:hover:bg-zinc-800";

export function Home(props: { state: AppState; api: Api }) {
  const entries = () => props.state.recent ?? [];
  return (
    <div data-testid="home" class="min-h-0 flex-1 overflow-auto px-4 py-12">
      <div class="mx-auto flex max-w-[540px] flex-col items-start">
        <h1 class="text-xl font-semibold">Open a Karate project</h1>
        <p class="mt-1 text-sm text-zinc-500 dark:text-zinc-400">
          Pick a folder, or reopen one you used before.
        </p>
        <button
          data-testid="open-button"
          class={`${primary} mt-4`}
          onClick={() => void props.api.openProject()}
        >
          Open folder…
        </button>
        <Show when={entries().length > 0}>
          <h2 class="mt-8 text-sm font-medium text-zinc-500 dark:text-zinc-400">Recent folders</h2>
          <ul
            data-testid="recent-list"
            class="mt-2 w-full divide-y divide-zinc-200 overflow-hidden rounded border border-zinc-200 dark:divide-zinc-700 dark:border-zinc-700"
          >
            <For each={entries()}>
              {(entry) => (
                <li class="flex items-stretch">
                  <button
                    type="button"
                    data-testid="recent-row"
                    disabled={!entry.exists}
                    class={`min-w-0 flex-1 px-3 py-2 text-left disabled:cursor-not-allowed ${rowHover} ${ring}`}
                    onClick={() => void props.api.openRecent(entry.path)}
                  >
                    <span class="flex items-baseline gap-2">
                      <span
                        data-testid="recent-name"
                        class={`truncate text-sm font-bold ${entry.exists ? "" : "text-zinc-500 dark:text-zinc-400"}`}
                      >
                        {name(entry.path)}
                      </span>
                      <Show when={!entry.exists}>
                        <span
                          data-testid="recent-missing"
                          class="shrink-0 text-xs text-amber-700 dark:text-amber-400"
                        >
                          not found
                        </span>
                      </Show>
                    </span>
                    <span class="block truncate font-mono text-xs text-zinc-500 dark:text-zinc-400">
                      {entry.path}
                    </span>
                  </button>
                  <button
                    type="button"
                    data-testid="recent-remove"
                    aria-label="Remove from list"
                    title="Remove from list"
                    class={`shrink-0 px-3 text-lg text-zinc-500 hover:bg-zinc-100 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100 ${ring}`}
                    onClick={() => void props.api.removeRecent(entry.path)}
                  >
                    ×
                  </button>
                </li>
              )}
            </For>
          </ul>
        </Show>
      </div>
    </div>
  );
}
