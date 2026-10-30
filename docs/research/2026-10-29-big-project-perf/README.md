# Big-project performance: 2,000 and 10,000 tests in the premium UI

Measured on 2026-10-29 against the head of PR #116 (premium UI, issue #107, `55ac9e5e`, still a draft) and `develop` at `60c1e1b5`. All builds were packaged (`npm run package`): Electron 45, minified renderer. The machine was a Windows 11 desktop with a Ryzen 7 5800X (8 cores, 16 threads). Other agent sessions ran on it at the same time, so every comparison below ran the builds in turn, round after round. Figures are the median of 3 rounds at 2,000 tests and the range of 2 rounds at 10,000.

**Bottom line:** the per-row tooltips are not what makes Grace slow on a big project. Switched off, they save 35k of 123k DOM nodes and 14,856 event listeners. They take about 140 ms off expand-all, and nothing off scrolling or selecting. Three other things are slow, and each has a small fix:

1. **Run all.** On the 2,000-test project, Karate is done about 7 s after the click. Grace shows "Finished" after 18.6 s, or 40.9 s with the HTTP tab open. Until then the progress bar stalls and jumps, and scrolling stutters. The cause is per-event work in the renderer. Sending each 100 ms batch of events as one message brings both runs to about 7 s. With two memos for the HTTP tab and the scroll fixes added, the page keeps 76 fps during a run instead of 14.
2. **Scrolling** costs 8 times the main-thread work it did on `develop`. A `rotate` on every row's chevron makes each hit test slow. Hover transitions repaint the whole tree layer on every frame. A chevron icon swap and `content-visibility: auto` on rows fix both.
3. **Size.** Grace draws one DOM row per test. At 10,000 tests, even with every fix above, it holds over 500k DOM nodes and over 400 MB of JS heap, and freezes for up to 5 s during a run. A virtualized tree and HTTP list (a prototype here) keep the page at about 4k nodes. Run all then ends 1.2 s after Karate, with no frame slower than 383 ms.

Do the small fixes first, then virtualize. Neither a shared delegated tooltip nor a move to Tauri would help. Tauri 2 renders with WebView2 on Windows, the same Blink engine, and every cost measured here is in Grace's own code. The [Tauri spike](../2026-10-29-tauri-spike/README.md) found no gain in memory or start time either.

---

## 1. Method

- **Project.** [`generate.mjs`](generate.mjs) writes a Karate project with 8 area folders × 5 module folders × N features. Each feature holds 6 scenarios and an outline of 4 rows. It has feature and scenario tags (54 tags in all) and a few long names. Half the scenarios call a Karate mock that the suite starts once. 1 test in 50 fails. N = 5 gives 2,000 tests, 600 HTTP calls and 2,449 tree rows. N = 25 gives 10,000 tests, 3,000 calls and 12,049 rows. The Karate CLI alone runs the 2,000 tests in 3.5 s, JVM start excluded.
- **Probes.** [`perf.spec.ts`](perf.spec.ts) and [`scroll.spec.ts`](scroll.spec.ts) launch the built app through Playwright's `_electron`, the way `e2e/helpers.ts` does. Each probe clicks inside the page and stops at the first task after the next painted frame (click to paint). Chromium's `Performance.getMetrics` gives the renderer's busy time. A `PerformanceObserver` gives its long tasks, and a `requestAnimationFrame` loop gives the frame rate and the longest frame.
  - **Run all** is timed from the click to "Finished" shown. The probe also logs the progress bar's value at each frame and compares it with the depth-0 `SCENARIO_EXIT` times in Karate's own event file.
  - **Scroll** is 60 wheel notches of 100 px with the pointer over the tree, idle and during Run all. A Chromium trace splits the main thread's time into paint, prepaint, hit tests and the rest.
  - **Attribution** used CPU profiles of unminified builds, taken through the DevTools protocol.
- **Window.** 1400 × 900, set with `BrowserWindow.setContentSize`. Playwright's `setViewportSize` pins the page at its size inside a smaller window, which cuts the page off on the right and bottom. That artefact was mistaken for a Grace bug during this work.
- **Builds.**
  - `develop`
  - **PR head**: PR #116's head
  - **B–E**: PR head with the fixes of section 5, in [`fixes.diff`](fixes.diff)
  - **B–E + V**: the same plus the virtualization prototype, in [`virtual.diff`](virtual.diff)
  - **T** (measurement only): `Tooltip` renders only its children

