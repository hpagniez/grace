import { Check, Minus } from "lucide";
import { createMemo, For, Show } from "solid-js";
import type { Api } from "../shared/ipc";
import { chipTags, cycleTag, filterOn, matchingLeaves, type FilterState } from "../shared/tags";
import { ghostSmall } from "./buttons";
import { Icon } from "./Icon";
import { leaves } from "./nodes";
import type { AppState } from "./reduce";
import { Tooltip } from "./Tooltip";
import { chip, chipExcluded, chipIncluded } from "./ui";

const CHIP = { off: chip, include: chipIncluded, exclude: chipExcluded } as const;
const HINT = {
  off: "Not filtered. Click to include.",
  include: "Included. Click to exclude.",
  exclude: "Excluded. Click to clear.",
} as const;
// Colour and strike-through are invisible to screen readers: the state goes in the name.
const SPOKEN = { off: "not filtered", include: "included", exclude: "excluded" } as const;

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
        class="flex flex-wrap items-center gap-1.5 border-b border-line px-3 py-1.5"
      >
        <For each={tags()}>
          {(tag) => (
            <Tooltip text={HINT[stateOf(tag)]} describe>
              <button
                type="button"
                data-testid="tag-chip"
                data-tag={tag}
                data-state={stateOf(tag)}
                class={CHIP[stateOf(tag)]}
                aria-label={`@${tag}, ${SPOKEN[stateOf(tag)]}`}
                disabled={locked()}
                onClick={() => void props.api.setFilter(cycleTag(plain(), tag))}
              >
                {/* A state shows by shape as well as colour (spec R7.3): an unfiltered tag has no icon. */}
                <Show when={stateOf(tag) !== "off"}>
                  <Icon icon={stateOf(tag) === "include" ? Check : Minus} size={14} />
                </Show>
                {`@${tag}`}
              </button>
            </Tooltip>
          )}
        </For>
        <Show when={filterOn(filter())}>
          <button
            type="button"
            data-testid="tag-clear"
            class={ghostSmall}
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
        </Show>
        {/* Always mounted: a live region added together with its text is often not announced. */}
        <span role="status" class="text-xs text-muted">
          <Show when={filterOn(filter())}>
            <span data-testid="tag-counter">{counter()}</span>
          </Show>
        </span>
        <label class="ml-auto flex h-6 items-center gap-1.5 text-xs">
          <input
            type="checkbox"
            data-testid="hide-non-matching"
            class="accent-accent"
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
