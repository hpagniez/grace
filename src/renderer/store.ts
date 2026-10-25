import { createStore } from "solid-js";
import type { Api } from "../shared/ipc";
import { initialState, reduce, type AppState } from "./reduce";

export function createAppStore(api: Api): {
  state: AppState;
  select(key: string): void;
  showPanel(panel: AppState["panel"]): void;
  dispose(): void;
} {
  const [state, setState] = createStore(initialState());
  // subscribe before ready(): main re-sends the last project event on ready
  const dispose = api.onEvent((e) => setState((draft) => reduce(draft, e)));
  void api.ready();
  return {
    state,
    select: (key) => setState((draft) => void (draft.selected = key)),
    showPanel: (panel) => setState((draft) => void (draft.panel = panel)),
    dispose,
  };
}
