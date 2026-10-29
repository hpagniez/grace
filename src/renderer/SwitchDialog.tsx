import { createSignal, Show } from "solid-js";
import { compareVersions } from "../shared/version";
import type { Backup, Release } from "../shared/updates";
import { btn, primary } from "./buttons";
import { showModalWhile } from "./modal";

const pad = (n: number) => String(n).padStart(2, "0");
/** `YYYY-MM-DD`, the local date of an ISO time. */
const localDate = (iso: string) => {
  const d = new Date(iso);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
};

/**
 * Asks before switching to `target` from the Versions table (R7): a rollback explains what happens to
 * the data and warns; an install of a newer version that has a saved copy asks which data to use.
 * Open while it is rendered: render it only while there is a question.
 */
export function SwitchDialog(props: {
  target: Release;
  running: string;
  backup: Backup | null;
  onConfirm(useSavedData: boolean): void;
  onCancel(): void;
}) {
  let dialog!: HTMLDialogElement;
  let cancel: HTMLButtonElement | undefined;
  let confirm: HTMLButtonElement | undefined;
  const [useSaved, setUseSaved] = createSignal(false);
  const rollback = () => compareVersions(props.target.version, props.running) < 0;
  const v = () => props.target.version;
  const date = () => (props.backup ? localDate(props.backup.savedAt) : "");
  // a rollback focuses Cancel (R7.5); an install keeps the current data until another choice is made
  showModalWhile(
    () => dialog,
    () => true,
    () => (rollback() ? cancel : confirm),
  );

  const warning = () =>
    "Going back to an older version may break your data." +
    (props.backup
      ? ` Runs, settings and secrets added since then are not in the restored copy; they stay in the backup of ${props.running}, and come back if you install ${props.running} again and use its saved data.`
      : "");

  return (
    <dialog
      ref={(e) => (dialog = e)}
      data-testid="switch-dialog"
      aria-labelledby="switch-dialog-title"
      class="m-auto w-[28rem] max-w-[95vw] rounded-lg border border-zinc-300 bg-white p-4 text-zinc-900 shadow-xl backdrop:bg-black/40 dark:border-zinc-600 dark:bg-zinc-900 dark:text-zinc-100"
      onClose={() => dialog.open || props.onCancel()} // Escape; not a late event after a reopen
    >
      <h2 id="switch-dialog-title" class="text-base font-semibold">
        {rollback() ? `Roll back to ${v()}?` : `Install ${v()}?`}
      </h2>
      <Show
        when={rollback()}
        fallback={
          <div class="mt-3 flex flex-col gap-2 text-sm">
            <label class="flex items-center gap-2">
              <input
                type="radio"
                name="switch-data"
                checked={useSaved()}
                onChange={() => setUseSaved(true)}
              />
              {`Use the data you had in ${v()} (${date()})`}
            </label>
            <label class="flex items-center gap-2">
              <input
                type="radio"
                name="switch-data"
                checked={!useSaved()}
                onChange={() => setUseSaved(false)}
              />
              Keep your current data
            </label>
          </div>
        }
      >
        <p class="mt-2 text-sm text-zinc-600 dark:text-zinc-400">
          {props.backup
            ? `Grace backs up your current data, then restores the data saved when you last used ${v()} (${date()}).`
            : `Grace backs up your current data. ${v()} has no saved data on this computer, so it starts from your current data, which it may not read correctly.`}
        </p>
        <p
          data-testid="switch-warning"
          class="mt-3 rounded border border-amber-300 bg-amber-50 px-3 py-2 text-sm text-amber-900 dark:border-amber-700 dark:bg-amber-950 dark:text-amber-200"
        >
          <span aria-hidden="true">⚠</span> {warning()}
        </p>
      </Show>
      <div class="mt-4 flex justify-end gap-2">
        <button
          type="button"
          data-testid="switch-cancel"
          ref={(e) => (cancel = e)}
          class={btn}
          onClick={() => props.onCancel()}
        >
          Cancel
        </button>
        <button
          type="button"
          data-testid="switch-confirm"
          ref={(e) => (confirm = e)}
          class={primary}
          onClick={() => props.onConfirm(rollback() ? props.backup !== null : useSaved())}
        >
          {rollback() ? "Roll back and restart" : "Install and restart"}
        </button>
      </div>
    </dialog>
  );
}
