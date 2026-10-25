import { Show } from "solid-js";
import type { Api } from "../shared/ipc";
import { Console } from "./Console";
import { ClearHistory, History } from "./History";
import type { AppState } from "./reduce";

type Panel = AppState["panel"];

const TAB = "border-b-2 px-3 py-1 text-sm hover:text-zinc-900 dark:hover:text-zinc-100";
const tabClass = (on: boolean) =>
  `${TAB} ${on ? "border-sky-600 font-medium" : "border-transparent text-zinc-600 dark:text-zinc-400"}`;

/** The Console and History tabs under the tree and details. The panel keeps one height for both. */
export function BottomPanel(props: { state: AppState; api: Api; showPanel: (p: Panel) => void }) {
  const panel = () => props.state.panel;
  return (
    <section class="flex h-48 shrink-0 flex-col border-t border-zinc-200 dark:border-zinc-700">
      <div
        role="tablist"
        aria-label="Console and run history"
        class="flex items-center gap-1 border-b border-zinc-200 bg-zinc-50 px-2 dark:border-zinc-700 dark:bg-zinc-800"
      >
        <button
          type="button"
          role="tab"
          id="tab-console"
          data-testid="tab-console"
          aria-selected={panel() === "console" ? "true" : "false"}
          class={tabClass(panel() === "console")}
          onClick={() => props.showPanel("console")}
        >
          Console
        </button>
        <button
          type="button"
          role="tab"
          id="tab-history"
          data-testid="tab-history"
          aria-selected={panel() === "history" ? "true" : "false"}
          class={tabClass(panel() === "history")}
          onClick={() => props.showPanel("history")}
        >
          {`History (${props.state.history.length})`}
        </button>
        <Show when={panel() === "history"}>
          <div class="ml-auto">
            <ClearHistory state={props.state} api={props.api} />
          </div>
        </Show>
      </div>
      <div role="tabpanel" aria-labelledby={`tab-${panel()}`} class="flex min-h-0 flex-1 flex-col">
        <Show
          when={panel() === "console"}
          fallback={<History state={props.state} api={props.api} />}
        >
          <Console lines={props.state.console} />
        </Show>
      </div>
    </section>
  );
}
