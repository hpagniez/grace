import {
  ArrowDownToDot,
  FileText,
  FolderOpen,
  Pause,
  Play,
  RedoDot,
  RefreshCw,
  RotateCcw,
  Square,
  type IconNode,
} from "lucide";
import { createEffect, For, Show } from "solid-js";
import type { DebugCommand } from "../shared/events";
import type { Api } from "../shared/ipc";
import type { AppState } from "./reduce";
import { matchingLeaves } from "../shared/tags";
// a file, never a data: URL, which the CSP (default-src 'self') would block
import logo from "../../assets/icon-small.svg?no-inline";
import { btn, btnIcon, narrowIcon, narrowLabel, primary } from "./buttons";
import { Cut, Middle } from "./Cut";
import { EnvPicker } from "./EnvPicker";
import { Icon } from "./Icon";
import { createNarrow } from "./narrow";
import { PropertiesDialog } from "./PropertiesDialog";
import { Threads } from "./Threads";
import { Tooltip } from "./Tooltip";
import { countPill } from "./ui";

const LAYOUT_LABEL = { maven: "Maven", gradle: "Gradle", standalone: "Standalone" } as const;

const folderName = (path: string) => path.split(/[\\/]/).filter(Boolean).pop() ?? path;
/** A cut name keeps this many characters of its end in sight; they fit the 760 px window. */
export const NAME_TAIL = 12;

/** The step debugger's controls while paused, in order, with their keys (step debugger R6.3). */
const STEPS: { command: DebugCommand; label: string; icon: IconNode; key: string; testid: string }[] = [
  { command: "continue", label: "Continue", icon: Play, key: "F5", testid: "debug-continue" },
  { command: "over", label: "Step over", icon: RedoDot, key: "F10", testid: "debug-over" },
  { command: "into", label: "Step into", icon: ArrowDownToDot, key: "F11", testid: "debug-into" },
];

/** Between the groups of buttons (spec R13.1). */
const Divider = () => <span aria-hidden="true" class="h-5 w-px shrink-0 bg-line" />;

