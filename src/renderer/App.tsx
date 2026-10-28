import { createEffect, createSignal, onCleanup, Show } from "solid-js";
import type { Api } from "../shared/ipc";
import { BottomPanel } from "./BottomPanel";
import { Details } from "./Details";
import { FilterBar } from "./FilterBar";
import { Home } from "./Home";
import { loadPanelHeight } from "./panelHeight";
import { Progress } from "./Progress";
import { StatusBar } from "./StatusBar";
import { Toolbar } from "./Toolbar";
import { Tree } from "./Tree";
import { createAppStore } from "./store";

export function App(props: { api: Api }) {
  const { state, select, showPanel, selectCall, toggleGroup, dispose } = createAppStore(props.api);
  onCleanup(dispose);
  // index.html's startup screen stays until main says what is open, so home never flashes
  createEffect(
    () => state.loaded,
    (loaded) => {
      if (loaded) document.getElementById("startup")?.remove();
    },
  );
  const home = () => !state.project && !state.opening;
  // "Folder not found" and the like arrive as an error with no project: home shows it as a notice
  const notice = () => state.notice ?? (home() ? state.error : null);
  const [height, setHeight] = createSignal(loadPanelHeight()); // read before the first paint
  const [dragging, setDragging] = createSignal(false);
  let main: HTMLElement | undefined;
  return (
    <Show when={state.loaded}>
      <div
        data-testid="app"
        class={`flex h-screen flex-col overflow-clip bg-white text-zinc-900 dark:bg-zinc-900 dark:text-zinc-100 ${dragging() ? "cursor-row-resize select-none" : state.busy || state.run.active ? "cursor-progress" : ""}`}
      >
        {/* the view cuts off at the status bar, so a short window never hides the bar */}
        <div data-testid="view" class="flex min-h-0 flex-1 flex-col overflow-clip">
          <Show when={!home()}>
            <Toolbar state={state} api={props.api} />
            <Progress state={state} />
            <FilterBar state={state} api={props.api} />
          </Show>
          <Show when={notice()}>
            <div
              data-testid="notice"
              class="border-b border-amber-300 bg-amber-50 px-3 py-1 text-sm text-amber-900 dark:border-amber-700 dark:bg-amber-950 dark:text-amber-200"
            >
              {notice()}
            </div>
          </Show>
          <Show when={!home()} fallback={<Home state={state} api={props.api} />}>
            <main ref={(e) => (main = e)} class="flex min-h-30 flex-1">
              <div class="flex min-h-0 w-1/2 flex-col">
                <Tree state={state} select={select} api={props.api} />
              </div>
              <div class="flex min-h-0 w-1/2 flex-col">
                <Details state={state} api={props.api} />
              </div>
            </main>
            <BottomPanel
              state={state}
              api={props.api}
              showPanel={showPanel}
              selectCall={selectCall}
              toggleGroup={toggleGroup}
              height={height()}
              setHeight={setHeight}
              main={() => main}
              setDragging={setDragging}
            />
          </Show>
        </div>
        {/* the Settings page (R6) is not built yet */}
        <StatusBar state={state} api={props.api} openSettings={() => {}} />
      </div>
    </Show>
  );
}
