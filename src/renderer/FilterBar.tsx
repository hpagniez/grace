import { Check, Minus } from "lucide";
import { createEffect, createMemo, createSignal, For, Show } from "solid-js";
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

  // The chips wrap in a strip one line tall; the browser says which ones went past it (spec R1).
  let strip: HTMLDivElement | undefined;
  const [folded, setFolded] = createSignal<ReadonlySet<string>>(new Set());
  const foldedOn = () => [...folded()].filter((t) => stateOf(t) !== "off").length;
  // The panel of every tag: a native popover, so Escape, a click outside and the focus come free (R3).
  let panel: HTMLDivElement | undefined;
  const [open, setOpen] = createSignal(false);
  // Keyed on the tag names: a filter echo makes a new array of the same tags.
  const tagKey = createMemo(() => tags().join(" "));
  createEffect(tagKey, (key) => {
    if (!key || !strip || typeof IntersectionObserver === "undefined") return;
    const past = new Set<string>();
    const io = new IntersectionObserver(
      (entries) => {
        for (const e of entries) {
          const tag = (e.target as HTMLElement).dataset.tag!;
          if (e.isIntersecting) past.delete(tag);
          else past.add(tag);
        }
        setFolded(new Set(past));
        // Nothing left to unfold: the panel goes with its fold chip (R3.5).
        if (past.size === 0 && open()) {
          panel?.hidePopover?.();
          setOpen(false);
        }
      },
      { root: strip },
    );
    strip.querySelectorAll("[data-tag]").forEach((c) => io.observe(c));
    return () => io.disconnect();
  });

  const TagChip = (p: { tag: string; folded?: boolean }) => (
    <Tooltip text={HINT[stateOf(p.tag)]} describe>
      <button
        type="button"
        data-testid="tag-chip"
        data-tag={p.tag}
        data-state={stateOf(p.tag)}
        // invisible: out of the Tab order and the accessibility tree, keeping its place (R1.3)
        class={p.folded ? `${CHIP[stateOf(p.tag)]} invisible` : CHIP[stateOf(p.tag)]}
        aria-label={`@${p.tag}, ${SPOKEN[stateOf(p.tag)]}`}
        disabled={locked()}
        onClick={() => void props.api.setFilter(cycleTag(plain(), p.tag))}
      >
        {/* A state shows by shape as well as colour (spec R7.3): an unfiltered tag has no icon. */}
        <Show when={stateOf(p.tag) !== "off"}>
          <Icon icon={stateOf(p.tag) === "include" ? Check : Minus} size={14} />
        </Show>
        {`@${p.tag}`}
      </button>
    </Tooltip>
  );

  return (
    <Show when={tags().length > 0}>
      <div
        data-testid="filter-bar"
        class="flex items-center gap-1.5 border-b border-line px-3 py-1.5"
        style={{ "anchor-name": "--filter-bar" }}
      >
        {/* The clip margin keeps the focus ring (2 px + 2 px); the 6 px row gap keeps line 2 out of it. */}
        <div
          ref={(e) => (strip = e)}
          data-testid="tag-strip"
          class="flex h-6 min-w-0 flex-wrap gap-1.5 overflow-clip [overflow-clip-margin:4px]"
        >
          <For each={tags()}>{(tag) => <TagChip tag={tag} folded={folded().has(tag)} />}</For>
        </div>
        <Show when={folded().size > 0}>
          <Tooltip text="Show all tags" class="shrink-0">
            <button
              type="button"
              data-testid="tag-more"
              class={chip}
              popovertarget="tag-panel"
              aria-expanded={open() ? "true" : "false"}
              aria-label={`${folded().size} more ${folded().size === 1 ? "tag" : "tags"}${foldedOn() ? `, ${foldedOn()} on` : ""}`}
            >
              {`+${folded().size}${foldedOn() ? ` · ${foldedOn()} on` : ""}`}
            </button>
          </Tooltip>
        </Show>
        {/* Under the bar, its width, over the tree (R3.1); its chips exist only while it is open (R3.6). */}
        <div
          ref={(e) => (panel = e)}
          id="tag-panel"
          data-testid="tag-panel"
          popover="auto"
          role="group"
          aria-label="All tags"
          onToggle={(e) => setOpen(e.newState === "open")}
          style={{
            "position-anchor": "--filter-bar",
            inset: "anchor(bottom) anchor(right) auto anchor(left)",
          }}
          class="m-0 max-h-[50vh] w-auto overflow-auto rounded-b-lg border border-line bg-raised p-2 shadow-float"
        >
          <Show when={open()}>
            <div class="flex flex-wrap gap-1.5">
              <For each={tags()}>{(tag) => <TagChip tag={tag} />}</For>
            </div>
          </Show>
        </div>
        <Show when={filterOn(filter())}>
          <button
            type="button"
            data-testid="tag-clear"
            class={`${ghostSmall} shrink-0`}
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
        <span role="status" class="shrink-0 text-xs whitespace-nowrap text-muted">
          <Show when={filterOn(filter())}>
            <span data-testid="tag-counter">{counter()}</span>
          </Show>
        </span>
        <label class="ml-auto flex h-6 shrink-0 items-center gap-1.5 text-xs whitespace-nowrap">
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
