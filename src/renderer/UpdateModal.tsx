import { Show } from "solid-js";
import type { Api } from "../shared/ipc";
import { btn, primary } from "./buttons";
import { showModalWhile } from "./modal";
import type { AppState } from "./reduce";

/**
 * The startup modal (R4): shown once when `state.updatePrompt` is set, unless an install is already
 * under way, whose progress dialog wins. Closing it any other way than a button saves nothing.
 */
export function UpdateModal(props: { state: AppState; api: Api; dismiss(): void }) {
  let dialog!: HTMLDialogElement;
  let update: HTMLButtonElement | undefined;
  let auto!: HTMLInputElement;
  showModalWhile(
    () => dialog,
    () => props.state.updatePrompt !== null && props.state.updateProgress === null,
    () => update,
  );

  const skip = (version: string) => {
    void props.api.skipVersion(version);
    props.dismiss();
  };
  const install = async (version: string) => {
    const save = auto.checked; // R4.4: only here, never with Skip
    props.dismiss(); // the progress dialog takes over
    if (save) await props.api.setUpdateSettings({ autoUpdate: true });
    await props.api.installVersion(version, false);
  };

  return (
    <dialog
      ref={(e) => (dialog = e)}
      data-testid="update-modal"
      aria-labelledby="update-modal-title"
      class="m-auto w-[28rem] max-w-[95vw] rounded-lg border border-zinc-300 bg-white p-4 text-zinc-900 shadow-xl backdrop:bg-black/40 dark:border-zinc-600 dark:bg-zinc-900 dark:text-zinc-100"
      onClose={() => dialog.open || props.dismiss()} // Escape; not a late event after a reopen
    >
      <Show when={props.state.updatePrompt}>
        {(r) => (
          <>
            <h2 id="update-modal-title" class="text-base font-semibold">
              {`Grace ${r().version} is available`}
            </h2>
            <p class="mt-2 text-sm text-zinc-600 dark:text-zinc-400">
              {`You have ${props.state.updates?.running}. Download: ${Math.round(r().size / 1e6)} MB. Your data is backed up before Grace restarts.`}{" "}
              <button
                type="button"
                data-testid="update-whats-new"
                class="rounded text-sky-700 underline hover:text-sky-900 dark:text-sky-400 dark:hover:text-sky-300"
                onClick={() => void props.api.openWhatsNew(r().version)}
              >
                What's new
              </button>
            </p>
            <label class="mt-3 flex items-center gap-2 text-sm">
              <input type="checkbox" data-testid="update-auto" ref={(e) => (auto = e)} />
              Update automatically at startup from now on
            </label>
            <div class="mt-4 flex justify-end gap-2">
              <button
                type="button"
                data-testid="update-skip"
                class={btn}
                onClick={() => skip(r().version)}
              >
                Skip this version
              </button>
              <button
                type="button"
                data-testid="update-install"
                ref={(e) => (update = e)}
                class={primary}
                onClick={() => void install(r().version)}
              >
                Update and restart
              </button>
            </div>
          </>
        )}
      </Show>
    </dialog>
  );
}
