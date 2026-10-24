import { onCleanup, Show } from "solid-js";
import type { Api } from "../shared/ipc";
import { Console } from "./Console";
import { Details } from "./Details";
import { Progress } from "./Progress";
import { Toolbar } from "./Toolbar";
import { Tree } from "./Tree";
import { createAppStore } from "./store";

export function App(props: { api: Api }) {
  const { state, select, dispose } = createAppStore(props.api);
  onCleanup(dispose);
  return (
    <div class="flex h-screen flex-col bg-white text-zinc-900 dark:bg-zinc-900 dark:text-zinc-100">
      <Toolbar state={state} api={props.api} />
      <Progress state={state} />
      <Show when={state.notice}>
        <div
          data-testid="notice"
          class="border-b border-amber-300 bg-amber-50 px-3 py-1 text-sm text-amber-900 dark:border-amber-700 dark:bg-amber-950 dark:text-amber-200"
        >
          {state.notice}
        </div>
      </Show>
      <main class="flex min-h-0 flex-1">
        <div class="flex min-h-0 w-1/2 flex-col">
          <Tree state={state} select={select} api={props.api} />
        </div>
        <div class="flex min-h-0 w-1/2 flex-col">
          <Details state={state} />
        </div>
      </main>
      <Console lines={state.console} />
    </div>
  );
}
