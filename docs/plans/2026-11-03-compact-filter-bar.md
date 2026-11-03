# A one-line filter bar Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The filter bar keeps one line; chips past the line fold behind a `+N` chip that opens a panel of every tag over the tree.

**Architecture:** The chips wrap, as today, inside a strip clipped to one 24 px line. One `IntersectionObserver` rooted on the strip marks the chips on hidden lines; Grace hides them (`invisible`) and counts them. The panel is a native `popover="auto"` opened by `popovertarget` on the fold chip, placed under the bar with CSS anchor positioning; its chips render only while it is open.

**Tech Stack:** SolidJS 2, Tailwind CSS v4, Chromium's Popover API and anchor positioning (Electron 45), Vitest + jsdom, Playwright.

**Spec:** [docs/specs/2026-11-03-compact-filter-bar.md](../specs/2026-11-03-compact-filter-bar.md)

## Global Constraints

- Solid 2 rules of `AGENTS.md`: `createEffect(compute, apply)`, `flush()`, `onClick`, no `classList`.
- Colours only through tokens; shared class strings from `ui.ts` and `buttons.ts`; tooltips only through `Tooltip`; no `title=`.
- Idle costs nothing: no timer, poll or layout read in Grace's code.
- The tag expression, `FilterState` and `cycleTag` do not change. `Tree.tsx` does not change.
- Copy: fold chip text `+N` or `+N · K on`; name `N more tags` / `1 more tag`, plus `, K on`; tooltip `Show all tags`; panel name `All tags`.

## Review Focus

- A filter click recreates the observer if the effect keys on `tags()` (a new array on every filter echo): key the effect on `tags().join(" ")`. Test: Task 1 checks one observer across a chip click.
- The panel stays open, or reopens empty, after every chip comes back on the line. Test: Task 2, "closes when nothing is folded".
- Chips in the panel stay enabled during a run. Test: Task 2, "disabled during a run".
- The e2e chip helper in `virtual.spec.ts` clicks a chip that is now folded. Task 3 moves that click into the panel.
- Clear, the counter or Hide non-matching wrap inside a full bar. Task 1 gives them `shrink-0 whitespace-nowrap`; the e2e height check of Task 3 catches a second line.

---

### Task 1: Fold the chips past the line

**Files:**
- Modify: `src/renderer/FilterBar.tsx`
- Test: `src/renderer/FilterBar.test.tsx`

**Interfaces:**
- Produces: test ids `tag-strip` (the clipped chip strip), `tag-more` (the fold chip); a local `TagChip(props: { tag: string; folded?: boolean })` component in `FilterBar.tsx` that Task 2 reuses for the panel; signals `folded: Accessor<ReadonlySet<string>>`.

- [ ] **Step 1: Write the failing tests**

At the top of `FilterBar.test.tsx`, a stand-in observer installed for the whole file (`beforeEach(() => vi.stubGlobal("IntersectionObserver", FakeIO))`, `afterEach(vi.unstubAllGlobals)`), so existing tests see nothing folded:

```ts
class FakeIO {
  static all: FakeIO[] = [];
  targets: Element[] = [];
  constructor(
    public cb: IntersectionObserverCallback,
    public opts?: IntersectionObserverInit,
  ) {
    FakeIO.all.push(this);
  }
  observe(el: Element) { this.targets.push(el); }
  disconnect() { this.targets = []; }
}
const io = () => FakeIO.all.at(-1)!;
/** The browser's report: the chips of `tags` past the line, every other chip on it. */
const fold = (...tags: string[]) => {
  io().cb(
    io().targets.map((t) => ({ target: t, isIntersecting: !tags.includes((t as HTMLElement).dataset.tag!) })) as unknown as IntersectionObserverEntry[],
    io() as unknown as IntersectionObserver,
  );
  flush();
};
const more = () => screen.queryByTestId("tag-more");
```

New `describe("folding", ...)` tests (clean sample: slow, smoke, wip):

