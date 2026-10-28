import { createStore } from "solid-js";
import type { HttpGroup } from "../shared/http";
import type { Api } from "../shared/ipc";
import { narrowedCalls, shownCalls, toggledGroups, type CallRef } from "./http";
import { initialState, reduce, type AppState } from "./reduce";

export function createAppStore(api: Api): {
  state: AppState;
  select(key: string): void;
  showPanel(panel: AppState["panel"]): void;
  selectCall(ref: CallRef): void;
  toggleGroup(g: HttpGroup | "all"): void;
  dismissPrompt(): void;
  dispose(): void;
} {
  const [state, setState] = createStore(initialState());
  // subscribe before ready(): main re-sends the last project event on ready
  const dispose = api.onEvent((e) => setState((draft) => reduce(draft, e)));
  void api.ready();
  return {
    state,
    select: (key) =>
      setState((draft) => {
        draft.selected = key;
        draft.selectedCall = null; // a tree click gives the details pane back
      }),
    showPanel: (panel) =>
      setState((draft) => {
        draft.panel = panel;
        if (panel !== "http") draft.selectedCall = null;
      }),
    selectCall: (ref) => setState((draft) => void (draft.selectedCall = ref)),
    toggleGroup: (g) =>
      setState((draft) => {
        const groups = toggledGroups(draft.httpGroups, g);
        draft.httpGroups = groups;
        const open = draft.selectedCall;
        const listed = shownCalls(narrowedCalls(draft), groups);
        // a call the chips hide leaves the details pane
        if (open && !listed.some((l) => l.ref.key === open.key && l.ref.index === open.index))
          draft.selectedCall = null;
      }),
    dismissPrompt: () => setState((draft) => void (draft.updatePrompt = null)),
    dispose,
  };
}
