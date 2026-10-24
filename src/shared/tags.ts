// Shared by main and renderer: keep this file free of Node imports.
// Tags are stored and compared without '@', case-sensitive.
import type { TestNode } from "./model";

export interface TagFilter {
  include: string[];
  exclude: string[];
}

export interface FilterState extends TagFilter {
  /** Hide tests and tree nodes that do not match; dim them when false. */
  hideNonMatching: boolean;
}

export const NO_FILTER: FilterState = { include: [], exclude: [], hideNonMatching: false };

const MAX_TAGS = 500;
const MAX_TAG_LENGTH = 200;

export const filterOn = (f: TagFilter): boolean => f.include.length > 0 || f.exclude.length > 0;

/**
 * Whether `-t` can select the tag. Karate splits `-t` on commas and turns it into JS without
 * escaping (`'` and `(` fail the run); a leading `@` or `~` would change the expression.
 */
export const filterable = (tag: string): boolean => /^[^\s,'(\\@~][^\s,'(\\]*$/.test(tag);

/** A test matches when it has any included tag (or none are included) and no excluded tag. */
export const matches = (tags: string[], f: TagFilter): boolean =>
  (f.include.length === 0 || tags.some((t) => f.include.includes(t))) &&
  !tags.some((t) => f.exclude.includes(t));

/** Scenario and row nodes at or under `n` that match, in tree order. */
export function matchingLeaves(n: TestNode, f: TagFilter): TestNode[] {
  if (n.kind === "scenario" || n.kind === "row") return matches(n.tags, f) ? [n] : [];
  return n.children.flatMap((c) => matchingLeaves(c, f));
}

/** Sorted union of the filterable tags of every scenario and row. */
export function projectTags(tree: TestNode): string[] {
  const tags = new Set<string>();
  const walk = (n: TestNode) => {
    if (n.kind === "scenario" || n.kind === "row")
      n.tags.filter(filterable).forEach((t) => tags.add(t));
    n.children.forEach(walk);
  };
  walk(tree);
  return [...tags].sort();
}

/** Tags to show as chips: the project's, plus filtered ones that vanished until the user prunes them. */
export const chipTags = (tree: TestNode | null, f: TagFilter): string[] =>
  [...new Set([...(tree ? projectTags(tree) : []), ...f.include, ...f.exclude])].sort();

/** off -> include -> exclude -> off. */
export function cycleTag(f: FilterState, tag: string): FilterState {
  const without = (list: string[]) => list.filter((t) => t !== tag);
  if (f.include.includes(tag))
    return { ...f, include: without(f.include), exclude: [...f.exclude, tag] };
  if (f.exclude.includes(tag)) return { ...f, exclude: without(f.exclude) };
  return { ...f, include: [...f.include, tag] };
}

export const pruneFilter = (f: FilterState, tags: string[]): FilterState => ({
  ...f,
  include: f.include.filter((t) => tags.includes(t)),
  exclude: f.exclude.filter((t) => tags.includes(t)),
});

const tagList = (x: unknown): x is string[] =>
  Array.isArray(x) &&
  x.length <= MAX_TAGS &&
  x.every((t) => typeof t === "string" && filterable(t) && t.length <= MAX_TAG_LENGTH);

/** Validates untrusted input (IPC, settings file). */
export function parseFilterState(x: unknown): FilterState | null {
  if (typeof x !== "object" || x === null) return null;
  const { include, exclude, hideNonMatching } = x as Record<string, unknown>;
  if (!tagList(include) || !tagList(exclude) || typeof hideNonMatching !== "boolean") return null;
  if (include.some((t) => exclude.includes(t))) return null;
  return { include, exclude, hideNonMatching };
}
