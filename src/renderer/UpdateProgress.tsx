import { Show } from "solid-js";
import type { Api } from "../shared/ipc";
import { btn } from "./buttons";
import { showModalWhile } from "./modal";
import type { AppState } from "./reduce";

const mb = (bytes: number) => Math.round(bytes / 1e6);

/**
 * The install's progress dialog (R8): the step under way and Cancel, or, once a step failed, the message
 * and Close. Escape does nothing while a step runs.
 */
export function UpdateProgress(props: { state: AppState; api: Api }) {
  let dialog!: HTMLDialogElement;
  let cancel: HTMLButtonElement | undefined;
  const progress = () => props.state.updateProgress;
  const running = () => {
    const p = progress();
    return p && !("error" in p) ? p : null;
  };
  const failure = () => {
    const p = progress();
    return p && "error" in p ? p.error : null;
  };
  showModalWhile(
    () => dialog,
    () => progress() !== null,
    () => cancel,
  );

  const percent = (p: { received: number; total: number }) =>
    p.total > 0 ? Math.min(100, Math.round((p.received / p.total) * 100)) : 0;

  return (
    <dialog
      ref={(e) => (dialog = e)}
      data-testid="update-progress"
      aria-labelledby="update-progress-title"
      class="m-auto w-[28rem] max-w-[95vw] rounded-lg border border-zinc-300 bg-white p-4 text-zinc-900 shadow-xl backdrop:bg-black/40 dark:border-zinc-600 dark:bg-zinc-900 dark:text-zinc-100"
      onCancel={(e) => running() && e.preventDefault()}
      onClose={() => {
        // Chrome closes a dialog on a repeated Escape even when `cancel` was prevented
        if (dialog.open) return;
        if (running()) dialog.showModal();
        else if (failure() !== null) void props.api.cancelUpdate(); // Escape closes a failure's message
      }}
    >
      <Show when={progress()}>
        {(p) => (
          <>
            <h2 id="update-progress-title" class="text-base font-semibold">
              {`${p().rollback ? "Rolling back to" : "Updating to"} ${p().version}`}
            </h2>
            <Show
              when={running()}
              fallback={
                <>
                  <p data-testid="update-error" class="mt-2 text-sm text-red-700 dark:text-red-400">
                    {failure()}
                  </p>
                  <p class="mt-1 text-sm">
                    {"See the "}
                    <button
                      type="button"
                      data-testid="update-release-page"
                      class="rounded text-sky-700 underline hover:text-sky-900 dark:text-sky-400 dark:hover:text-sky-300"
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
                  <div
                    role="progressbar"
                    aria-labelledby="update-progress-title"
                    aria-valuemin={0}
                    aria-valuemax={100}
                    aria-valuenow={r().step === "download" ? percent(r()) : undefined}
                    class="mt-3 h-2 overflow-hidden rounded bg-zinc-200 dark:bg-zinc-700"
                  >
                    <div
                      class={`h-full bg-green-600 ${r().step === "download" ? "" : "animate-pulse"}`}
                      style={{ width: r().step === "download" ? `${percent(r())}%` : "100%" }}
                    />
                  </div>
                  <p
                    data-testid="update-step"
                    class="mt-2 text-sm text-zinc-600 dark:text-zinc-400"
                  >
                    {r().step === "download"
                      ? `Downloading… ${mb(r().received)} of ${mb(r().total)} MB`
                      : r().step === "check"
                        ? "Checking…"
                        : "Unpacking…"}
                  </p>
                </>
              )}
            </Show>
            <div class="mt-4 flex justify-end">
              {/* one button, so focus stays on it when a step fails: both call cancelUpdate */}
              <button
                type="button"
                data-testid="update-cancel"
                ref={(e) => (cancel = e)}
                class={btn}
                onClick={() => void props.api.cancelUpdate()}
              >
                {running() ? "Cancel" : "Close"}
              </button>
            </div>
          </>
        )}
      </Show>
    </dialog>
  );
}
