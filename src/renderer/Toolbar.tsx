import type { Api } from "../shared/ipc";
import type { AppState } from "./reduce";
import { matchingLeaves } from "../shared/tags";
import { btn, primary } from "./buttons";
import { EnvPicker } from "./EnvPicker";
import { PropertiesDialog } from "./PropertiesDialog";

const LAYOUT_LABEL = { maven: "Maven", gradle: "Gradle", standalone: "Standalone" } as const;

export function Toolbar(props: { state: AppState; api: Api }) {
  const active = () => props.state.run.active;
  const busy = () => active() || props.state.busy;
  const canRunAll = () =>
    !busy() &&
    !!props.state.tree &&
    matchingLeaves(props.state.tree, props.state.filter).length > 0;
  const newest = () => props.state.history[0]; // main sends the newest run first
  const failedCount = () => newest()?.failed ?? 0;
  return (
    <header class="flex items-center gap-2 border-b border-zinc-200 px-3 py-2 dark:border-zinc-700">
      <div class="flex min-w-0 flex-1 items-baseline gap-2">
        <span
          data-testid="project-path"
          class="truncate font-mono text-sm"
          title={props.state.project?.root}
        >
          {props.state.project?.root ?? "No project"}
        </span>
        <span
          data-testid="project-layout"
          class="shrink-0 rounded bg-zinc-200 px-1.5 text-xs dark:bg-zinc-700"
        >
          {props.state.project ? LAYOUT_LABEL[props.state.project.layout] : ""}
        </span>
      </div>
      <EnvPicker state={props.state} api={props.api} />
      <PropertiesDialog state={props.state} api={props.api} />
      <button
        data-testid="open-button"
        class={btn}
        disabled={busy()}
        onClick={() => void props.api.openProject()}
      >
        Open…
      </button>
      <button
        data-testid="refresh-button"
        class={btn}
        disabled={busy()}
        onClick={() => void props.api.refresh()}
      >
        Refresh
      </button>
      <button
        data-testid="run-all-button"
        class={primary}
        disabled={!canRunAll()}
        onClick={() => void props.api.run(props.state.tree!.key)}
      >
        Run all
      </button>
      <button
        data-testid="rerun-failed-button"
        class={btn}
        disabled={busy() || failedCount() === 0}
        onClick={() => void props.api.rerunFailed(newest()!.id)}
      >
        {`Re-run failed (${failedCount()})`}
      </button>
      <button
        data-testid="cancel-button"
        class={btn}
        disabled={!busy()}
        onClick={() => void props.api.cancel()}
      >
        Cancel
      </button>
      <button
        data-testid="report-button"
        class={btn}
        disabled={busy() || !props.state.run.reportAvailable}
        onClick={() => void props.api.openReport()}
      >
        Report
      </button>
    </header>
  );
}
