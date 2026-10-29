import { createSignal, onCleanup } from "solid-js";
import { NARROW_QUERY } from "./buttons";

/** Whether the toolbar's labels are hidden (the window is narrower than 1150 px); follows a resize. */
export function createNarrow() {
  const query = window.matchMedia?.(NARROW_QUERY); // jsdom has none
  const [narrow, setNarrow] = createSignal(query?.matches ?? false);
  if (query) {
    const update = () => setNarrow(query.matches);
    query.addEventListener("change", update);
    onCleanup(() => query.removeEventListener("change", update));
  }
  return narrow;
}