## 2. Results at 2,000 tests

| | develop | PR head | B–E | B–E + V |
|---|---|---|---|---|
| Open: renderer busy / longest task | 737 / 244 ms | 1,286 / 465 ms | 882 / 473 ms | 311 / 91 ms |
| DOM nodes / listeners / JS heap after open | 57.8k / 30 / 59 MB | 123.5k / 14,856 / 91 MB | 123.4k / 14,856 / 87 MB | 2.5k / 337 / 16 MB |
| Expand all (2,449 rows) | 348 ms | 728 ms | 450 ms | 23 ms |
| Select a test | 63 ms | 54 ms | 35 ms | 11 ms |
| Run all, Console tab: click to "Finished" | 10.2 s | 18.6 s | 7.3 s | 6.7 s |
| … Karate's last test (same clock) | 6.7 s | 7.5 s | 7.0 s | 6.4 s |
| … progress bar behind Karate at the end | 3.2 s | 10.8 s | 0.11 s | 0.08 s |
| … frame rate / longest frame | 33 fps / 475 ms | 14 fps / 1,100 ms | 76 fps / 608 ms | 114 fps / 67 ms |
| Run all, HTTP tab open: click to "Finished" | 20.0 s | 40.9 s | 7.5 s | 6.6 s |
| … progress bar behind Karate at the end | 13.1 s | 31.6 s | 0.11 s | 0.09 s |
| … frame rate / longest frame | 21 fps / 442 ms | 9 fps / 1,083 ms | 70 fps / 717 ms | 116 fps / 58 ms |
| Open the HTTP tab (600 calls) | 61 ms | 135 ms | 118 ms | 15 ms |
| Idle scroll: main-thread paint / hit tests | 357 / 4 ms | 2,862 / 324 ms | 170 / 133 ms | 104 / 10 ms |
| Idle scroll: longest frame | 13 ms | 33 ms | 25 ms | 17 ms |
| Scroll during Run all: longest frame | 192 ms | 350 ms | 100 ms | 17 ms |

On the PR head, the progress bar falls further and further behind Karate. With the Console tab, it trails by 6.2 s at half way and 10.8 s at the end. The page also paints only about 14 frames a second, with gaps up to 1.1 s, so the bar sits still and then jumps. With the events batched, it stays 0.1 to 0.2 s behind Karate all the way.

## 3. Results at 10,000 tests

| | PR head | B–E | B–E + V |
|---|---|---|---|
| Open: renderer busy / longest task | 6.1 / 2.3 s | 3.4–4.1 / 2.2 s | 0.7–0.9 / 0.29 s |
| DOM nodes / JS heap after open | 527k / 414 MB | 527k–631k / 406–423 MB | 3.7k / 74 MB |
| Expand all (12,049 rows) | 3.5 s | 2.1–3.2 s | 49–52 ms |
| Select a test | 236 ms | 204–272 ms | 11 ms |
| Run all, Console tab: click to "Finished" | not finished (stopped) | 30.3–32.9 s | 19.6–20.2 s |
| … Karate's last test | | 20.4–20.6 s | 18.2–19.0 s |
| … progress bar behind Karate at the end | | 9.1–12.0 s | 0.7–1.2 s |
| … frame rate / longest frame | | 7–8 fps / 3.0–5.1 s | 36–39 fps / 342–383 ms |
| Run all, HTTP tab open: click to "Finished" | not run | 37.4–42.3 s | 18.7–20.2 s |
| … progress bar behind Karate at the end | | 16.3–20.7 s | 0.19–0.24 s |
| Open the HTTP tab (3,000 calls) | | 532–581 ms | 23 ms |
| Idle scroll: hit tests / longest frame | | 586–631 / 67–75 ms | 13 / 9–33 ms |
| Scroll during Run all: longest frame | | 467–483 ms | 25–33 ms |

The PR head's 10,000-test Run all was stopped by hand before it finished, so it has no figures. The fixes keep Grace usable at 2,000 tests but not at 10,000. There, every expand, select and status update touches all 12,049 rows. Only the virtualized build tracks Karate.

