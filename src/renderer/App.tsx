import { createSignal, onCleanup, onSettled, Show } from "solid-js";
import type { Api } from "../shared/ipc";
import { BottomPanel } from "./BottomPanel";
import { Details } from "./Details";
import { FilterBar } from "./FilterBar";
import { loadPanelHeight } from "./panelHeight";
import { Progress } from "./Progress";
import { Toolbar } from "./Toolbar";
import { Tree } from "./Tree";
import { createAppStore } from "./store";

export function App(props: { api: Api }) {
  const { state, select, showPanel, selectCall, toggleGroup, dispose } = createAppStore(props.api);
  onCleanup(dispose);
  onSettled(() => document.getElementById("startup")?.remove()); // index.html's startup screen
  const [height, setHeight] = createSignal(loadPanelHeight()); // read before the first paint
  const [dragging, setDragging] = createSignal(false);
  let main: HTMLElement | undefined;
  return (
    <div
      data-testid="app"
      class={`flex h-screen flex-col overflow-clip bg-white text-zinc-900 dark:bg-zinc-900 dark:text-zinc-100 ${dragging() ? "cursor-row-resize select-none" : state.busy || state.run.active ? "cursor-progress" : ""}`}
    >
      <Toolbar state={state} api={props.api} />
      <Progress state={state} />
      <FilterBar state={state} api={props.api} />
      <Show when={state.notice}>
        <div
          data-testid="notice"
          class="border-b border-amber-300 bg-amber-50 px-3 py-1 text-sm text-amber-900 dark:border-amber-700 dark:bg-amber-950 dark:text-amber-200"
        >
          {state.notice}
        </div>
      </Show>
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
    </div>
  );
}
