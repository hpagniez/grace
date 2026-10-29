import { Folder, FolderOpen, TriangleAlert, X } from "lucide";
import { For, Show } from "solid-js";
import type { JSX } from "@solidjs/web";
import type { Api } from "../shared/ipc";
// a file, never a data: URL, which the CSP (default-src 'self') would block
import mascot from "../../assets/mascot.svg?no-inline";
import { btnIconSmall, primary } from "./buttons";
import { Cut } from "./Cut";
import { Icon } from "./Icon";
import type { AppState } from "./reduce";
import { Tooltip } from "./Tooltip";
import { badge, TINT } from "./ui";

const name = (path: string) => path.split(/[\\/]/).pop() || path;

/**
 * The start screen (spec R12), on the ground in the light theme and on the surface in the dark one:
 * no project is open, so there is no sheet. `notice` shows above the column.
 */
export function Home(props: { state: AppState; api: Api; notice?: JSX.Element }) {
  const entries = () => props.state.recent ?? [];
  let openButton: HTMLButtonElement | undefined;
  return (
    <div data-testid="home" class="flex min-h-0 flex-1 flex-col bg-(--home-bg)">
      {props.notice}
      <div class="min-h-0 flex-1 overflow-auto px-4 py-12">
        <div class="mx-auto flex max-w-[540px] flex-col items-start">
          {/* decorative: the heading says what the screen is for (ADR 0013) */}
          <img data-testid="mascot" src={mascot} alt="" width="240" height="180" class="mb-6" />
          <h1 class="text-xl font-semibold">Open a Karate project</h1>
          <p class="mt-1 text-sm text-muted">
            {entries().length > 0
              ? "Pick a folder, or reopen one you used before."
              : "Pick the folder that holds your .feature files. Grace brings its own Java and Karate."}
          </p>
          <button
            ref={(e) => (openButton = e)}
            data-testid="open-button"
            class={`${primary} mt-4`}
            onClick={() => void props.api.openProject()}
          >
            <Icon icon={FolderOpen} />
            Open folder…
          </button>
          <Show when={entries().length > 0}>
            <h2 class="mt-8 text-sm font-medium text-muted">Recent folders</h2>
            <ul
              data-testid="recent-list"
              class="mt-2 w-full divide-y divide-line overflow-hidden rounded-lg border border-line bg-surface"
            >
              <For each={entries()}>
                {(entry) => (
                  <li class="flex items-center pr-2">
                    <Tooltip
                      text={entry.exists ? undefined : "Folder not found"}
                      class="min-w-0 flex-1"
                    >
                      <button
                        type="button"
                        data-testid="recent-row"
                        disabled={!entry.exists}
                        class="flex min-w-0 flex-1 items-center gap-3 px-3 py-2 text-left transition-colors duration-(--duration-fast) ease-standard focus-visible:-outline-offset-2 enabled:hover:bg-hover disabled:cursor-not-allowed"
                        onClick={() => void props.api.openRecent(entry.path)}
                      >
                        <Icon icon={Folder} class="text-muted" />
                        <span class="flex min-w-0 flex-1 flex-col">
                          <span class="flex items-center gap-2">
                            <span
                              data-testid="recent-name"
                              class={`truncate font-medium ${entry.exists ? "" : "text-muted"}`}
                            >
                              {name(entry.path)}
                            </span>
                            <Show when={!entry.exists}>
                              <span
                                data-testid="recent-missing"
                                class={`${badge} ${TINT.cancelled} shrink-0`}
                              >
                                <Icon icon={TriangleAlert} size={12} />
                                Not found
                              </span>
                            </Show>
                          </span>
                          <Cut full={entry.path} wrap="w-full" class="font-mono text-xs text-muted">
                            {entry.path}
                          </Cut>
                        </span>
                      </button>
                    </Tooltip>
                    <Tooltip text="Remove from list">
                      <button
                        type="button"
                        data-testid="recent-remove"
                        aria-label="Remove from list"
                        class={`${btnIconSmall} ml-2 shrink-0`}
                        onClick={() => {
                          void props.api.removeRecent(entry.path);
                          // the rows are rebuilt when the new list arrives: keep the focus off the gone row
                          openButton?.focus();
                        }}
                      >
                        <Icon icon={X} />
                      </button>
                    </Tooltip>
                  </li>
                )}
              </For>
            </ul>
          </Show>
        </div>
      </div>
    </div>
  );
}
