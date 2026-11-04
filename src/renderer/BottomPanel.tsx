import {
  Braces,
  Globe,
  History as HistoryIcon,
  Search,
  SquareTerminal,
  type IconNode,
} from "lucide";
import { createMemo, createSignal, flush, For, Match, Show, Switch } from "solid-js";
import type { HttpGroup } from "../shared/http";
import type { Api } from "../shared/ipc";
import { Banner } from "./Banner";
import { btnIconSmall } from "./buttons";
import { Console, ConsoleErrors, createConsoleSearch } from "./Console";
import { ClearHistory, History } from "./History";
import { callsOf, scopeTests, shownCalls, type CallRef } from "./http";
import { HttpCalls } from "./HttpCalls";
import { Icon } from "./Icon";
import { loadTabOrder, moveTab, saveTabOrder, TAB_IDS, type TabId } from "./panelTabs";
import { Splitter } from "./Splitter";
import type { AppState } from "./reduce";
import { Tooltip } from "./Tooltip";
import { countPill, tab, tabDrop, tabOn } from "./ui";
import { Variables } from "./Variables";

type Panel = AppState["panel"];

const TABS: Record<TabId, { icon: IconNode; label: string }> = {
  console: { icon: SquareTerminal, label: "Console" },
  history: { icon: HistoryIcon, label: "History" },
  http: { icon: Globe, label: "HTTP" },
};

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
  /** Set on the three tabs that can be moved: dragged, or marked as the one a dragged tab lands beside. */
  movable?: boolean;
  drop?: "left" | "right";
}) {
  return (
    <button
      type="button"
      role="tab"
      id={`tab-${props.id}`}
      data-testid={`tab-${props.id}`}
      data-drop={props.drop}
      draggable={props.movable ? "true" : undefined}
      aria-selected={props.active ? "true" : "false"}
      class={`${props.active ? tabOn : tab} ${props.drop ? tabDrop[props.drop] : ""}`}
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

/**
 * The Console, History and HTTP tabs under the tree and details, and Variables during a debug run. The panel
 * keeps one height for all.
 */
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
  // Memos, so a run's status updates do not walk the tree: the tests in scope change with the tree and
  // selection only, and one list feeds both the tab's count (read twice) and the tab.
  const scope = createMemo(() => scopeTests(props.state));
  const narrowed = createMemo(() => callsOf(scope(), props.state.calls));
  const httpCount = createMemo(() => shownCalls(narrowed(), props.state.httpGroups).length);
  let section: HTMLElement | undefined;

  const [order, setOrder] = createSignal(loadTabOrder()); // read before the first paint
  const [from, setFrom] = createSignal<TabId>();
  const [over, setOver] = createSignal<TabId>();
  /** The tab an event came from, among the three that move; undefined for Variables or the gaps. */
  const tabOf = (e: Event) => TAB_IDS.find((t) => (e.target as Element).closest(`#tab-${t}`));
  const place = (id: TabId, to: number) => {
    const next = moveTab(order(), id, to);
    if (next === order()) return;
    setOrder(next);
    saveTabOrder(next);
  };
  /** The side of the tab a dragged tab is over where it will land: it takes that tab's place. */
  const mark = (id: TabId) =>
    from() && over() === id && from() !== id
      ? order().indexOf(from()!) < order().indexOf(id)
        ? "right"
        : "left"
      : undefined;
  const dragEnd = () => {
    setFrom(undefined);
    setOver(undefined);
  };
  const dragStart = (e: DragEvent) => {
    const id = tabOf(e);
    if (!id) return;
    e.dataTransfer!.effectAllowed = "move";
    setFrom(id);
  };
  const dragOver = (e: DragEvent) => {
    const id = tabOf(e);
    if (!from() || !id) return; // only a tab dragged from here, and only over the three
    e.preventDefault(); // allows the drop
    e.dataTransfer!.dropEffect = "move";
    setOver(id);
  };
  const drop = (e: DragEvent) => {
    const id = tabOf(e);
    if (from() && id) place(from()!, order().indexOf(id));
    dragEnd(); // here too: the moved button may never get its dragend
  };
  const key = (e: KeyboardEvent) => {
    const id = tabOf(e);
    if (!id || !e.altKey || e.ctrlKey || e.shiftKey || e.metaKey) return;
    if (e.key !== "ArrowLeft" && e.key !== "ArrowRight") return;
    e.preventDefault();
    place(id, order().indexOf(id) + (e.key === "ArrowLeft" ? -1 : 1));
    flush(); // a button moved in the page loses the focus: give it back once it is in its new place
    (e.target as HTMLElement).focus();
  };

  // Here, not in the Console: its content unmounts while another tab shows, and the search outlives that (R3.12).
  const search = createConsoleSearch({
    lines: () => props.state.console,
    dropped: () => props.state.consoleDropped,
    shown: () => panel() === "console",
    // the project view stays mounted under the Settings page, with visibility: hidden, which only this option sees
    visible: () => section?.checkVisibility({ visibilityProperty: true }) ?? false,
    showConsole: () => props.showPanel("console"),
  });
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
          onDragStart={dragStart}
          onDragOver={dragOver}
          onDragLeave={(e) => {
            if (!e.currentTarget.contains(e.relatedTarget as Node | null)) setOver(undefined);
          }}
          onDrop={drop}
          onDragEnd={dragEnd}
          onKeyDown={key}
        >
          <For each={order()}>
            {(id) => (
              <PanelTab
                id={id}
                icon={TABS[id].icon}
                label={TABS[id].label}
                count={
                  id === "history"
                    ? props.state.history.length
                    : id === "http"
                      ? httpCount()
                      : undefined
                }
                active={panel() === id}
                show={() => props.showPanel(id)}
                movable
                drop={mark(id)}
              />
            )}
          </For>
          <Show when={props.state.run.active && props.state.run.debug}>
            <PanelTab
              id="variables"
              icon={Braces}
              label="Variables"
              active={panel() === "variables"}
              show={() => props.showPanel("variables")}
            />
          </Show>
        </div>
        <Show when={panel() === "history"}>
          <div class="ml-auto">
            <ClearHistory state={props.state} api={props.api} />
          </div>
        </Show>
        <Show when={panel() === "console"}>
          <div class="ml-auto flex items-center gap-1">
            <ConsoleErrors search={search} />
            <Tooltip text="Find (Ctrl+F)">
              <button
                type="button"
                data-testid="find-button"
                aria-label="Find"
                class={btnIconSmall}
                onClick={() => search.openBar()}
              >
                <Icon icon={Search} size={14} />
              </button>
            </Tooltip>
          </div>
        </Show>
      </div>
      <div role="tabpanel" aria-labelledby={`tab-${panel()}`} class="flex min-h-0 flex-1 flex-col">
        <Switch>
          <Match when={panel() === "console"}>
            <Console lines={props.state.console} search={search}>
              <Banner state={props.state} />
            </Console>
          </Match>
          <Match when={panel() === "history"}>
            <History state={props.state} api={props.api} />
          </Match>
          <Match when={panel() === "variables"}>
            <Variables state={props.state} />
          </Match>
          <Match when={panel() === "http"}>
            <HttpCalls
              state={props.state}
              narrowed={narrowed()}
              selectCall={props.selectCall}
              toggleGroup={props.toggleGroup}
            />
          </Match>
        </Switch>
      </div>
    </section>
  );
}
