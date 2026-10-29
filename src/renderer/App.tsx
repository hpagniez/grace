import { createEffect, createSignal, onCleanup, Show } from "solid-js";
import type { Api } from "../shared/ipc";
import { BottomPanel } from "./BottomPanel";
import { Details } from "./Details";
import { FilterBar } from "./FilterBar";
import { Home } from "./Home";
import { Notice } from "./Notice";
import { loadPanelHeight } from "./panelHeight";
import { Progress } from "./Progress";
import { Settings } from "./Settings";
import { StatusBar } from "./StatusBar";
import { Toolbar } from "./Toolbar";
import { Tree } from "./Tree";
import { UpdateModal } from "./UpdateModal";
import { UpdateProgress } from "./UpdateProgress";
import { createAppStore } from "./store";
import { sheet } from "./ui";

export function App(props: { api: Api }) {
  const { state, select, showPanel, selectCall, toggleGroup, dismissPrompt, dispose } =
    createAppStore(props.api);
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
  const [settingsOpen, setSettingsOpen] = createSignal(false);
  let main: HTMLElement | undefined;
  return (
    <Show when={state.loaded}>
      <div
        data-testid="app"
        class={`flex h-screen flex-col overflow-clip bg-ground text-ink ${dragging() ? "cursor-row-resize select-none" : state.busy || state.run.active ? "cursor-progress" : ""}`}
      >
        {/* the view cuts off at the status bar, so a short window never hides the bar */}
        <div data-testid="view" class="relative flex min-h-0 flex-1 flex-col overflow-clip">
          {/* The project view stays mounted under the Settings page (R6.1), so Back finds every local state
              as it was: collapsed tree nodes, scroll positions, the console following its tail.
              Invisible, not hidden: it keeps its layout and scroll offsets, and takes no focus or clicks. */}
          <div
            data-testid="project-view"
            class={`flex min-h-0 flex-1 flex-col ${settingsOpen() ? "invisible" : ""}`}
          >
            <Show
              when={!home()}
              fallback={
                <Home
                  state={state}
                  api={props.api}
                  notice={<Show when={!settingsOpen()}>{noticeBox()}</Show>}
                />
              }
            >
              <Toolbar state={state} api={props.api} />
              {/* the project view on the sheet (spec R3.3) */}
              <div data-testid="sheet" class={sheet}>
                <Progress state={state} />
                <FilterBar state={state} api={props.api} />
                <Show when={!settingsOpen()}>{noticeBox()}</Show>
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
          <Show when={settingsOpen()}>
            <div class="absolute inset-0 flex flex-col">
              {/* the notice moves onto the page, so one arriving there is seen */}
              <Settings
                state={state}
                api={props.api}
                notice={noticeBox()}
                back={() => setSettingsOpen(false)}
              />
            </div>
          </Show>
        </div>
        <StatusBar state={state} api={props.api} openSettings={() => setSettingsOpen(true)} />
        <UpdateModal state={state} api={props.api} dismiss={dismissPrompt} />
        <UpdateProgress state={state} api={props.api} />
      </div>
    </Show>
  );
}
