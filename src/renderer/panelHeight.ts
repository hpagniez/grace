// The bottom panel's height rules (spec UI polish R1), in px.
export const PANEL_DEFAULT = 192;
export const PANEL_MIN = 96;
/** The least the tree and details area keeps. */
export const MAIN_MIN = 120;
export const PANEL_STEP = 16;
export const PANEL_KEY = "karate-kid.panelHeight";

/** The largest panel height when the tree/details area and the panel share `room` px; never below the minimum. */
export const maxPanel = (room: number): number => Math.max(PANEL_MIN, room - MAIN_MIN);

export const clampPanel = (h: number, max: number): number =>
  Math.min(Math.max(Math.round(h), PANEL_MIN), max);

/** The height a key on the splitter asks for; null for a key the splitter does not use. */
export function keyHeight(key: string, shown: number, max: number): number | null {
  const next =
    key === "ArrowUp"
      ? shown + PANEL_STEP
      : key === "ArrowDown"
        ? shown - PANEL_STEP
        : key === "Home"
          ? PANEL_MIN
          : key === "End"
            ? max
            : null;
  return next === null ? null : clampPanel(next, max);
}

/** The saved height; the default when none is saved, the value is unusable or storage throws. */
export function loadPanelHeight(storage?: Pick<Storage, "getItem">): number {
  try {
    const raw = (storage ?? globalThis.localStorage).getItem(PANEL_KEY);
    const h = raw === null ? NaN : Number(raw);
    return Number.isFinite(h) && h >= PANEL_MIN ? h : PANEL_DEFAULT;
  } catch {
    return PANEL_DEFAULT;
  }
}

/** Never throws: a height that cannot be saved is only forgotten. */
export function savePanelHeight(h: number, storage?: Pick<Storage, "setItem">): void {
  try {
    (storage ?? globalThis.localStorage).setItem(PANEL_KEY, String(Math.round(h)));
  } catch {
    // forgotten
  }
}
