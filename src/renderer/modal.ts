import { createEffect, onCleanup } from "solid-js";

/**
 * Shows `dialog` as a modal while `open()` is true, puts focus on `first()`, and gives it back to where
 * it was when the dialog closes (R4.6), also when the component goes away while it is open. Call it in
 * the component that renders the `<dialog>`.
 */
export function showModalWhile(
  dialog: () => HTMLDialogElement,
  open: () => boolean,
  first: () => HTMLElement | undefined,
) {
  let before: Element | null = null;
  createEffect(open, (isOpen) => {
    const d = dialog();
    if (isOpen) {
      if (d.open) return;
      before = document.activeElement;
      d.showModal();
      first()?.focus();
    } else {
      if (d.open) d.close();
      (before as HTMLElement | null)?.focus();
      before = null;
    }
  });
  // a modal dialog makes the rest of the page inert, so it must close before focus can go back
  onCleanup(() => {
    if (!before) return;
    if (dialog().open) dialog().close();
    (before as HTMLElement).focus();
  });
}
