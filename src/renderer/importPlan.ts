import {
  envVarFromKey,
  nameError,
  nameKeys,
  propertyValueError,
  type ImportEntry,
} from "../shared/env";

export interface ImportPlan {
  /** The rows to add or replace, in file order. */
  rows: { key: string; name: string; value: string }[];
  adds: number;
  /** The new names that already have a row in the list. */
  replaces: string[];
  /** Never the value: it is a secret. */
  skips: { key: string; reason: string }[];
}

/**
 * What importing `entries` into a list holding `existing` does (docs/specs/2026-11-04-env-var-properties.md
 * R5): each key named by its kind, checked by its rules, and matched with the row it replaces.
 */
export function importPlan(
  entries: ImportEntry[],
  o: { envVar: boolean; prefix: string; existing: { name: string; envVar: boolean }[] },
): ImportPlan {
  const taken = new Set(o.existing.flatMap((r) => nameKeys(r.name, r.envVar)));
  const earlier = new Map<string, string>(); // name key -> the key that took it
  const plan: ImportPlan = { rows: [], adds: 0, replaces: [], skips: [] };
  for (const { key, value } of entries) {
    const name = o.envVar ? envVarFromKey(o.prefix, key) : key;
    const keys = nameKeys(name, o.envVar);
    const first = keys.map((k) => earlier.get(k)).find((k) => k !== undefined);
    const reason =
      nameError(name, o.envVar) ??
      (value === "" ? "Empty value" : propertyValueError(value)) ??
      (first !== undefined ? `Same name as ${first}` : null);
    if (reason) {
      plan.skips.push({ key, reason });
      continue;
    }
    for (const k of keys) earlier.set(k, key);
    plan.rows.push({ key, name, value });
    if (keys.some((k) => taken.has(k))) plan.replaces.push(name);
    else plan.adds++;
  }
  return plan;
}
