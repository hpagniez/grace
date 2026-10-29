import { Check, FileText, History as HistoryIcon, Minus, Play, RotateCcw } from "lucide";
import { createSignal, For, Show } from "solid-js";
import {
  resultLabel,
  whatRanLabel,
  whenLabel,
  type HistoryEntry,
  type StoredSelection,
} from "../shared/history";
import type { Api } from "../shared/ipc";
import { filterOn } from "../shared/tags";
import { btn, btnSmall, danger, ghostSmall } from "./buttons";
import { EmptyState } from "./EmptyState";
import { Icon } from "./Icon";
import type { AppState } from "./reduce";
import { StatusMark, type MarkStatus } from "./StatusMark";
import { Tooltip } from "./Tooltip";
import { chipExcluded, chipIncluded, dialog as dialogStyle } from "./ui";

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

/** The mark of a run's result: a failure beats a cancel (R7.1); the words next to it say which. */
const resultStatus = (r: HistoryEntry): MarkStatus =>
  r.outcome === "failed" || r.failed > 0
    ? "failed"
    : r.outcome === "cancelled"
      ? "cancelled"
      : "passed";

// Headers stick to the top of the scroll (R18.4). Rows are 28 px. The row's scroll-mt-7 clears the header when the row itself
// is scrolled into view; a focused button inside it is cleared by the scroller's scroll-pt-7 (R10.2).
const th =
  "sticky top-0 h-7 whitespace-nowrap bg-surface px-3 text-xs font-medium text-muted shadow-[inset_0_-1px_0_var(--color-line)]";
const td = "whitespace-nowrap px-3";
const num = "text-right tabular-nums";

/** The "Clear history" button of the tab header and the dialog that confirms it. */
export function ClearHistory(props: { state: AppState; api: Api }) {
  let dialog!: HTMLDialogElement;
  // The count the dialog was opened with: main's empty list, arriving while it fades out, must not change its words.
  const [count, setCount] = createSignal(0);
  const locked = () => props.state.run.active || props.state.busy;
  const show = () => {
    setCount(props.state.history.length);
    dialog.showModal();
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
        class={`${dialogStyle} w-[28rem] max-w-[95vw]`}
      >
        {/* Always mounted: the title and buttons stay while the dialog fades out (R5.2). */}
        <p id="clear-history-title" class="text-lg font-semibold">
          {count() === 1
            ? "Delete the 1 run of this project and its report?"
            : `Delete the ${count()} runs of this project and their reports?`}
        </p>
        <div class="mt-5 flex justify-end gap-2">
          {/* text-base! (R2.2): the buttons' own text-sm comes later in the stylesheet */}
          <button
            type="button"
            data-testid="clear-history-confirm"
            class={`${danger} text-base!`}
            onClick={() => {
              dialog.close();
              void props.api.clearHistory();
            }}
          >
            Delete
          </button>
          <button
            type="button"
            data-testid="clear-history-cancel"
            class={`${btn} text-base!`}
            autofocus
            onClick={() => dialog.close()}
          >
            Cancel
          </button>
        </div>
      </dialog>
    </>
  );
}

/** The project's listed runs, newest first, as main sends them. */
export function History(props: { state: AppState; api: Api }) {
  const locked = () => props.state.run.active || props.state.busy;
  return (
    // scroll-pt-7: a button focused under the 28 px sticky header scrolls clear of it (R10.2)
    <div data-testid="history" class="min-h-0 flex-1 scroll-pt-7 overflow-auto">
      <Show
        when={props.state.history.length > 0}
        fallback={
          <EmptyState
            icon={HistoryIcon}
            text="No runs yet. Runs appear here when they finish."
            testid="history-empty"
          />
        }
      >
        <table class="w-full text-left text-sm">
          <thead>
            <tr>
              <th class={th}>When</th>
              <th class={th}>Env</th>
              <th class={`${th} text-right`}>Threads</th>
              <th class={th}>Tags</th>
              <th class={th}>What ran</th>
              <th class={th}>Result</th>
              <th class={`${th} text-right`}>Duration</th>
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
                    class="h-7 scroll-mt-7 border-b border-line"
                  >
                    <td data-testid="history-when" class={`${td} tabular-nums`}>
                      {whenLabel(run.startedAt, now)}
                    </td>
                    <td data-testid="history-env" class={td}>
                      {run.env ?? "—"}
                    </td>
                    <td data-testid="history-threads" class={`${td} ${num}`}>
                      {run.threads ?? "—"}
                    </td>
                    <td data-testid="history-tags" class={td}>
                      <Show when={filterOn(run.filter)} fallback="—">
                        <span class="flex gap-1">
                          <For each={run.filter.include}>
                            {(tag) => (
                              <span data-testid="history-tag" class={chipIncluded}>
                                <Icon icon={Check} size={14} />
                                {`@${tag}`}
                              </span>
                            )}
                          </For>
                          <For each={run.filter.exclude}>
                            {(tag) => (
                              <span data-testid="history-tag" class={chipExcluded}>
                                <Icon icon={Minus} size={14} />
                                {`not @${tag}`}
                              </span>
                            )}
                          </For>
                        </span>
                      </Show>
                    </td>
                    <td class={td}>
                      {/* the path adds to the label for a scenario, and shows a cut one whole */}
                      <Tooltip text={selectionPath(run.selection)} class="min-w-0 max-w-56">
                        <span data-testid="history-what" class="min-w-0 truncate">
                          {whatRanLabel(run, now)}
                        </span>
                      </Tooltip>
                    </td>
                    <td data-testid="history-result" class={`${td} tabular-nums`}>
                      {/* a block flex, not inline-flex: baseline alignment would lift the words above their row */}
                      <div class="flex items-center gap-1.5">
                        <StatusMark status={resultStatus(run)} decorative />
                        {resultLabel(run)}
                      </div>
                    </td>
                    <td data-testid="history-duration" class={`${td} ${num} font-mono`}>
                      {durationLabel(run.durationMillis)}
                    </td>
                    <td class={td}>
                      <span class="flex gap-1">
                        <button
                          type="button"
                          data-testid="history-report"
                          class={ghostSmall}
                          disabled={locked() || !run.reportAvailable}
                          onClick={() => void props.api.openRunReport(run.id)}
                        >
                          <Icon icon={FileText} size={14} />
                          Report
                        </button>
                        <button
                          type="button"
                          data-testid="history-rerun"
                          class={ghostSmall}
                          disabled={locked()}
                          onClick={() => void props.api.rerun(run.id)}
                        >
                          <Icon icon={Play} size={14} />
                          Re-run
                        </button>
                        <button
                          type="button"
                          data-testid="history-rerun-failed"
                          class={ghostSmall}
                          disabled={locked() || run.failed === 0}
                          onClick={() => void props.api.rerunFailed(run.id)}
                        >
                          <Icon icon={RotateCcw} size={14} />
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
