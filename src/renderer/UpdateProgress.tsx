import { createEffect, createMemo, Show } from "solid-js";
import type { Api } from "../shared/ipc";
import type { UpdateProgress as Progress } from "../shared/updates";
import { btnDialog } from "./buttons";
import { showModalWhile } from "./modal";
import { Notice } from "./Notice";
import type { AppState } from "./reduce";
import { dialog as dialogStyle, link } from "./ui";

const mb = (bytes: number) => Math.round(bytes / 1e6);
const runningOf = (p: Progress | null) => (p && !("error" in p) ? p : null);
const errorOf = (p: Progress | null) => (p && "error" in p ? p.error : null);

/**
 * The install's progress dialog (app-updates R8): the step under way and Cancel (none at the hand-over),
 * or, once a step failed, the message and Close. Escape does nothing while a step runs.
 */
export function UpdateProgress(props: { state: AppState; api: Api }) {
  let dialog!: HTMLDialogElement;
  let cancel: HTMLButtonElement | undefined;
  const progress = () => props.state.updateProgress;
  const running = () => runningOf(progress());
  const failure = () => errorOf(progress());
  // the last progress: its words stay while the dialog fades out (R5.2)
  const shown = createMemo<Progress | null>((last) => progress() ?? last ?? null);
  showModalWhile(
    () => dialog,
    () => progress() !== null,
    () => cancel,
  );
  // a failed hand-over brings the button back as Close, after the restart step had none (R4.6)
  createEffect(
    () => (running()?.step === "restart" ? "restart" : failure() !== null ? "failed" : "other"),
    (now, before) => {
      if (before === "restart" && now === "failed") cancel?.focus();
    },
  );

  const percent = (p: { received: number; total: number }) =>
    p.total > 0 ? Math.min(100, Math.round((p.received / p.total) * 100)) : 0;

  return (
    <dialog
      ref={(e) => (dialog = e)}
      data-testid="update-progress"
      aria-labelledby="update-progress-title"
      class={`${dialogStyle} w-[28rem] max-w-[95vw]`}
      onCancel={(e) => running() && e.preventDefault()}
      onClose={() => {
        // Chrome closes a dialog on a repeated Escape even when `cancel` was prevented
        if (dialog.open) return;
        if (running()) dialog.showModal();
        else if (failure() !== null) void props.api.cancelUpdate(); // Escape closes a failure's message
      }}
    >
      <Show when={shown()}>
        {(p) => (
          <>
            <h2 id="update-progress-title" class="text-lg font-semibold">
              {`${p().rollback ? "Rolling back to" : "Updating to"} ${p().version}`}
            </h2>
            <Show
              when={runningOf(p())}
              fallback={
                <>
                  <div class="mt-3">
                    <Notice kind="error" text={errorOf(p()) ?? ""} testid="update-error" />
                  </div>
                  <p class="mt-3 text-sm">
                    {"See the "}
                    <button
                      type="button"
                      data-testid="update-release-page"
                      class={link}
                      onClick={() => void props.api.openWhatsNew(p().version)}
                    >
                      release page
                    </button>
                    {"."}
                  </p>
                </>
              }
            >
              {(r) => (
                <>
                  {/* the run's bar (spec R14.1): the download fills it, the other steps cross it with the waiting stripe */}
                  <div
                    role="progressbar"
                    aria-labelledby="update-progress-title"
                    aria-valuemin={0}
                    aria-valuemax={100}
                    aria-valuenow={r().step === "download" ? percent(r()) : undefined}
                    class="relative mt-4 h-1.5 overflow-hidden rounded-full bg-line"
                  >
                    <Show
                      when={r().step === "download"}
                      fallback={
                        <div class="absolute inset-y-0 left-0 w-[30%] animate-slide rounded-full bg-running motion-reduce:animate-none" />
                      }
                    >
                      <div
                        class="h-full rounded-full bg-running"
                        style={{ width: `${percent(r())}%` }}
                      />
                    </Show>
                  </div>
                  <p data-testid="update-step" class="mt-2 text-sm text-muted tabular-nums">
                    {r().step === "download"
                      ? `Downloading… ${mb(r().received)} of ${mb(r().total)} MB`
                      : r().step === "check"
                        ? "Checking…"
                        : r().step === "unpack"
                          ? "Unpacking…"
                          : "Restarting Grace…"}
                  </p>
                </>
              )}
            </Show>
            {/* none while Grace hands over to the switch script: that cannot be cancelled (R8.5) */}
            <Show when={runningOf(p())?.step !== "restart"}>
              <div class="mt-5 flex justify-end">
                {/* one button, so focus stays on it when a step fails: both call cancelUpdate */}
                <button
                  type="button"
                  data-testid="update-cancel"
                  ref={(e) => (cancel = e)}
                  class={btnDialog}
                  onClick={() => void props.api.cancelUpdate()}
                >
                  {runningOf(p()) ? "Cancel" : "Close"}
                </button>
              </div>
            </Show>
          </>
        )}
      </Show>
    </dialog>
  );
}
