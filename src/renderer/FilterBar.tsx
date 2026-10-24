import { createMemo, For, Show } from "solid-js";
import type { Api } from "../shared/ipc";
import { chipTags, cycleTag, filterOn, matchingLeaves, type FilterState } from "../shared/tags";
import { leaves } from "./nodes";
import type { AppState } from "./reduce";

const chipBase =
  "rounded border px-1.5 py-0.5 text-xs disabled:cursor-not-allowed disabled:opacity-40";
const CHIP = {
  off: `${chipBase} border-zinc-300 bg-zinc-100 text-zinc-700 hover:bg-zinc-200 dark:border-zinc-600 dark:bg-zinc-800 dark:text-zinc-300 dark:hover:bg-zinc-700`,
  include: `${chipBase} border-green-600 bg-green-100 text-green-800 dark:bg-green-900/40 dark:text-green-300`,
  exclude: `${chipBase} border-red-600 bg-red-100 text-red-800 line-through dark:bg-red-900/40 dark:text-red-300`,
} as const;
const HINT = {
  off: "Not filtered. Click to include.",
  include: "Included. Click to exclude.",
  exclude: "Excluded. Click to clear.",
} as const;

/** Tag chips for the open project. The window never edits the filter itself: main echoes it back. */
export function FilterBar(props: { state: AppState; api: Api }) {
  const filter = () => props.state.filter;
  // A store proxy cannot be structured-cloned, so copy before handing the filter to IPC.
  const plain = (): FilterState => ({
    include: [...filter().include],
    exclude: [...filter().exclude],
    hideNonMatching: filter().hideNonMatching,
  });
  const locked = () => props.state.run.active || props.state.busy;
  const tags = createMemo(() =>
    props.state.tree ? chipTags(props.state.tree, props.state.filter) : [],
  );
  const stateOf = (tag: string) =>
    filter().include.includes(tag) ? "include" : filter().exclude.includes(tag) ? "exclude" : "off";
  const counter = () =>
    `${matchingLeaves(props.state.tree!, filter()).length} of ${leaves(props.state.tree!).length} tests match`;
  return (
    <Show when={tags().length > 0}>
      <div
        data-testid="filter-bar"
        class="flex flex-wrap items-center gap-1.5 border-b border-zinc-200 px-3 py-1.5 dark:border-zinc-700"
      >
        <For each={tags()}>
          {(tag) => (
            <button
              type="button"
              data-testid="tag-chip"
              data-tag={tag}
              data-state={stateOf(tag)}
              class={CHIP[stateOf(tag)]}
              title={HINT[stateOf(tag)]}
              disabled={locked()}
              onClick={() => void props.api.setFilter(cycleTag(plain(), tag))}
            >
              {`@${tag}`}
            </button>
          )}
        </For>
        <Show when={filterOn(filter())}>
          <button
            type="button"
            data-testid="tag-clear"
            class="rounded px-1.5 py-0.5 text-xs text-sky-700 hover:underline disabled:cursor-not-allowed disabled:opacity-40 dark:text-sky-400"
            disabled={locked()}
            onClick={() =>
              void props.api.setFilter({
                include: [],
                exclude: [],
                hideNonMatching: filter().hideNonMatching,
              })
            }
          >
            Clear
          </button>
          <span data-testid="tag-counter" class="text-xs text-zinc-600 dark:text-zinc-400">
            {counter()}
          </span>
        </Show>
        <label class="ml-auto flex items-center gap-1 text-xs">
          <input
            type="checkbox"
            data-testid="hide-non-matching"
            checked={filter().hideNonMatching}
            disabled={locked()}
            onChange={(e) =>
              void props.api.setFilter({ ...plain(), hideNonMatching: e.currentTarget.checked })
            }
          />
          Hide non-matching
        </label>
      </div>
    </Show>
  );
}
