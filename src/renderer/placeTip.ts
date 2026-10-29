const GAP = 6; // between the target and the tooltip
const EDGE = 8; // the tooltip stays this far inside the window

/**
 * Where a tooltip asks to go. `below` by default; a row's tooltip goes `left` or `right` of its target,
 * or `over` its own cut text, so it never lies on the next row, which the person may be about to click.
 */
export type Side = "below" | "left" | "right" | "over";

/**
 * Where a tooltip goes (spec R9.4), never closer than 8 px to the window's edges.
 * - `below`: 6 px below its target and centred on it; `above` when it would come within 8 px of the
 *   window's bottom.
 * - `left` and `right`: 6 px beside its target and centred on it from top to bottom; on the other side
 *   when it would come within 8 px of the window's edge.
 * - `over`: on the target's own top-left.
 */
export function placeTip(
  target: { top: number; left: number; bottom: number; right: number },
  tip: { width: number; height: number },
  view: { width: number; height: number },
  side: Side = "below",
): { top: number; left: number; side: Side | "above" } {
  const inside = (at: number, size: number, room: number) =>
    Math.max(EDGE, Math.min(at, room - EDGE - size));
  const x = (at: number) => inside(at, tip.width, view.width);
  const y = (at: number) => inside(at, tip.height, view.height);
  if (side === "over") return { top: y(target.top), left: x(target.left), side };
  if (side === "below") {
    const above = target.bottom + GAP + tip.height > view.height - EDGE;
    const top = above ? Math.max(EDGE, target.top - GAP - tip.height) : target.bottom + GAP;
    const centred = (target.left + target.right - tip.width) / 2;
    return { top, left: x(centred), side: above ? "above" : "below" };
  }
  const leftOf = target.left - GAP - tip.width;
  const rightOf = target.right + GAP;
  const flip = side === "left" ? leftOf < EDGE : rightOf + tip.width > view.width - EDGE;
  const placed = flip ? (side === "left" ? "right" : "left") : side;
  const top = y((target.top + target.bottom - tip.height) / 2);
  return { top, left: x(placed === "left" ? leftOf : rightOf), side: placed };
}
