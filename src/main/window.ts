import type { Rectangle } from "electron";

const BORDER = 8;

const overlap = (a: Rectangle, b: Rectangle) =>
  Math.max(0, Math.min(a.x + a.width, b.x + b.width) - Math.max(a.x, b.x)) *
  Math.max(0, Math.min(a.y + a.height, b.y + b.height) - Math.max(a.y, b.y));

/**
 * Where the saved window goes: onto the work area it overlaps most (the first on a tie), shrunk to
 * fit and moved fully inside it. Null when it overlaps none: the window opens as on a first start.
 */
export function placeWindow(saved: Rectangle, workAreas: Rectangle[]): Rectangle | null {
  let best: Rectangle | null = null;
  let most = 0;
  for (const area of workAreas) {
    const o = overlap(saved, area);
    if (o > most) [best, most] = [area, o];
  }
  if (!best) return null;
  const a = best;
  // Windows' invisible resize border: a window flush with a screen edge (snapped, say) sticks out
  // by up to 8 DIP on the left, right and bottom. It stays where it is.
  const flush =
    saved.x >= a.x - BORDER &&
    saved.x + saved.width <= a.x + a.width + BORDER &&
    saved.y >= a.y &&
    saved.y + saved.height <= a.y + a.height + BORDER;
  if (flush) {
    const [x, y, width, height] = [saved.x, saved.y, saved.width, saved.height].map(Math.round);
    return { x, y, width, height };
  }
  const width = Math.min(Math.round(saved.width), best.width);
  const height = Math.min(Math.round(saved.height), best.height);
  const inside = (pos: number, size: number, start: number, room: number) =>
    Math.min(Math.max(Math.round(pos), start), start + room - size);
  return {
    x: inside(saved.x, width, best.x, best.width),
    y: inside(saved.y, height, best.y, best.height),
    width,
    height,
  };
}
