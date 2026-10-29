import { createEffect, createSignal, onCleanup, Show } from "solid-js";
import type { Api } from "../shared/ipc";
import { BottomPanel } from "./BottomPanel";
import { Details } from "./Details";
import { FilterBar } from "./FilterBar";
import { Home } from "./Home";
import { Notice } from "./Notice";
import { loadPanelHeight } from "./panelHeight";
import { Progress } from "./Progress";
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
  const noticeBox = () => (
    <Show when={notice()}>
      {(text) => (
        <div class="px-3 py-2">
          <Notice kind="warning" text={text()} testid="notice" />
        </div>
      )}
    </Show>
  );
  const [height, setHeight] = createSignal(loadPanelHeight()); // read before the first paint
  const [dragging, setDragging] = createSignal(false);
  let main: HTMLElement | undefined;
  return (
    <Show when={state.loaded}>
      <div
        data-testid="app"
        class={`flex h-screen flex-col overflow-clip bg-ground text-ink ${dragging() ? "cursor-row-resize select-none" : state.busy || state.run.active ? "cursor-progress" : ""}`}
      >
        <Show
          when={!home()}
          fallback={
            <>
              {noticeBox()}
              <Home state={state} api={props.api} />
            </>
          }
        >
          <Toolbar state={state} api={props.api} />
          {/* the sheet (spec R3.3): the project view on a surface, inset and rounded in the light theme, flat in the dark one; overflow-clip, like the root, so focus cannot scroll it */}
          <div
            data-testid="sheet"
            class="mx-(--sheet-inset) mb-(--sheet-inset) flex min-h-0 flex-1 flex-col overflow-clip rounded-(--sheet-radius) border-x-(length:--sheet-side) border-t border-b-(length:--sheet-side) border-line bg-surface"
          >
            <Progress state={state} />
            <FilterBar state={state} api={props.api} />
            {noticeBox()}
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
        </Show>
      </div>
    </Show>
  );
}
