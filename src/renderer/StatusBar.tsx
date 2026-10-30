import { CircleArrowUp, Download, Settings as Gear } from "lucide";
import { Show } from "solid-js";
import type { Api } from "../shared/ipc";
import { Icon } from "./Icon";
import type { AppState } from "./reduce";
import { Tooltip } from "./Tooltip";
import { badge, TINT } from "./ui";
import { latestAvailable, updateBlocked } from "./updateState";

// As tall as the bar: a 24 px target (R10.3). At the window's edge the focus ring goes inside (R10.1).
const item =
  "inline-flex h-6 items-center gap-1 rounded-sm px-1.5 transition-colors duration-(--duration-fast) ease-standard focus-visible:-outline-offset-2 enabled:hover:bg-hover enabled:hover:text-ink disabled:cursor-not-allowed disabled:opacity-40";

/**
 * The bar along the bottom of every view (app-updates R3): the update marker, the running version and
 * Settings. 24 px on the ground, muted 12 px text, 14 px icons (spec R21).
 */
export function StatusBar(props: { state: AppState; api: Api; openSettings(): void }) {
  const latest = () => latestAvailable(props.state.updates);
  const blocked = () => updateBlocked(props.state);
  return (
    <footer
      data-testid="status-bar"
      class="flex h-6 shrink-0 items-center justify-end gap-1 bg-ground px-2 text-xs text-muted"
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
                    class={item}
                    onClick={() => props.openSettings()}
                  >
                    <span class={`${badge} ${TINT.running}`}>
                      <Icon icon={CircleArrowUp} size={12} />
                      {v()} available
                    </span>
                  </button>
                  <Tooltip text={blocked()} describe>
                    <button
                      type="button"
                      data-testid="update-link"
                      disabled={!!blocked()}
                      class={`${item} font-medium text-ink`}
                      onClick={() => void props.api.installVersion(v(), false)}
                    >
                      <Icon icon={Download} size={14} />
                      Update
                    </button>
                  </Tooltip>
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
        <Icon icon={Gear} size={14} />
        Settings
      </button>
    </footer>
  );
}
