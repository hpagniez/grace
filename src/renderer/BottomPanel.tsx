import { Globe, History as HistoryIcon, SquareTerminal, type IconNode } from "lucide";
import { Match, Show, Switch } from "solid-js";
import type { HttpGroup } from "../shared/http";
import type { Api } from "../shared/ipc";
import { Banner } from "./Banner";
import { Console } from "./Console";
import { ClearHistory, History } from "./History";
import { narrowedCalls, shownCalls, type CallRef } from "./http";
import { HttpCalls } from "./HttpCalls";
import { Icon } from "./Icon";
import { Splitter } from "./Splitter";
import type { AppState } from "./reduce";
import { countPill, tab, tabOn } from "./ui";

type Panel = AppState["panel"];

/**
 * One tab: its icon, its label and, when it has one, the count in a pill (R18.2). The pill shows the bare
 * figure; the brackets are spoken only, so the tab still reads "History (2)" to screen readers and tests.
 */
function PanelTab(props: {
  id: Panel;
  icon: IconNode;
  label: string;
  count?: number;
  active: boolean;
  show: () => void;
}) {
  return (
    <button
      type="button"
      role="tab"
      id={`tab-${props.id}`}
      data-testid={`tab-${props.id}`}
      aria-selected={props.active ? "true" : "false"}
      class={props.active ? tabOn : tab}
      onClick={props.show}
    >
      <Icon icon={props.icon} size={14} />
      {props.label}
      <Show when={props.count !== undefined}>
        {/* relative: the spoken brackets are absolute, and one with no positioned parent escapes the sheet's clip and makes the page scrollable */}
        <span class={`${countPill} relative`}>
          <span class="sr-only">{" ("}</span>
          {props.count}
          <span class="sr-only">)</span>
        </span>
      </Show>
    </button>
  );
}

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
      class="flex min-h-24 flex-col"
    >
      <Splitter
        panel={() => section}
        main={props.main}
        setHeight={props.setHeight}
        setDragging={props.setDragging}
      />
      <div class="flex items-center border-b border-line px-2">
        <div
          role="tablist"
          aria-label="Console, run history and HTTP calls"
          class="-mb-px flex items-center gap-1"
        >
          <PanelTab
            id="console"
            icon={SquareTerminal}
            label="Console"
            active={panel() === "console"}
            show={() => props.showPanel("console")}
          />
          <PanelTab
            id="history"
            icon={HistoryIcon}
            label="History"
            count={props.state.history.length}
            active={panel() === "history"}
            show={() => props.showPanel("history")}
          />
          <PanelTab
            id="http"
            icon={Globe}
            label="HTTP"
            count={shownCalls(narrowedCalls(props.state), props.state.httpGroups).length}
            active={panel() === "http"}
            show={() => props.showPanel("http")}
          />
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
            <Console lines={props.state.console}>
              <Banner state={props.state} />
            </Console>
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