export function Toolbar(props: { state: AppState; api: Api }) {
  const active = () => props.state.run.active;
  const busy = () => active() || props.state.busy;
  const canRunAll = () =>
    !busy() &&
    !!props.state.tree &&
    matchingLeaves(props.state.tree, props.state.filter).length > 0;
  const name = () => folderName(props.state.project?.root ?? props.state.opening ?? "No project");
  const newest = () => props.state.history[0]; // main sends the newest run first
  const failedCount = () => newest()?.failed ?? 0;
  // a button whose label is hidden gets its label as a tooltip; a visible label needs none
  const narrow = createNarrow();
  const spoken = (label: string) => (narrow() ? label : undefined);
  const debugging = () => active() && props.state.run.debug;
  // F5, F10 and F11 anywhere in the window, only while paused: no listener otherwise
  createEffect(
    () => !!props.state.paused,
    (paused) => {
      if (!paused) return;
      const onKey = (e: KeyboardEvent) => {
        const step = STEPS.find((s) => s.key === e.key);
        if (!step) return;
        e.preventDefault();
        void props.api.debugCommand(step.command);
      };
      window.addEventListener("keydown", onKey);
      return () => window.removeEventListener("keydown", onKey);
    },
  );
  return (
    <header class="flex min-h-11 items-center gap-2 bg-ground px-3">
      <Tooltip text="Home">
        <button
          data-testid="home-button"
          class={btnIcon}
          aria-label="Home"
          disabled={busy()}
          onClick={() => void props.api.closeProject()}
        >
          <img src={logo} alt="" width="20" height="20" />
        </button>
      </Tooltip>
      {/* one 20 px line that wraps and hides the second: once the name and the chip no longer fit side by side, the chip goes there, whole, and only then is the name cut. The path is in the status bar. */}
      <div class="flex h-5 min-w-0 flex-1 flex-wrap items-center gap-x-2 overflow-hidden pr-2">
        <Cut full={name()} class="flex text-base font-medium" testid="project-name">
          <Middle head={name().slice(0, -NAME_TAIL)} tail={name().slice(-NAME_TAIL)} />
        </Cut>
        <span
          data-testid="project-layout"
          class="h-5 shrink-0 rounded-sm bg-selected px-1.5 text-2xs leading-5 font-medium text-muted empty:hidden"
        >
          {props.state.project ? LAYOUT_LABEL[props.state.project.layout] : ""}
        </span>
      </div>
      <EnvPicker state={props.state} api={props.api} />
      <Threads state={props.state} api={props.api} />
      <PropertiesDialog state={props.state} api={props.api} />
      <Divider />
      <Tooltip text="Refresh">
        <button
          data-testid="refresh-button"
          class={btnIcon}
          aria-label="Refresh"
          disabled={busy()}
          onClick={() => void props.api.refresh()}
        >
          <Icon icon={RefreshCw} />
        </button>
      </Tooltip>
      <Tooltip text={spoken("Open…")}>
        <button
          data-testid="open-button"
          class={`${btn} ${narrowIcon}`}
          disabled={busy()}
          onClick={() => void props.api.openProject()}
        >
          <Icon icon={FolderOpen} />
          <span class={narrowLabel}>Open…</span>
        </button>
      </Tooltip>
      <Divider />
      <Tooltip text={spoken("Report")}>
        <button
          data-testid="report-button"
          class={`${btn} ${narrowIcon}`}
          disabled={busy() || !props.state.run.reportAvailable}
          onClick={() => void props.api.openReport()}
        >
          <Icon icon={FileText} />
          <span class={narrowLabel}>Report</span>
        </button>
      </Tooltip>
      <Tooltip text={spoken("Re-run failed")}>
        <button
          data-testid="rerun-failed-button"
          class={`${btn} max-[1280px]:px-2`}
          disabled={busy() || failedCount() === 0}
          onClick={() => void props.api.rerunFailed(newest()!.id)}
        >
          <Icon icon={RotateCcw} />
          <span class={narrowLabel}>Re-run failed</span>
          {/* a 0 on a disabled button says nothing: the count shows once there is one */}
          <Show when={failedCount() > 0}>
            {" "}
            <span class={countPill}>{failedCount()}</span>
          </Show>
        </button>
      </Tooltip>
      <Tooltip text={spoken("Cancel")}>
        <button
          data-testid="cancel-button"
          class={`${btn} ${narrowIcon}`}
          disabled={!busy()}
          onClick={() => void props.api.cancel()}
        >
          <Icon icon={Square} />
          <span class={narrowLabel}>Cancel</span>
        </button>
      </Tooltip>
      <Show
        when={debugging()}
        fallback={
          <button
            data-testid="run-all-button"
            class={primary}
            disabled={!canRunAll()}
            onClick={() => void props.api.run(props.state.tree!.key)}
          >
            <Icon icon={Play} class="text-on-primary" />
            Run all
          </button>
        }
      >
        <Show
          when={props.state.paused}
          fallback={
            <Tooltip text={spoken("Pause")}>
              <button
                data-testid="debug-pause"
                class={`${btn} ${narrowIcon}`}
                onClick={() => void props.api.debugCommand("pause")}
              >
                <Icon icon={Pause} />
                <span class={narrowLabel}>Pause</span>
              </button>
            </Tooltip>
          }
        >
          <For each={STEPS}>
            {(s) => (
              <Tooltip text={spoken(s.label)}>
                <button
                  data-testid={s.testid}
                  aria-keyshortcuts={s.key}
                  class={`${s.command === "continue" ? primary : btn} ${narrowIcon}`}
                  onClick={() => void props.api.debugCommand(s.command)}
                >
                  <Icon icon={s.icon} class={s.command === "continue" ? "text-on-primary" : undefined} />
                  <span class={narrowLabel}>{s.label}</span>
                </button>
              </Tooltip>
            )}
          </For>
        </Show>
      </Show>
    </header>
  );
}
