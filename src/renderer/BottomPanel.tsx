import { Match, Show, Switch } from "solid-js";
import type { HttpGroup } from "../shared/http";
import type { Api } from "../shared/ipc";
import { Console } from "./Console";
import { ClearHistory, History } from "./History";
import { narrowedCalls, shownCalls, type CallRef } from "./http";
import { HttpCalls } from "./HttpCalls";
import { Splitter } from "./Splitter";
import type { AppState } from "./reduce";

type Panel = AppState["panel"];

const TAB = "border-b-2 px-3 py-1 text-sm hover:text-zinc-900 dark:hover:text-zinc-100";
const tabClass = (on: boolean) =>
  `${TAB} ${on ? "border-sky-600 font-medium" : "border-transparent text-zinc-600 dark:text-zinc-400"}`;

/** The Console, History and HTTP tabs under the tree and details. The panel keeps one height for all. */
export function BottomPanel(props: {
  state: AppState;
  api: Api;
  showPanel: (p: Panel) => void;
  selectCall: (ref: CallRef) => void;
  toggleGroup: (g: HttpGroup | "all") => void;
  /** The saved height; flexbox shrinks the panel when the window is too small for it. */
  height: number;
  setHeight: (h: number) => void;
  main: () => HTMLElement | undefined;
  setDragging: (on: boolean) => void;
}) {
  const panel = () => props.state.panel;
  let section: HTMLElement | undefined;
  return (
    <section
      ref={(e) => (section = e)}
      data-testid="bottom-panel"
      style={`height:${props.height}px`}
      class="flex min-h-24 flex-col border-t border-zinc-200 dark:border-zinc-700"
    >
      <Splitter
        panel={() => section}
        main={props.main}
        setHeight={props.setHeight}
        setDragging={props.setDragging}
      />
      <div class="flex items-center border-b border-zinc-200 bg-zinc-50 px-2 dark:border-zinc-700 dark:bg-zinc-800">
        <div
          role="tablist"
          aria-label="Console, run history and HTTP calls"
          class="flex items-center gap-1"
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
          <button
            type="button"
            role="tab"
            id="tab-http"
            data-testid="tab-http"
            aria-selected={panel() === "http" ? "true" : "false"}
            class={tabClass(panel() === "http")}
            onClick={() => props.showPanel("http")}
          >
            {`HTTP (${shownCalls(narrowedCalls(props.state), props.state.httpGroups).length})`}
          </button>
        </div>
        <Show when={panel() === "history"}>
          <div class="ml-auto">
            <ClearHistory state={props.state} api={props.api} />
          </div>
        </Show>
      </div>
      <div role="tabpanel" aria-labelledby={`tab-${panel()}`} class="flex min-h-0 flex-1 flex-col">
        <Switch>
          <Match when={panel() === "console"}>
            <Console lines={props.state.console} />
          </Match>
          <Match when={panel() === "history"}>
            <History state={props.state} api={props.api} />
          </Match>
          <Match when={panel() === "http"}>
            <HttpCalls
              state={props.state}
              selectCall={props.selectCall}
              toggleGroup={props.toggleGroup}
            />
          </Match>
        </Switch>
      </div>
    </section>
  );
}
