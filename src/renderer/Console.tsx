import type { JSX } from "@solidjs/web";
import { ArrowDown, ArrowUp, CircleX, X } from "lucide";
import { createEffect, createMemo, createSignal, flush, For, onCleanup, Show } from "solid-js";
import { btnIconSmall } from "./buttons";
import {
  errorLines,
  findMatches,
  isErrorLine,
  startIndex,
  stepIndex,
  type Match,
} from "./consoleFind";
import { Icon } from "./Icon";
import { Tooltip } from "./Tooltip";
import { findBar, input } from "./ui";

const NEAR_BOTTOM_PX = 20;
const HIGHLIGHTS = ["console-match", "console-match-current", "console-error-current"] as const;

const lineClass = (line: string) =>
  isErrorLine(line) ? "text-failed" : line.includes(" WARN ") ? "text-cancelled" : "";

/** What the mounted Console lends the search: its rows and its scroller. */
export interface ConsoleView {
  /** The line's element. */
  row(index: number): HTMLElement | undefined;
  /** The first line in view; the line count when none is. */
  topIndex(): number;
  /** Centres the line in the scroller; with `onlyIfHidden`, only when it is not in view. */
  show(index: number, onlyIfHidden: boolean): void;
  /** Focuses the find field and selects its text. */
  focusField(): void;
}

/** The console's find and error stepping (console search spec R2, R3), kept by the bottom panel. */
export interface ConsoleSearch {
  query(): string;
  type(text: string): void;
  open(): boolean;
  openBar(): void;
  close(): void;
  matches(): readonly Match[];
  /** Index into matches(); -1 for none. */
  current(): number;
  step(dir: 1 | -1): void;
  errors(): readonly number[];
  currentError(): number;
  stepError(dir: 1 | -1): void;
  /** Returns the detach. */
  attach(view: ConsoleView): () => void;
}

/** A place in the console by line number, the store's dropped lines plus its index, so it keeps its line (R5). */
interface Place {
  line: number;
  start: number;
}

export function createConsoleSearch(o: {
  lines: () => readonly string[];
  dropped: () => number;
  /** The Console tab is the panel's tab. */
  shown: () => boolean;
  /** The panel is on screen: not under the Settings page (R5.3). */
  visible: () => boolean;
  showConsole: () => void;
}): ConsoleSearch {
  const [query, setQuery] = createSignal("");
  const [open, setOpen] = createSignal(false);
  const [held, setHeld] = createSignal<Place | null>(null);
  const [heldError, setHeldError] = createSignal<number | null>(null); // a line number
  let view: ConsoleView | undefined;
  let back: Element | null = null; // what had the focus before the bar opened

  // nothing is searched while the bar is closed (R3.11)
  const matches = createMemo(() => (open() ? findMatches(o.lines(), query()) : []));
  const current = createMemo(() => {
    const p = held();
    const at = p ? p.line - o.dropped() : -1;
    return p ? matches().findIndex((m) => m.line === at && m.start === p.start) : -1;
  });
  // only while the Console tab shows: the count is on its tab row (R2.1)
  const errors = createMemo(() => (o.shown() ? errorLines(o.lines()) : []));
  const currentError = createMemo(() => {
    const line = heldError();
    return line === null ? -1 : errors().indexOf(line - o.dropped());
  });
  const top = () => view?.topIndex() ?? 0;
  const hold = (list: readonly Match[], i: number) =>
    setHeld({ line: list[i].line + o.dropped(), start: list[i].start });
  // While there are matches one is current: the first from the top of the view, without scrolling (R3.10).
  createEffect(
    () => ({ list: matches(), at: current() }),
    ({ list, at }) => {
      if (list.length > 0 && at < 0)
        hold(
          list,
          startIndex(
            list.map((m) => m.line),
            top(),
            1,
          ),
        );
    },
  );

  const type = (text: string) => {
    setQuery(text);
    setHeld(null);
    flush();
    const i = current();
    if (i >= 0) view?.show(matches()[i].line, true);
  };
  const step = (dir: 1 | -1) => {
    const list = matches();
    if (list.length === 0) return;
    const i = current();
    const next =
      i < 0
        ? startIndex(
            list.map((m) => m.line),
            top(),
            dir,
          )
        : stepIndex(list.length, i, dir);
    hold(list, next);
    flush();
    view?.show(list[next].line, false);
  };
  const stepError = (dir: 1 | -1) => {
    const list = errors();
    if (list.length === 0) return;
    const i = currentError();
    const next = i < 0 ? startIndex(list, top(), dir) : stepIndex(list.length, i, dir);
    setHeldError(list[next] + o.dropped());
    flush();
    view?.show(list[next], false);
  };
  const openBar = () => {
    if (!open()) back = document.activeElement;
    o.showConsole();
    setOpen(true);
    flush(); // the Console and its bar must be drawn to take focus
    view?.focusField();
  };
  const close = () => {
    setOpen(false);
    flush();
    if (back instanceof HTMLElement && back.isConnected) back.focus();
    back = null;
  };

  const onKey = (e: KeyboardEvent) => {
    if (document.querySelector("dialog[open]") || !o.visible()) return;
    if (e.ctrlKey && !e.shiftKey && !e.altKey && e.key.toLowerCase() === "f") {
      e.preventDefault();
      openBar();
    } else if (e.key === "F8" && !e.ctrlKey && !e.altKey) {
      e.preventDefault();
      stepError(e.shiftKey ? -1 : 1);
    }
  };
  window.addEventListener("keydown", onKey);
  onCleanup(() => window.removeEventListener("keydown", onKey));

  return {
    query,
    type,
    open,
    openBar,
    close,
    matches,
    current,
    step,
    errors,
    currentError,
    stepError,
    attach(v) {
      view = v;
      return () => {
        if (view === v) view = undefined;
      };
    },
  };
}