## 4. Why it is slow

**One IPC message per Karate event.** Main reads Karate's event file every 100 ms (`Runner`'s `poll`). It sends each event to the window as its own `app:event` message (`src/main.ts` `emit`). At 2,000 tests that is dozens of messages per poll. The store applies each one in its own task (`src/renderer/store.ts`), so Solid runs a full update per event. When those updates cost more than the time between events, the renderer builds a backlog. It is still draining that backlog long after Karate has exited.

**The HTTP tab's count walks every test on every change.** `narrowedCalls` (`src/renderer/http.ts`) walks every test under the tree selection. With nothing or the top folder selected, that is all 2,000 store nodes. It re-runs on every `calls` event, 600 per run. In PR #116 the count feeds `PanelTab`, which reads it twice per change: once for the pill's `Show when` and once for the text. So it runs at least 1,200 times per run, whichever tab is showing. In a CPU profile of the PR head (unminified, Console tab, top folder selected), 16.6 s of the 20.2 s of script time sat under that getter. With the HTTP tab open, `HttpCalls` makes a third pass. Each pass also builds new `ListedCall` objects, so all 600 rows re-render on every event.

**Folder status rolls up from scratch.** A folder's `status` memo (`deriveStatus`) walks its whole subtree on every status change. The top folder walks all 12,049 rows at 10,000 tests. This cost stays even in the virtualized build, and is the likely cause of the 383 ms frames left there. That was not profiled.

**Scrolling repaints the whole tree.** A trace of 60 wheel notches on the PR head showed:

- **Paint:** 2.9 s of main-thread paint over about 420 paints, against 0.36 s and 120 paints on `develop`.
- **Hit tests:** 324 ms, against 4 ms on `develop`.

Switching one thing off at a time at runtime (single runs, 60 notches, PR head):

| Variant | Paint | Hit tests | Longest frame |
|---|---|---|---|
| As built | 3.0 s | 364 ms | 42 ms |
| `prefers-reduced-motion` (all transitions off) | 1.0 s | 341 ms | 17 ms |
| No SVG icons in the tree | 1.6 s | 4 ms | 9 ms |
| Chevron without its `rotate` | 2.8 s | 6 ms | 25 ms |
| `content-visibility: auto` on tree rows | 0.27 s | 293 ms | 25 ms |
| Sheet without rounded clip / scroller with opaque background | no change | no change | no change |

- **Hit tests.** Every open folder's chevron has `rotate-90`, which Tailwind 4 writes as the `rotate` property. Each one is a transform, and every hit test must undo it.
- **Repaints.** Most likely the run button's `transition-colors`, which follows `group-hover`. As rows slide under a still pointer, each frame starts a colour transition somewhere in the tree. Turning transitions off cut paint by two thirds.
- **The rest.** Lucide's inline SVGs add prepaint and layerize work. `develop` drew status dots and text glyphs, with no SVG and no transition.

**The DOM grows with the suite.** A tree row of the PR head is about 50 DOM nodes and up to 4 `Tooltip` wrappers. That makes 123k nodes at 2,000 tests and 527k at 10,000. Opening, expand-all and selection grow with it. At 10,000 tests, selecting one test takes 236 ms: every row reads the selection, so a new selection updates all 12,049 rows.

## 5. Fixes measured

