import { stat as fsStat } from "node:fs/promises";
import type { AppEvent, RecentEntry } from "../shared/events";
import { within } from "./karate/process";

export type Stat = (p: string) => Promise<{ isDirectory(): boolean }>;

/**
 * The checks that have not settled, by stat function and path. A hung stat keeps one of libuv's four
 * threads until Windows gives up, so a later call joins it instead of starting another.
 */
const pending = new WeakMap<Stat, Map<string, Promise<boolean>>>();

function exists(path: string, stat: Stat): Promise<boolean> {
  const byPath = pending.get(stat) ?? new Map<string, Promise<boolean>>();
  pending.set(stat, byPath);
  let check = byPath.get(path);
  if (!check) {
    check = stat(path).then(
      (s) => s.isDirectory(),
      () => false,
    );
    byPath.set(path, check);
    const forget = () => byPath.delete(path);
    check.then(forget, forget);
  }
  return check;
}

/**
 * Whether each folder still exists, all checked in parallel, order kept. A file or a failing stat is
 * not found. A check that has not settled within `ms` counts as existing: a disconnected network
 * share can take 20 s or more to answer, and an unknown folder must not be shown as gone.
 */
export async function checkRecent(
  paths: string[],
  stat: Stat = fsStat,
  ms = 1000,
): Promise<RecentEntry[]> {
  return Promise.all(
    paths.map(async (path) => {
      const check = exists(path, stat);
      const answered = await within(check, ms);
      return { path, exists: answered ? await check : true };
    }),
  );
}

/**
 * A function that loads the recent list, checks it and emits `recent`. A call emits nothing if a
 * later call started meanwhile, so a slow old check never overwrites a newer list.
 */
export function recentSender(
  load: () => string[],
  emit: (e: AppEvent) => void,
  stat?: Stat,
  ms?: number,
): () => Promise<void> {
  let latest = 0;
  return async () => {
    const mine = ++latest;
    const entries = await checkRecent(load(), stat, ms);
    if (mine === latest) emit({ type: "recent", entries });
  };
}