/** Karate's output; `children` (the banner) come first, in the same scroller. */
export function Console(props: {
  lines: readonly string[];
  search: ConsoleSearch;
  children?: JSX.Element;
}) {
  let el!: HTMLDivElement;
  let field: HTMLInputElement | undefined;
  let pinned = true;
  const onScroll = () => {
    pinned = el.scrollHeight - el.scrollTop - el.clientHeight <= NEAR_BOTTOM_PX;
  };
  createEffect(
    // the array is replaced on every append; its length stops changing at the line cap
    () => props.lines,
    () => {
      if (pinned) el.scrollTop = el.scrollHeight;
    },
  );

  // the lines are the scroller's last children, after the banner
  const row = (i: number) =>
    el.children[el.childElementCount - props.lines.length + i] as HTMLElement | undefined;
  onCleanup(
    props.search.attach({
      row,
      topIndex() {
        let lo = 0;
        let hi = props.lines.length;
        while (lo < hi) {
          const mid = (lo + hi) >> 1;
          const r = row(mid)!;
          if (r.offsetTop + r.offsetHeight > el.scrollTop) hi = mid;
          else lo = mid + 1;
        }
        return lo;
      },
      show(i, onlyIfHidden) {
        const r = row(i);
        if (!r) return;
        const inView =
          r.offsetTop >= el.scrollTop &&
          r.offsetTop + r.offsetHeight <= el.scrollTop + el.clientHeight;
        if (onlyIfHidden && inView) return;
        el.scrollTop = Math.max(0, r.offsetTop - (el.clientHeight - r.offsetHeight) / 2);
      },
      focusField() {
        field?.focus();
        field?.select();
      },
    }),
  );

  const range = (line: number, start?: number, end?: number) => {
    const r = new Range();
    const text = row(line)?.firstChild;
    if (!text) return null;
    if (start === undefined) r.selectNodeContents(text);
    else {
      r.setStart(text, start);
      r.setEnd(text, end!);
    }
    return r;
  };
  const paint = (name: (typeof HIGHLIGHTS)[number], ranges: (Range | null)[]) => {
    const kept = ranges.filter((r) => r !== null);
    if (kept.length > 0) CSS.highlights.set(name, new Highlight(...kept));
    else CSS.highlights.delete(name);
  };
  // The browser paints over the text: no row is redrawn (R4.1).
  createEffect(
    () => ({ list: props.search.matches(), at: props.search.current() }),
    ({ list, at }) => {
      paint(
        "console-match",
        list.map((m) => range(m.line, m.start, m.end)),
      );
      const m = list[at];
      paint("console-match-current", m ? [range(m.line, m.start, m.end)] : []);
    },
  );
  createEffect(
    () => ({ list: props.search.errors(), at: props.search.currentError() }),
    ({ list, at }) => paint("console-error-current", at >= 0 ? [range(list[at])] : []),
  );
  onCleanup(() => HIGHLIGHTS.forEach((name) => CSS.highlights.delete(name)));

  const count = () => {
    const n = props.search.matches().length;
    if (!props.search.query()) return "";
    return n > 0 ? `${props.search.current() + 1} of ${n}` : "No results";
  };
  return (
    <div class="relative flex min-h-0 flex-1 flex-col">
      <Show when={props.search.open()}>
        <div data-testid="find-bar" class={findBar}>
          <input
            ref={(e) => (field = e)}
            data-testid="find-field"
            aria-label="Find in console"
            placeholder="Find"
            class={`${input} w-48`}
            value={props.search.query()}
            onInput={(e) => props.search.type(e.currentTarget.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") props.search.step(e.shiftKey ? -1 : 1);
              else if (e.key === "Escape") props.search.close();
              else return;
              e.preventDefault();
            }}
          />
          <span
            data-testid="find-count"
            role="status"
            class="min-w-16 px-1 text-xs text-muted tabular-nums"
          >
            {count()}
          </span>
          <Tooltip text="Previous match (Shift+Enter)">
            <button
              type="button"
              data-testid="find-prev"
              aria-label="Previous match"
              class={btnIconSmall}
              onClick={() => props.search.step(-1)}
            >
              <Icon icon={ArrowUp} size={14} />
            </button>
          </Tooltip>
          <Tooltip text="Next match (Enter)">
            <button
              type="button"
              data-testid="find-next"
              aria-label="Next match"
              class={btnIconSmall}
              onClick={() => props.search.step(1)}
            >
              <Icon icon={ArrowDown} size={14} />
            </button>
          </Tooltip>
          <Tooltip text="Close (Escape)">
            <button
              type="button"
              data-testid="find-close"
              aria-label="Close find"
              class={btnIconSmall}
              onClick={() => props.search.close()}
            >
              <Icon icon={X} size={14} />
            </button>
          </Tooltip>
        </div>
      </Show>
      <div
        ref={(e) => (el = e)}
        data-testid="console"
        onScroll={onScroll}
        class="relative min-h-0 flex-1 overflow-auto whitespace-pre-wrap bg-sunken px-3 py-2 font-mono text-xs text-ink"
      >
        {props.children}
        <For each={props.lines}>{(line) => <div class={lineClass(line)}>{line}</div>}</For>
      </div>
    </div>
  );
}