- `"folds the chips past the line behind a count"`: `load(); fold("smoke", "wip")` → `io().opts!.root === screen.getByTestId("tag-strip")`; `io().targets` are the three chips; `more()!.textContent === "+2"`; `more()!.getAttribute("aria-label") === "2 more tags"`; `chip("smoke").className` and `chip("wip").className` contain `"invisible"`, `chip("slow").className` does not.
- `"counts the folded tags that are on"`: `load({ include: [], exclude: ["wip"], hideNonMatching: false }); fold("smoke", "wip")` → text `"+2 · 1 on"`, name `"2 more tags, 1 on"`. Then `fold("wip")` → text `"+1 · 1 on"`, name `"1 more tag, 1 on"`.
- `"has no fold chip when every chip fits"`: `load(); fold()` → `more()` is null; `screen.getByTestId("filter-bar").className` does not contain `"flex-wrap"`; the strip's class contains `"flex-wrap"`, `"h-6"`, `"overflow-clip"`.
- `"keeps one observer while only the filter changes"`: `load(); fold("wip"); const n = FakeIO.all.length;` then `send({ type: "filter", filter: smokeNotWip() })` → `FakeIO.all.length === n` and `more()!.textContent === "+1 · 1 on"`.
- `"shows the next click on the fold chip in a tooltip"`: fake timers as in the existing tooltip test; pointer rests on `more()!.parentElement` 500 ms → tooltip text `"Show all tags"`.

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/renderer/FilterBar.test.tsx`
Expected: the five new tests FAIL (no `tag-strip`, no `tag-more`); the existing ones PASS.

- [ ] **Step 3: Implement in `FilterBar.tsx`**

- Move the chip button and its `Tooltip` into `TagChip(props: { tag: string; folded?: boolean })` inside `FilterBar` (it closes over `stateOf`, `locked`, `plain`, `props.api`); `folded` adds `invisible` to the button's class.
- Bar: drop `flex-wrap`; add `style={{ "anchor-name": "--filter-bar" }}` (used by Task 2).
- Strip: `<div ref={strip} data-testid="tag-strip" class="flex h-6 min-w-0 flex-wrap gap-1.5 overflow-clip [overflow-clip-margin:4px]">`, the 4 px clip margin keeping the 2 px + 2 px focus ring (R1.4); the 6 px row gap keeps the second line outside it.
- Observer: `createEffect(() => tags().join(" "), (key) => { ... })` that returns early when `!key || !strip || typeof IntersectionObserver === "undefined"`, else clears a local `Set<string>`, creates `new IntersectionObserver(cb, { root: strip })`, observes every `[data-tag]` in the strip, and returns `() => io.disconnect()`. `cb` adds or deletes `dataset.tag` per entry by `isIntersecting`, then `setFolded(new Set(set))`.
- Fold chip after the strip, inside `<Show when={folded().size > 0}>`: `<Tooltip text="Show all tags" class="shrink-0">` around `<button type="button" data-testid="tag-more" class={chip} aria-label=… >`; K = folded tags whose `stateOf` is not `"off"`.
- Clear: add `shrink-0`; the status span and the label: `shrink-0 whitespace-nowrap`.

- [ ] **Step 4: Run the file**

Run: `npx vitest run src/renderer/FilterBar.test.tsx`
Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add src/renderer/FilterBar.tsx src/renderer/FilterBar.test.tsx
git commit -m "feat(ui): fold the filter chips past one line behind a count"
```

### Task 2: The panel of every tag

**Files:**
- Modify: `src/renderer/FilterBar.tsx`
- Test: `src/renderer/FilterBar.test.tsx`

**Interfaces:**
- Consumes: `TagChip`, `folded`, the `--filter-bar` anchor name, the `fold` / `more` test helpers (Task 1).
- Produces: test id `tag-panel` (`id="tag-panel"`, `popover="auto"`); the fold chip gets `popovertarget="tag-panel"` and `aria-expanded`.

- [ ] **Step 1: Write the failing tests**

Helpers: `const panel = () => screen.getByTestId("tag-panel");` and, since jsdom has no Popover API, the browser's toggle as an event:

```ts
const toggle = (newState: "open" | "closed") => {
  panel().dispatchEvent(Object.assign(new Event("toggle"), { newState }));
  flush();
};
const panelChips = () => [...panel().querySelectorAll<HTMLButtonElement>("[data-testid=tag-chip]")];
```

