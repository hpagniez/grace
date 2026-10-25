import { createSignal, flush, For, Show } from "solid-js";
import { resultLabel, whatRanLabel, whenLabel, type StoredSelection } from "../shared/history";
import type { Api } from "../shared/ipc";
import { filterOn } from "../shared/tags";
import { btn, btnSmall, danger } from "./buttons";
import type { AppState } from "./reduce";

/** "18s", or "1m 12s" from a minute on; a run under one second is "0s". */
export function durationLabel(millis: number): string {
  const s = Math.floor(millis / 1000);
  return s >= 60 ? `${Math.floor(s / 60)}m ${String(s % 60).padStart(2, "0")}s` : `${s}s`;
}

/** The tooltip of "What ran": where the selected node sits. Nothing for a Re-run failed run. */
const selectionPath = (s: StoredSelection | null): string | undefined => {
  if (!s) return undefined;
  const path = s.kind === "folder" || s.kind === "feature" ? s.file : `${s.file} › ${s.name}`;
  return path === "" ? undefined : path;
};

const chip = "rounded border px-1.5 text-xs";
const INCLUDE = `${chip} border-green-600 bg-green-100 text-green-800 dark:bg-green-900/40 dark:text-green-300`;
const EXCLUDE = `${chip} border-red-600 bg-red-100 text-red-800 dark:bg-red-900/40 dark:text-red-300`;
const th = "sticky top-0 whitespace-nowrap bg-zinc-100 px-3 py-1 font-medium dark:bg-zinc-800";
const td = "whitespace-nowrap px-3 py-1";

/** The "Clear history" button of the tab header and the dialog that confirms it. */
export function ClearHistory(props: { state: AppState; api: Api }) {
  let dialog!: HTMLDialogElement;
  const [open, setOpen] = createSignal(false);
  const locked = () => props.state.run.active || props.state.busy;
  const show = () => {
    setOpen(true);
    flush(); // the dialog's content must exist for showModal to focus Cancel
    dialog.showModal();
  };
  const close = () => {
    dialog.close();
    setOpen(false);
  };
  return (
    <>
      <button
        type="button"
        data-testid="clear-history"
        class={btnSmall}
        disabled={locked() || props.state.history.length === 0}
        onClick={show}
      >
        Clear history
      </button>
      <dialog
        ref={(e) => (dialog = e)}
        data-testid="clear-history-dialog"
        aria-labelledby="clear-history-title"
        class="m-auto w-[28rem] max-w-[95vw] rounded-lg border border-zinc-300 bg-white p-4 text-zinc-900 shadow-xl backdrop:bg-black/40 dark:border-zinc-600 dark:bg-zinc-900 dark:text-zinc-100"
        onClose={() => setOpen(false)} // Escape
      >
        <Show when={open()}>
          <p id="clear-history-title" class="text-sm">
            {`Delete the ${props.state.history.length} runs of this project and their reports?`}
          </p>
          <div class="mt-4 flex justify-end gap-2">
            <button
              type="button"
              data-testid="clear-history-confirm"
              class={danger}
              onClick={() => {
                close();
                void props.api.clearHistory();
              }}
            >
              Delete
            </button>
            <button
              type="button"
              data-testid="clear-history-cancel"
              class={btn}
              autofocus
              onClick={close}
            >
              Cancel
            </button>
          </div>
        </Show>
      </dialog>
    </>
  );
}

/** The project's listed runs, newest first, as main sends them. */
export function History(props: { state: AppState; api: Api }) {
  const locked = () => props.state.run.active || props.state.busy;
  return (
    <div data-testid="history" class="min-h-0 flex-1 overflow-auto">
      <Show
        when={props.state.history.length > 0}
        fallback={
          <p data-testid="history-empty" class="px-3 py-2 text-sm text-zinc-600 dark:text-zinc-400">
            No runs yet
          </p>
        }
      >
        <table class="w-full text-left text-xs">
          <thead>
            <tr>
              <th class={th}>When</th>
              <th class={th}>Env</th>
              <th class={th}>Tags</th>
              <th class={th}>What ran</th>
              <th class={th}>Result</th>
              <th class={th}>Duration</th>
              <th class={th}>
                <span class="sr-only">Actions</span>
              </th>
            </tr>
          </thead>
          <tbody>
            <For each={props.state.history}>
              {(run) => {
                const now = Date.now(); // one clock for every label of the row
                return (
                  <tr
                    data-testid="history-row"
                    data-id={run.id}
                    class="border-b border-zinc-100 dark:border-zinc-800"
                  >
                    <td data-testid="history-when" class={td}>
                      {whenLabel(run.startedAt, now)}
                    </td>
                    <td data-testid="history-env" class={td}>
                      {run.env ?? "—"}
                    </td>
                    <td data-testid="history-tags" class={td}>
                      <Show when={filterOn(run.filter)} fallback="—">
                        <span class="flex gap-1">
                          <For each={run.filter.include}>
                            {(tag) => (
                              <span data-testid="history-tag" class={INCLUDE}>
                                {`@${tag}`}
                              </span>
                            )}
                          </For>
                          <For each={run.filter.exclude}>
                            {(tag) => (
                              <span data-testid="history-tag" class={EXCLUDE}>
                                {`not @${tag}`}
                              </span>
                            )}
                          </For>
                        </span>
                      </Show>
                    </td>
                    <td
                      data-testid="history-what"
                      title={selectionPath(run.selection)}
                      class={`${td} max-w-56 truncate`}
                    >
                      {whatRanLabel(run, now)}
                    </td>
                    <td
                      data-testid="history-result"
                      class={`${td} ${run.outcome === "failed" || run.failed > 0 ? "text-red-700 dark:text-red-400" : ""}`}
                    >
                      {resultLabel(run)}
                    </td>
                    <td data-testid="history-duration" class={`${td} font-mono`}>
                      {durationLabel(run.durationMillis)}
                    </td>
                    <td class={td}>
                      <span class="flex gap-1">
                        <button
                          type="button"
                          data-testid="history-report"
                          class={btnSmall}
                          disabled={locked() || !run.reportAvailable}
                          onClick={() => void props.api.openRunReport(run.id)}
                        >
                          Report
                        </button>
                        <button
                          type="button"
                          data-testid="history-rerun"
                          class={btnSmall}
                          disabled={locked()}
                          onClick={() => void props.api.rerun(run.id)}
                        >
                          Re-run
                        </button>
                        <button
                          type="button"
                          data-testid="history-rerun-failed"
                          class={btnSmall}
                          disabled={locked() || run.failed === 0}
                          onClick={() => void props.api.rerunFailed(run.id)}
                        >
                          Re-run failed
                        </button>
                      </span>
                    </td>
                  </tr>
                );
              }}
            </For>
          </tbody>
        </table>
      </Show>
    </div>
  );
}
