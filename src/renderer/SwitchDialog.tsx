import { createSignal, Show } from "solid-js";
import { compareVersions } from "../shared/version";
import type { Backup, Release } from "../shared/updates";
import { btnDialog, dangerDialog, primaryDialog } from "./buttons";
import { showModalWhile } from "./modal";
import { Notice } from "./Notice";
import { dialog as dialogStyle } from "./ui";

const pad = (n: number) => String(n).padStart(2, "0");
/** `YYYY-MM-DD`, the local date of an ISO time. */
const localDate = (iso: string) => {
  const d = new Date(iso);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
};

/**
 * Asks before switching to `target` from the Versions table (app-updates R7): a rollback explains what
 * happens to the data and warns; an install of a newer version that has a saved copy asks which data to
 * use. Open while `open` is true; keep it rendered after, so its words stay while it fades out (R5.2).
 */
export function SwitchDialog(props: {
  open: boolean;
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
    () => props.open,
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
      class={`${dialogStyle} w-[28rem] max-w-[95vw]`}
      onClose={() => dialog.open || props.onCancel()} // Escape; not a late event after a reopen
    >
      <h2 id="switch-dialog-title" class="text-lg font-semibold">
        {rollback() ? `Roll back to ${v()}?` : `Install ${v()}?`}
      </h2>
      <Show
        when={rollback()}
        fallback={
          <div class="mt-3 flex flex-col gap-1 text-sm">
            <label class="flex min-h-6 items-center gap-2">
              <input
                type="radio"
                name="switch-data"
                class="accent-accent"
                checked={useSaved()}
                onChange={() => setUseSaved(true)}
              />
              {`Use the data you had in ${v()} (${date()})`}
            </label>
            <label class="flex min-h-6 items-center gap-2">
              <input
                type="radio"
                name="switch-data"
                class="accent-accent"
                checked={!useSaved()}
                onChange={() => setUseSaved(false)}
              />
              Keep your current data
            </label>
          </div>
        }
      >
        <p class="mt-2 text-sm">
          {props.backup
            ? `Grace backs up your current data, then restores the data saved when you last used ${v()} (${date()}).`
            : `Grace backs up your current data. ${v()} has no saved data on this computer, so it starts from your current data, which it may not read correctly.`}
        </p>
        <div class="mt-3">
          <Notice kind="warning" text={warning()} testid="switch-warning" />
        </div>
      </Show>
      {/* the action first, Cancel last (spec R19.1); a rollback may break the data, so it is the danger style */}
      <div class="mt-5 flex justify-end gap-2">
        <button
          type="button"
          data-testid="switch-confirm"
          ref={(e) => (confirm = e)}
          class={rollback() ? dangerDialog : primaryDialog}
          onClick={() => props.onConfirm(rollback() ? props.backup !== null : useSaved())}
        >
          {rollback() ? "Roll back and restart" : "Install and restart"}
        </button>
        <button
          type="button"
          data-testid="switch-cancel"
          ref={(e) => (cancel = e)}
          class={btnDialog}
          onClick={() => props.onCancel()}
        >
          Cancel
        </button>
      </div>
    </dialog>
  );
}
