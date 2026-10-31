// Small numbers helpers of the bench: the median of a measure's rounds and the order the builds take turns in.

/**
 * The median of the numbers in `xs`, or null when there are none (a `null` is a round that could not measure it).
 * @param {(number | null)[]} xs
 * @returns {number | null}
 */
export function median(xs) {
  const sorted = xs.filter((x) => typeof x === "number").sort((x, y) => x - y);
  if (!sorted.length) return null;
  const mid = sorted.length >> 1;
  return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
}

/**
 * The builds' runs in order: one warm-up of each build, then `rounds` rounds, each build once per round, the order
 * flipping every round (a, b, b, a, a, b, ...) so a drift in the machine's load does not favour one build. The first
 * `builds.length` entries are the warm-ups.
 * @param {number} rounds
 * @param {("a" | "b")[]} builds
 * @returns {("a" | "b")[]}
 */
export function order(rounds, builds) {
  const turns = Array.from({ length: rounds }, (_, i) => (i % 2 ? [...builds].reverse() : builds));
  return [...builds, ...turns.flat()];
}