- `"opens a panel of every tag that cycles them"`: `load(); fold("smoke", "wip")` → `more()!.getAttribute("popovertarget") === "tag-panel"`; `panel().getAttribute("popover") === "auto"`; `panel().getAttribute("aria-label") === "All tags"`; `panelChips()` is empty and `aria-expanded` is `"false"`. `toggle("open")` → `aria-expanded` `"true"`; `panelChips().map((c) => c.textContent)` is `["@slow", "@smoke", "@wip"]`; none has `invisible`. `fireEvent.click(panelChips()[1])` → `api.setFilter` called with `{ include: ["smoke"], exclude: [], hideNonMatching: false }`; `panelChips()` still has 3. `toggle("closed")` → empty, `"false"`.
- `"closes the panel when nothing is folded"`: `load(); fold("wip"); toggle("open"); fold()` → `more()` is null, `panelChips()` empty.
- `"disables the panel's chips during a run"`: `load(); fold("wip"); send({ type: "busy", busy: true }); toggle("open")` → every `panelChips()` disabled; `more()!.disabled === false`.

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/renderer/FilterBar.test.tsx`
Expected: the three new tests FAIL (no `tag-panel`).

- [ ] **Step 3: Implement in `FilterBar.tsx`**

- `const [open, setOpen] = createSignal(false)`; `let panel: HTMLDivElement | undefined`.
- Fold chip: `popovertarget="tag-panel"`, `aria-expanded={open() ? "true" : "false"}`.
- After the fold chip, always mounted inside the bar: `<div ref={panel} id="tag-panel" data-testid="tag-panel" popover="auto" role="group" aria-label="All tags" onToggle={(e) => setOpen(e.newState === "open")} style={{ "position-anchor": "--filter-bar", inset: "anchor(bottom) anchor(right) auto anchor(left)" }} class="m-0 w-auto max-h-[50vh] overflow-auto rounded-b-lg border border-line bg-raised p-2 shadow-float">` holding `<Show when={open()}><div class="flex flex-wrap gap-1.5"><For each={tags()}>{(tag) => <TagChip tag={tag} />}</For></div></Show>`.
- In the observer callback, once `set.size === 0` and `open()`: `panel?.hidePopover?.()` and `setOpen(false)`.

- [ ] **Step 4: Run the file, typecheck, lint**

Run: `npx vitest run src/renderer/FilterBar.test.tsx src/renderer/palette.test.ts`, `npm run typecheck`, `npm run lint`
Expected: all PASS, no errors.

- [ ] **Step 5: Commit**

```bash
git add src/renderer/FilterBar.tsx src/renderer/FilterBar.test.tsx
git commit -m "feat(ui): open every tag in a panel from the fold chip"
```

### Task 3: The bench project in the built app

**Files:**
- Modify: `e2e/virtual.spec.ts`

**Interfaces:**
- Consumes: `tag-strip`, `tag-more`, `tag-panel` (Tasks 1, 2); `openBig()`, `tree()` of `virtual.spec.ts`.

- [ ] **Step 1: Write the e2e test** `"The filter bar keeps one line with 53 tags"` (spec R1 to R3)

- `bar` height ≤ 37 px (24 px chip + 2 × 6 px padding + 1 px border).
- `hidden()` = count of `[data-testid=tag-strip] [data-testid=tag-chip].invisible`, > 0; `tag-more` has text `+${hidden}`.
- The tree's top before opening = after opening the panel (`tree(win).boundingBox()`).
- Click `tag-more` → `tag-panel` holds 53 `tag-chip`; click its `[data-tag="module-5-5"]` → `data-state` `include`; `expect.poll` of `tag-more`'s text equals `` `+${await hidden()} · 1 on` ``.
- Press Escape → `tag-panel` hidden, `tag-more` focused.

- [ ] **Step 2: Move the folded chip click of `"A filter change scrolls to the selected row"` into the panel**

Replace the `chip` helper with `panelChip(win, tag)` = `win.getByTestId("tag-panel").locator(`[data-tag="${tag}"]`)`; click `tag-more`, click `panelChip(win, "module-5-5")`, check `include`, press Escape, then go on as before.

- [ ] **Step 3: Run it** (after the parent session's go; e2e needs `vendor/`)

Run: `npx playwright test e2e/virtual.spec.ts` once packaged by `npm run e2e`, or the full `npm run e2e`.
Expected: all PASS.

- [ ] **Step 4: Commit**

```bash
git add e2e/virtual.spec.ts
git commit -m "test(e2e): fold the bench project's 53 tags into one line"
```
