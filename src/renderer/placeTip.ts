const GAP = 6; // between the target and the tooltip
const EDGE = 8; // the tooltip stays this far inside the window

/**
 * Where a tooltip goes (spec R9.4): below its target and centred on it, above when it would come
 * within 8 px of the window's bottom, and never closer than 8 px to the window's left, right or top.
 */
export function placeTip(
  target: { top: number; left: number; bottom: number; width: number },
  tip: { width: number; height: number },
  view: { width: number; height: number },
): { top: number; left: number; above: boolean } {
  const above = target.bottom + GAP + tip.height > view.height - EDGE;
  const top = above ? Math.max(EDGE, target.top - GAP - tip.height) : target.bottom + GAP;
  const centred = target.left + (target.width - tip.width) / 2;
  const left = Math.max(EDGE, Math.min(centred, view.width - EDGE - tip.width));
  return { top, left, above };
}
