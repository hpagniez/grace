import { createMemo, Show } from "solid-js";
import type { Api } from "../shared/ipc";
import type { Release } from "../shared/updates";
import { btnDialog, primaryDialog } from "./buttons";
import { showModalWhile } from "./modal";
import type { AppState } from "./reduce";
import { dialog as dialogStyle, link } from "./ui";

/**
 * The startup modal (app-updates R4): shown once when `state.updatePrompt` is set, unless an install is
 * already under way, whose progress dialog wins. Closing it any other way than a button saves nothing.
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
  // The last prompt: its words stay while the dialog fades out (R5.2). Keyed below, so a new prompt
  // draws afresh, with the box unticked.
  const shown = createMemo<Release | null>((last) => props.state.updatePrompt ?? last ?? null);

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
      class={`${dialogStyle} w-[28rem] max-w-[95vw]`}
      onClose={() => dialog.open || props.dismiss()} // Escape; not a late event after a reopen
    >
      <Show when={shown()} keyed>
        {(r) => (
          <>
            <h2 id="update-modal-title" class="text-lg font-semibold">
              {`Grace ${r.version} is available`}
            </h2>
            <p class="mt-2 text-sm">
              {`You have ${props.state.updates?.running}. Download: ${Math.round(r.size / 1e6)} MB. Your data is backed up before Grace restarts.`}{" "}
              <button
                type="button"
                data-testid="update-whats-new"
                class={link}
                onClick={() => void props.api.openWhatsNew(r.version)}
              >
                What's new
              </button>
            </p>
            <label class="mt-4 flex min-h-6 items-center gap-2 text-sm">
              <input
                type="checkbox"
                data-testid="update-auto"
                class="accent-accent"
                ref={(e) => (auto = e)}
              />
              Update automatically at startup from now on
            </label>
            {/* the action first, the other choice last (spec R19.1) */}
            <div class="mt-5 flex justify-end gap-2">
              <button
                type="button"
                data-testid="update-install"
                ref={(e) => (update = e)}
                class={primaryDialog}
                onClick={() => void install(r.version)}
              >
                Update and restart
              </button>
              <button
                type="button"
                data-testid="update-skip"
                class={btnDialog}
                onClick={() => skip(r.version)}
              >
                Skip this version
              </button>
            </div>
          </>
        )}
      </Show>
    </dialog>
  );
}
