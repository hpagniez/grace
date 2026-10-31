import type { JSX } from "@solidjs/web";
import { createEffect, createMemo, createSignal, flush, For, onSettled, untrack } from "solid-js";
import { keyboardFocus } from "./Tooltip";
import {
  anchoredScrollTop,
  drawnRange,
  pageRows,
  scrollToShow,
  survivor,
  type View,
} from "./virtual";

export interface VirtualListHandle {
  /** Scrolls the least to show the row with this key (R4.1); nothing when it is not in the list. */
  reveal(key: string): void;
}

/**
 * Fixed-height rows, of which only those in view are drawn (spec R1), as one tab stop: the arrow keys move
 * through every row, drawn or not, and select as they go (R3, R4). The focus row is the last focused one if
 * it is there, else the selected one, else the first. It alone has tabindex 0 and is always drawn.
 */
export function VirtualList<T>(props: {
  items: readonly T[];
  keyOf(item: T): string;
  rowHeight: number;
  /** px before the first row and after the last; default 0. */
  pad?: number;
  role: "tree" | "listbox";
  itemRole: "treeitem" | "option";
  /** The list's aria-label. */
  label: string;
  /** On the role element. */
  testid?: string;
  /** On the scrolling element. */
  scrollerTestid?: string;
  /** On every row element. */
  itemClass?: string;
  /** Spread on the row element. */
  attrs(item: T, index: number): Record<string, string | number | undefined>;
  selectedKey: string | null;
  onSelect(item: T): void;
  /** The list's own keys; true when handled. */
  onKey?(e: KeyboardEvent, item: T, index: number): boolean;
  ref?(handle: VirtualListHandle): void;
  children(item: T, index: number, keyboardFocused: () => boolean): JSX.Element;
}) {
  let scroller!: HTMLDivElement;
  let list!: HTMLDivElement;
  const [scrollTop, setScrollTop] = createSignal(0);
  const [height, setHeight] = createSignal(0);
  const [active, setActive] = createSignal<string | null>(null); // the row last focused
  const [keyboard, setKeyboard] = createSignal<string | null>(null); // the row focused from the keyboard
  let refocus: string | null = null; // the row to focus once it is the focus row, drawn

  const pad = () => props.pad ?? 0;
  const view = (top: number): View => ({
    scrollTop: top,
    height: height(),
    rowHeight: props.rowHeight,
    pad: pad(),
  });
  const keys = createMemo(() => props.items.map((item) => props.keyOf(item)));
  const focusIndex = createMemo(() => {
    const all = keys();
    const a = active();
    const i = a === null ? -1 : all.indexOf(a);
    if (i >= 0) return i;
    const s = props.selectedKey === null ? -1 : all.indexOf(props.selectedKey);
    return s >= 0 ? s : all.length > 0 ? 0 : -1;
  });
  const focusKey = createMemo(() => keys()[focusIndex()] ?? null);
  const drawn = createMemo(() => {
    const { first, last } = drawnRange(keys().length, view(scrollTop()));
    const at = (index: number) => ({ key: keys()[index], index, item: props.items[index] });
    // The focus row comes first, so the DOM reconcile never moves its node: a focused node that moves loses
    // focus. Its place on screen comes from `top`, its place in the list from the row's aria attributes.
    const f = focusIndex();
    const rows = f >= 0 ? [at(f)] : [];
    for (let i = first; i < last; i++) if (i !== f) rows.push(at(i));
    return rows;
  });

  const rowOf = (key: string) =>
    [...list.children].find((el) => (el as HTMLElement).dataset.key === key) as
      | HTMLElement
      | undefined;
  /** The key of a row element; null for anything else, a button in a row included. */
  const rowKey = (t: EventTarget | null) =>
    t instanceof HTMLElement && t.parentElement === list ? t.dataset.key! : null;
  const scrollTo = (top: number) => {
    scroller.scrollTop = top;
    setScrollTop(scroller.scrollTop); // as the browser clamped it
  };

  const moveTo = (index: number) => {
    const item = props.items[index];
    const key = props.keyOf(item);
    scrollTo(scrollToShow(index, view(scroller.scrollTop)));
    setActive(key);
    flush(); // the row must be drawn to take focus
    rowOf(key)?.focus({ preventScroll: true });
    setKeyboard(key); // a key moved it, whatever :focus-visible guesses
    props.onSelect(item);
  };

  const onKeyDown = (e: KeyboardEvent) => {
    const f = focusIndex();
    if (f < 0) return;
    const last = keys().length - 1;
    const page = pageRows(view(scroller.scrollTop));
    const plain = !e.ctrlKey && !e.metaKey && !e.altKey;
    const moves: Record<string, number> = {
      ArrowDown: f + 1,
      ArrowUp: f - 1,
      Home: 0,
      End: last,
      PageDown: Math.min(last, f + page),
      PageUp: Math.max(0, f - page),
    };
    const to = plain ? moves[e.key] : undefined;
    if (to !== undefined) {
      e.preventDefault(); // at an end too, so the page stays still (R3.6)
      if (to >= 0 && to <= last && to !== f) moveTo(to);
    } else if (plain && (e.key === "Enter" || e.key === " ")) {
      e.preventDefault();
      props.onSelect(props.items[f]);
    } else if (props.onKey?.(e, props.items[f], f)) e.preventDefault();
  };

  // Rows added or removed above the view keep the rows in view still (R4.6). A focused row that left hands
  // over to the nearest row above (R4.4, R4.5). The scroll position and the focus are read before the rows
  // are redrawn: Chromium fires focusout when it removes the focused row, and clamps the scroll.
  createEffect(
    () => ({
      keys: keys(),
      top: scroller?.scrollTop ?? 0,
      holds: !!list?.contains(document.activeElement),
    }),
    (now, before) => {
      if (!before) return;
      const top = untrack(() => anchoredScrollTop(view(now.top), before.keys, now.keys));
      if (top !== now.top) scrollTo(top);
      const a = untrack(active);
      if (a === null || now.keys.includes(a)) return;
      // An unfocused list falls back to the selected row, else the first (R3.1).
      if (!now.holds) {
        setActive(null);
        return;
      }
      const next = survivor(a, before.keys, now.keys);
      refocus = next;
      setActive(next);
    },
  );
  // Runs after the effect above. Until the survivor is active, the focus row is the selected or the first
  // one: only the survivor takes focus.
  createEffect(focusKey, (key) => {
    if (key === null || key !== refocus) return;
    refocus = null;
    rowOf(key)?.focus({ preventScroll: true });
  });

  onSettled(() => {
    setHeight(scroller.clientHeight);
    if (typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver(() => setHeight(scroller.clientHeight));
    ro.observe(scroller);
    return () => ro.disconnect();
  });

  props.ref?.({
    reveal(key) {
      const i = keys().indexOf(key);
      if (i >= 0) scrollTo(scrollToShow(i, view(scroller.scrollTop)));
    },
  });

  return (
    <div
      ref={(e) => (scroller = e)}
      data-testid={props.scrollerTestid}
      class="min-h-0 flex-1 overflow-auto"
      onScroll={() => setScrollTop(scroller.scrollTop)}
    >
      <div
        ref={(e) => (list = e)}
        role={props.role}
        aria-label={props.label}
        data-testid={props.testid}
        data-rows={props.items.length}
        style={{
          position: "relative",
          height: `${props.items.length * props.rowHeight + 2 * pad()}px`,
        }}
        onKeyDown={onKeyDown}
        onFocusIn={(e) => {
          const row = e.target as HTMLElement;
          const key = rowKey(row);
          if (key === null) return;
          const moves = key !== focusKey();
          setActive(key);
          // Focus that lands on another row first (a press, a screen reader): as the focus row it moves
          // to the front, which drops its focus once.
          if (moves) {
            flush();
            if (document.activeElement !== row) row.focus({ preventScroll: true });
          }
          setKeyboard(keyboardFocus(row) ? key : null);
        }}
        onFocusOut={(e) => rowKey(e.target) !== null && setKeyboard(null)}
      >
        <For each={drawn()} keyed={(r) => r.key}>
          {(r) => {
            let el!: HTMLDivElement;
            const key = untrack(r).key;
            // Only a new item or place redraws the row's content, not every scroll.
            const index = createMemo(() => r().index);
            const item = createMemo(() => r().item);
            return (
              <div
                ref={(e) => (el = e)}
                role={props.itemRole}
                data-key={key}
                tabindex={key === focusKey() ? 0 : -1}
                class={props.itemClass}
                style={{
                  position: "absolute",
                  left: "0",
                  right: "0",
                  top: `${pad() + index() * props.rowHeight}px`,
                  height: `${props.rowHeight}px`,
                }}
                // A press takes no focus: Chromium drops the click of a row that moves during its press, as
                // the focus row moves to the front. The click then focuses the row, from a button in it too,
                // so the next key moves from it.
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => {
                  setActive(key);
                  flush(); // to the front first, then focus: a focused node that moves loses focus
                  // A script focus would match :focus-visible: a click is not keyboard focus.
                  el.focus({ preventScroll: true, focusVisible: false });
                }}
                {...props.attrs(item(), index())}
              >
                {props.children(item(), index(), () => keyboard() === key)}
              </div>
            );
          }}
        </For>
      </div>
    </div>
  );
}
