/** The window math of a virtual list: pure functions, so the list component only wires them to the DOM (spec R1.6, R4). */

/** Rows drawn beyond the view, either side, so a fast scroll shows no gap. */
export const OVERSCAN = 10;
/** px, a scroller that measures 0 (jsdom, R1.6). */
export const FALLBACK_VIEW = 600;

export interface View {
  scrollTop: number;
  /** The scroller's clientHeight; 0 means FALLBACK_VIEW. */
  height: number;
  rowHeight: number;
  /** px before the first row. */
  pad: number;
}

const viewHeight = (v: View) => v.height || FALLBACK_VIEW;

/** The rows to draw: first inclusive, last exclusive, with OVERSCAN either side. scrollTop is clamped to the content first. */
export function drawnRange(count: number, v: View): { first: number; last: number } {
  const height = viewHeight(v);
  const top = Math.min(Math.max(v.scrollTop, 0), Math.max(0, v.pad + count * v.rowHeight - height));
  const first = Math.max(0, Math.floor((top - v.pad) / v.rowHeight) - OVERSCAN);
  const last = Math.min(count, Math.ceil((top + height - v.pad) / v.rowHeight) + OVERSCAN);
  return { first, last };
}

/** The scrollTop that shows row `index` with the least scroll (R4.1); v.scrollTop when it is in view. */
export function scrollToShow(index: number, v: View): number {
  const top = v.pad + index * v.rowHeight;
  const bottom = top + v.rowHeight;
  if (top < v.scrollTop) return top;
  const height = viewHeight(v);
  return bottom > v.scrollTop + height ? bottom - height : v.scrollTop;
}

/** Whole rows in view, at least 1 (Page Up, Page Down). */
export function pageRows(v: View): number {
  return Math.max(1, Math.floor(viewHeight(v) / v.rowHeight));
}

/** The scrollTop that keeps the first row in view still on screen when the rows change (R4.6). 0 stays 0; an anchor that is gone keeps v.scrollTop. */
export function anchoredScrollTop(
  v: View,
  before: readonly string[],
  after: readonly string[],
): number {
  if (v.scrollTop === 0) return 0;
  const index = Math.floor((v.scrollTop - v.pad) / v.rowHeight);
  const anchor = before[index];
  const now = anchor === undefined ? -1 : after.indexOf(anchor);
  if (now < 0) return v.scrollTop;
  // The offset into the anchor row survives, so the rows do not jump by a part of a row.
  return v.scrollTop + (now - index) * v.rowHeight;
}

/** Where focus goes when `key` leaves the list: the nearest row before it that is still there, else the first row, else null (R4.4, R4.5). */
export function survivor(
  key: string,
  before: readonly string[],
  after: readonly string[],
): string | null {
  const kept = new Set(after);
  for (let i = before.indexOf(key) - 1; i >= 0; i--) if (kept.has(before[i])) return before[i];
  return after[0] ?? null;
}
