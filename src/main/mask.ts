import type { AppEvent } from "../shared/events";

/**
 * Every spelling of a secret that can show up in text: as typed, URL-encoded and JSON-escaped, without
 * duplicates. An empty value has none, since it would match everywhere.
 * shortcut: encodeURIComponent throws on a lone surrogate; values are printable ASCII (checked when
 * they are saved), so revisit if that rule is relaxed.
 */
export function secretForms(value: string): string[] {
  if (value === "") return [];
  return [...new Set([value, encodeURIComponent(value), JSON.stringify(value).slice(1, -1)])];
}

/** A function that replaces every form of every value with `****`, longest form first, matching literally. */
export function makeMasker(values: string[]): (text: string) => string {
  const forms = values.flatMap(secretForms).sort((a, b) => b.length - a.length);
  return (text) => forms.reduce((masked, form) => masked.split(form).join("****"), text);
}

// Identifiers the window matches events and tests on, never run text: masking them would break it.
const EXACT = new Set(["type", "key", "keys", "selection", "runId", "status", "outcome", "env"]);

/**
 * The event with every string in it masked, at any depth, except the fields in EXACT: run text a
 * future event field carries is masked without anyone listing it.
 */
export function maskEvent(e: AppEvent, mask: (text: string) => string): AppEvent {
  const walk = (v: unknown): unknown =>
    typeof v === "string"
      ? mask(v)
      : Array.isArray(v)
        ? v.map(walk)
        : v !== null && typeof v === "object"
          ? Object.fromEntries(Object.entries(v).map(([k, x]) => [k, EXACT.has(k) ? x : walk(x)]))
          : v;
  return walk(e) as AppEvent;
}
