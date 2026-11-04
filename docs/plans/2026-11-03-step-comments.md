# Step Comments in Details Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Details shows a step's `#` comments above it, as the feature file has them.

**Architecture:** `Step` and `FileStep` get an optional `comments: string[]`. `extractSteps` copies Karate's `step.comments`; `fileSteps` collects `#` lines with Karate's rule. `StepRow` draws them above the row. The run's masker already walks every string of the `steps` event.

**Tech Stack:** SolidJS 2, Vitest (Node and jsdom), Playwright `_electron`.

**Spec:** [docs/specs/2026-11-03-step-comments.md](../specs/2026-11-03-step-comments.md).

## Global Constraints

- A step with no comment has no `comments` field and no extra element (R1.5, R2.2).
- Comments are kept as Karate gives them: trimmed, `#` kept (R1.1).
- Colours only through semantic tokens (`text-muted`); no `title=`, no raw palette class.

---

### Task 1: Comments in the steps main sends

**Files:** `src/shared/events.ts`, `src/main/karate/steps.ts`, `src/main/karate/steps.test.ts`, `src/main/mask.test.ts`

- [ ] Add `comments?: string[]` to `Step` and `FileStep`, with a doc comment.
- [ ] Tests first: `extractSteps` on `run-clean.jsonl` gives line 20 `["# expected greeting is wrong on purpose"]` and line 18 no field; a synthetic Background step result keeps its comments. `fileSteps` attaches two comments across a blank line, keeps a Background step's, and attaches none above a tag, `Scenario:`, after a last step, before a doc string, between table rows or inside a doc string. `maskEvent` masks a step comment.
- [ ] `extractSteps`: `comments: r.step.comments`, only when present.
- [ ] `fileSteps`: a pending list; a `#` line outside a doc string adds to it, a blank line keeps it, a step line takes it, any other line empties it.
- [ ] `npx vitest run src/main/karate/steps.test.ts src/main/mask.test.ts`, then commit `feat(runner): keep a step's comments in its steps`.

### Task 2: Comments above the step in Details

**Files:** `src/renderer/Details.tsx`, `src/renderer/Details.test.tsx`

- [ ] Tests first: a run step with two comments shows them above its row in `data-testid="step-comments"`, muted and mono; a step with none has no such element; a file step that is paused at shows its comment.
- [ ] `StepRow`: a `Show` on `props.step.comments` drawing one line each, `ml-6`, `font-mono text-xs text-muted break-words`.
- [ ] `npx vitest run src/renderer/Details.test.tsx`, then commit `feat(ui): show a step's comments above it in details`.

### Task 3: End to end

**Files:** `e2e/details.spec.ts`

- [ ] In "Failure details", select "failing match" before the run and check the step at line 20 shows the comment; after the run, check the failed step shows it and the error box has it between the place and the step.
- [ ] Typecheck, lint; `npm run e2e` after the parent's go; commit `test(e2e): check step comments before and after a run`.
