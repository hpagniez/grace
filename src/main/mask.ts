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