/** The error count and stepping on the Console tab's row (R2); absent with no error. */
export function ConsoleErrors(props: { search: ConsoleSearch }) {
  const count = () => {
    const n = props.search.errors().length;
    const i = props.search.currentError();
    const noun = n === 1 ? "error" : "errors";
    return i >= 0 ? `${i + 1} of ${n} ${noun}` : `${n} ${noun}`;
  };
  return (
    <Show when={props.search.errors().length > 0}>
      <div
        data-testid="console-errors"
        role="group"
        aria-label="Errors"
        class="flex items-center gap-1"
      >
        <span class="inline-flex items-center gap-1 px-1 text-xs font-medium text-failed tabular-nums">
          <Icon icon={CircleX} size={14} />
          <span data-testid="errors-count" role="status">
            {count()}
          </span>
        </span>
        <Tooltip text="Previous error (Shift+F8)">
          <button
            type="button"
            data-testid="error-prev"
            aria-label="Previous error"
            class={btnIconSmall}
            onClick={() => props.search.stepError(-1)}
          >
            <Icon icon={ArrowUp} size={14} />
          </button>
        </Tooltip>
        <Tooltip text="Next error (F8)">
          <button
            type="button"
            data-testid="error-next"
            aria-label="Next error"
            class={btnIconSmall}
            onClick={() => props.search.stepError(1)}
          >
            <Icon icon={ArrowDown} size={14} />
          </button>
        </Tooltip>
      </div>
    </Show>
  );
}
