// The order of the bottom panel's Console, History and HTTP tabs (spec reorder-panel-tabs R1, R4).
export const TAB_IDS = ["console", "history", "http"] as const;
export type TabId = (typeof TAB_IDS)[number];
export const TABS_KEY = "grace.panelTabs";

/** The list with `id` moved to place `to`, held between the first and the last; the same list if it stays. */
export function moveTab(order: TabId[], id: TabId, to: number): TabId[] {
  const from = order.indexOf(id);
  const place = Math.min(Math.max(to, 0), order.length - 1);
  if (place === from) return order;
  const next = order.filter((t) => t !== id);
  next.splice(place, 0, id);
  return next;
}

/** The saved order; the default when none is saved, it is not exactly the three tabs once each, or storage throws. */
export function loadTabOrder(storage?: Pick<Storage, "getItem">): TabId[] {
  try {
    const saved: unknown = JSON.parse((storage ?? globalThis.localStorage).getItem(TABS_KEY) ?? "");
    if (
      Array.isArray(saved) &&
      TAB_IDS.every((t) => saved.includes(t)) &&
      saved.length === TAB_IDS.length
    )
      return saved as TabId[];
  } catch {
    // the default
  }
  return [...TAB_IDS];
}

/** Never throws: an order that cannot be saved is only forgotten. */
export function saveTabOrder(order: TabId[], storage?: Pick<Storage, "setItem">): void {
  try {
    (storage ?? globalThis.localStorage).setItem(TABS_KEY, JSON.stringify(order));
  } catch {
    // forgotten
  }
}
