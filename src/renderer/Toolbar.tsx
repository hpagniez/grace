import { FileText, FolderOpen, Play, RefreshCw, RotateCcw, Square } from "lucide";
import { Show } from "solid-js";
import type { Api } from "../shared/ipc";
import type { AppState } from "./reduce";
import { matchingLeaves } from "../shared/tags";
// a file, never a data: URL, which the CSP (default-src 'self') would block
import logo from "../../assets/icon-small.svg?no-inline";
import { btn, btnIcon, narrowIcon, narrowLabel, primary } from "./buttons";
import { EnvPicker } from "./EnvPicker";
import { Icon } from "./Icon";
import { createNarrow } from "./narrow";
import { PropertiesDialog } from "./PropertiesDialog";
import { Threads } from "./Threads";
import { Tooltip } from "./Tooltip";
import { countPill } from "./ui";

const LAYOUT_LABEL = { maven: "Maven", gradle: "Gradle", standalone: "Standalone" } as const;

const folderName = (path: string) => path.split(/[\\/]/).filter(Boolean).pop() ?? path;

/** Between the groups of buttons (spec R13.1). */
const Divider = () => <span aria-hidden="true" class="h-5 w-px shrink-0 bg-line" />;

export function Toolbar(props: { state: AppState; api: Api }) {
  const active = () => props.state.run.active;
  const busy = () => active() || props.state.busy;
  const canRunAll = () =>
    !busy() &&
    !!props.state.tree &&
    matchingLeaves(props.state.tree, props.state.filter).length > 0;
  const path = () => props.state.project?.root ?? props.state.opening ?? "No project";
  const newest = () => props.state.history[0]; // main sends the newest run first
  const failedCount = () => newest()?.failed ?? 0;
  // a button whose label is hidden gets its label as a tooltip; a visible label needs none
  const narrow = createNarrow();
  const spoken = (label: string) => (narrow() ? label : undefined);
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
      {/* a container: below 1280 px the layout chip goes when the name and the chip no longer fit side by side */}
      <div class="@container flex min-w-0 flex-1 items-center gap-2 pr-2">
        <Tooltip text={path()} class="min-w-0 shrink-100000">
          <span data-testid="project-name" class="truncate text-base font-medium">
            {folderName(path())}
          </span>
        </Tooltip>
        <span
          data-testid="project-layout"
          class="h-5 min-w-0 truncate rounded-sm bg-selected px-1.5 text-2xs leading-5 font-medium text-muted empty:hidden max-[1280px]:@max-[160px]:hidden"
        >
          {props.state.project ? LAYOUT_LABEL[props.state.project.layout] : ""}
        </span>
        <Tooltip text={path()} class="min-w-0 shrink-1000000">
          {/* rtl cuts the start of a long path; the ltr span inside keeps its characters in order. squeezed, the path gives way first (shrink-1000000), then the name (shrink-100000), then the chip (shrink 1): the factors are far apart so a chip that fits loses no fraction of a pixel, which would already show an ellipsis */}
          <span
            data-testid="project-path"
            class="min-w-0 truncate font-mono text-xs text-muted"
            style="direction:rtl;text-align:left"
          >
            <span dir="ltr">{path()}</span>
          </span>
        </Tooltip>
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
      <button
        data-testid="run-all-button"
        class={primary}
        disabled={!canRunAll()}
        onClick={() => void props.api.run(props.state.tree!.key)}
      >
        <Icon icon={Play} class="text-on-primary" />
        Run all
      </button>
    </header>
  );
}
