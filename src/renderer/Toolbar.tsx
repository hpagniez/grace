import type { Api } from "../shared/ipc";
import type { AppState } from "./reduce";
import { leaves } from "./nodes";

const LAYOUT_LABEL = { maven: "Maven", gradle: "Gradle", standalone: "Standalone" } as const;

const base = "rounded border px-3 py-1 text-sm disabled:cursor-not-allowed disabled:opacity-40";
const btn = `${base} border-zinc-300 bg-white hover:bg-zinc-100 dark:border-zinc-600 dark:bg-zinc-800 dark:hover:bg-zinc-700`;
const primary = `${base} border-green-700 bg-green-600 text-white hover:bg-green-700`;

export function Toolbar(props: { state: AppState; api: Api }) {
  const active = () => props.state.run.active;
  const busy = () => active() || props.state.busy;
  const canRunAll = () => !busy() && !!props.state.tree && leaves(props.state.tree).length > 0;
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
        disabled={active() || !props.state.run.reportAvailable}
        onClick={() => void props.api.openReport()}
      >
        Report
      </button>
    </header>
  );
}
