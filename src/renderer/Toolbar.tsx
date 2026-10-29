import { FileText, FolderOpen, Play, RefreshCw, RotateCcw, Square } from "lucide";
import type { Api } from "../shared/ipc";
import type { AppState } from "./reduce";
import { matchingLeaves } from "../shared/tags";
// a file, never a data: URL, which the CSP (default-src 'self') would block
import logo from "../../assets/icon-small.svg?no-inline";
import { btn, btnIcon, primary } from "./buttons";
import { EnvPicker } from "./EnvPicker";
import { Icon } from "./Icon";
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
      <div class="flex min-w-0 flex-1 items-center gap-2 pr-2">
        <Tooltip text={path()} class="min-w-0">
          <span data-testid="project-name" class="truncate text-base font-medium">
            {folderName(path())}
          </span>
        </Tooltip>
        <span
          data-testid="project-layout"
          class="inline-flex h-5 shrink-0 items-center rounded-sm bg-selected px-1.5 text-2xs font-medium text-muted empty:hidden"
        >
          {props.state.project ? LAYOUT_LABEL[props.state.project.layout] : ""}
        </span>
        <Tooltip text={path()} class="min-w-0 shrink-100">
          {/* rtl cuts the start of a long path; the ltr span inside keeps its characters in order. shrink-100: the path gives way before the name does */}
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
      <button
        data-testid="open-button"
        class={btn}
        disabled={busy()}
        onClick={() => void props.api.openProject()}
      >
        <Icon icon={FolderOpen} />
        Open…
      </button>
      <Divider />
      <button
        data-testid="report-button"
        class={btn}
        disabled={busy() || !props.state.run.reportAvailable}
        onClick={() => void props.api.openReport()}
      >
        <Icon icon={FileText} />
        Report
      </button>
      <button
        data-testid="rerun-failed-button"
        class={btn}
        disabled={busy() || failedCount() === 0}
        onClick={() => void props.api.rerunFailed(newest()!.id)}
      >
        <Icon icon={RotateCcw} />
        Re-run failed <span class={countPill}>{failedCount()}</span>
      </button>
      <button
        data-testid="cancel-button"
        class={btn}
        disabled={!busy()}
        onClick={() => void props.api.cancel()}
      >
        <Icon icon={Square} />
        Cancel
      </button>
      <button
        data-testid="run-all-button"
        class={primary}
        disabled={!canRunAll()}
        onClick={() => void props.api.run(props.state.tree!.key)}
      >
        <Icon icon={Play} class="text-play" />
        Run all
      </button>
    </header>
  );
}