| Fix | What it does | Effect (2,000 tests unless said) | Size |
|---|---|---|---|
| A | Memo the HTTP tab's count in `BottomPanel` | Back to one count per change, as on `develop`. HTTP-tab run 33 s against 36 s for the PR head in the same rounds: within the noise | 3 lines |
| B | Memo the tests in scope (tree + selection only); one narrowed list for both the count and `HttpCalls` | With A: Console-tab run 9.9 s, HTTP-tab run 21.3 s, against 18–22 s and 44–49 s for the PR head across batches | ~25 lines |
| C | Make a test's `ListedCall` objects once per calls array (`WeakMap`), so unchanged rows keep their objects | With A and B: HTTP-tab run 9.7 s, against 44.1 s for the PR head in the same rounds | ~10 lines |
| D | Main sends the events of one tick as one message; the preload hands them to the store in one task | Alone: both runs 7.4–7.7 s, bar 0.1–0.2 s behind Karate; idle scroll unchanged | 12 lines |
| E | Tree rows `content-visibility: auto`; chevron swaps `ChevronRight` / `ChevronDown` instead of rotating | Idle scroll paint 2.9 → 0.17 s, hit tests 324 → 133 ms, expand-all 728 → 450 ms (B–E against the PR head; B, C and D do not change these) | 3 lines |
| V | Virtualize the tree (flat list of visible rows, collapse state in a `Set`) and the HTTP list (fixed-height rows) | See the B–E + V columns; at 10,000 tests about 4k DOM nodes | ~200 lines, prototype |
| T | `Tooltip` renders only its children (measurement only) | On top of A and B, in the same rounds: 123.5k → 88k nodes, 14,856 → 34 listeners, expand-all 833 → 691 ms, Console run 9.9 → 9.0 s, HTTP run 21.3 → 16.5 s. Scroll and select unchanged | |

`fixes.diff` holds A to E together, as measured. `virtual.diff` holds V on top of them. Both are against PR #116's head.

**The tooltip question.** The per-row wrappers cost DOM nodes and listeners in proportion to the rows drawn. They cost nothing measurable on scroll or selection. A shared delegated tooltip would save the 14,856 listeners and about 35k nodes. Virtualization already removes the wrappers of off-screen rows, so it is not worth doing on its own.

## 6. Recommendation

Open these as issues, in this order:

1. **Batch run events per tick (D).** It is the smallest change with the biggest effect: run time, progress bar, scrolling during a run. It needs a test that one IPC message carries many events, and an e2e check that the bar still reaches the total.
2. **HTTP tab memos (B, C) and the tree's scroll fixes (E).** Each is small and leaves the UI as it is. For E, the chevron loses its rotation animation, unless the rotation stays only on the row being toggled.
3. **Virtualize the tree and the HTTP list (V).** This is the only change that holds for "thousands of scenarios". It needs its own spec:
   - the tree's ARIA: `aria-level`, `aria-setsize` and `aria-posinset` on flat `treeitem`s
   - keyboard moves through rows that are not drawn
   - focus kept when its row scrolls out
   - scrolling to the selected test
   - e2e tests that count `tree-node` or `http-call` rows today
   - Find in page no longer seeing rows that are not drawn
4. **Then, if 10,000-test runs still show frames over 300 ms**, roll folder statuses up as each test's status changes, instead of walking subtrees.

Also seen, not performance: with 54 tags, the filter bar takes 4 to 6 lines (127–187 px) above the tree. It could keep one line and fold the rest.

## 7. Limits

- One machine, shared with other agent sessions. Interleaving the builds and taking medians keeps the comparisons fair, but the absolute times vary by up to 40 % between rounds. The fix rows of section 5 come from different batches of rounds; each compares against the PR head of its own batch.
- At 10,000 tests, only 2 rounds were run. The PR head's Run all there was stopped before it finished.
- The virtualized build is a prototype. It has none of the accessibility or test work listed above, and the e2e suite was not run on it or on the fix builds.
- Playwright's wheel events are steady 100 px notches, not a real trackpad or a high-resolution wheel.

## 8. Rerun

From the repo root of a checkout of the build to measure, after `npm install`, `npm run vendor` and `npm run package`:

```
node docs/research/2026-10-29-big-project-perf/generate.mjs <empty folder>        # 2,000 tests
node docs/research/2026-10-29-big-project-perf/generate.mjs <empty folder> 25     # 10,000 tests
```

Then set three environment variables and run Playwright with this folder's config:

- `PERF_PROJECT=<folder>`
- `PERF_OUT=<file>.jsonl` (one line per run of `perf.spec.ts`)
- `SCROLL_OUT=<file>.jsonl` (one line per run of `scroll.spec.ts`)

```
npx playwright test --config docs/research/2026-10-29-big-project-perf/playwright.config.ts
```

Optional variables:

- `PERF_PROFILE=<prefix>` saves a CPU profile of each run.
- `PERF_ONLY_CONSOLE=1` skips the HTTP-tab run.

Apply `fixes.diff`, then `virtual.diff`, to a checkout of `55ac9e5e` to rebuild the fix builds.
