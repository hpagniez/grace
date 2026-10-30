// What the progress bar draws during Run all on the generated big project: every 500 ms, the drawn width of
// its green and red segments against the counts it reads out, once with the Console tab and once with the
// HTTP tab, with every animation that starts in the page (CDP's Animation domain). Skipped unless BAR_OUT=<file.jsonl>
// is set, so a round of perf.spec.ts and scroll.spec.ts is as before:
//   PERF_PROJECT=<dir> BAR_OUT=<file.jsonl> npx playwright test --config docs/checks/2026-10-30-virtual-lists-perf/playwright.config.ts bar.spec.ts
// BAR_FORCE=read or BAR_FORCE=spin adds work to every update (below); BAR_SHOT=<prefix> saves a picture of the bar.
import { appendFileSync } from "node:fs";
import { expect, launch, test } from "../../../e2e/helpers";

test("progress bar during Run all", async () => {
  test.skip(!process.env.BAR_OUT, "BAR_OUT not set");
  test.setTimeout(1_800_000);
  const dir = process.env.PERF_PROJECT!;
  const { app, win } = await launch();
  await app.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows()[0].setContentSize(1400, 900));
  await app.evaluate(({ dialog }, d) => {
    dialog.showOpenDialog = (async () => ({ canceled: false, filePaths: [d] })) as never;
  }, dir);
  await win.getByTestId("open-button").click();
  await expect(win.getByTestId("run-all-button")).toBeEnabled({ timeout: 300_000 });
  // As perf.spec.ts runs: the whole project selected, so the details pane and the HTTP tab show all of it.
  const top = win.locator('[data-testid=tree-node][data-key="d:tests"]');
  await top.locator("button:not([aria-label])").click();
  await expect(top.locator("xpath=..")).toHaveAttribute("aria-selected", "true");

  const result: Record<string, unknown> = {};
  for (const tab of ["console", "http"]) {
    await win.getByTestId(`tab-${tab}`).click();
    await win.evaluate((force) => {
      const w = window as unknown as { bar: unknown[]; barTimer: number; barForce?: MutationObserver };
      w.bar = [];
      // BAR_FORCE=read: a style read in every task that changes the counts, as a list or a console that measures
      // itself after an update does. It tests whether such reads hold a retargeted transition at its start.
      w.barForce?.disconnect();
      if (force === "read") {
        w.barForce = new MutationObserver(() => void document.body.offsetHeight);
        w.barForce.observe(document.querySelector("[data-testid=progress-text]")!, { characterData: true, childList: true, subtree: true });
      }
      // BAR_FORCE=spin: a new spinning icon in every task that changes the counts, as a tree row whose status
      // turns to running does. It tests whether a compositor animation started beside a retargeted transition
      // holds that transition at its start.
      if (force === "spin") {
        let spin = document.createElement("div");
        spin.className = "animate-spin";
        spin.style.cssText = "position:fixed;left:0;bottom:0;width:4px;height:4px";
        document.body.append(spin);
        w.barForce = new MutationObserver(() => spin.replaceWith((spin = spin.cloneNode() as HTMLDivElement)));
        w.barForce.observe(document.querySelector("[data-testid=progress-text]")!, { characterData: true, childList: true, subtree: true });
      }
      const text = (id: string) => document.querySelector(`[data-testid=${id}]`)?.textContent ?? "";
      w.barTimer = window.setInterval(() => {
        const bar = document.querySelector<HTMLElement>("[data-testid=progress-bar]")!;
        const [green, red] = [...bar.children] as HTMLElement[];
        const width = (e: HTMLElement) => Math.round(e.getBoundingClientRect().width * 10) / 10;
        w.bar.push({
          t: Date.now(),
          track: width(bar),
          green: width(green),
          red: width(red),
          greenStyle: green.style.width,
          redStyle: red.style.width,
          passed: Number(text("progress-passed")),
          failed: Number(text("progress-failed")),
          // CSS transitions on each segment: pending ones have no start time yet
          greenTransitions: green.getAnimations().map((a) => a.playState + (a.startTime === null ? ":pending" : "")),
          redTransitions: red.getAnimations().map((a) => a.playState + (a.startTime === null ? ":pending" : "")),
        });
      }, 500);
    }, process.env.BAR_FORCE ?? "");
    // Every animation started in the page during the run, by kind and name (CDP's Animation domain).
    const cdp = await win.context().newCDPSession(win);
    const started: Record<string, number[]> = {}; // ms after the click
    cdp.on("Animation.animationStarted", ({ animation: a }) => {
      const k = `${a.type} ${a.name || a.cssId || ""}`;
      (started[k] ??= []).push(Date.now() - t0);
    });
    let t0 = Date.now();
    await cdp.send("Animation.enable");
    t0 = Date.now();
    await win.getByTestId("run-all-button").click();
    // a picture of the bar half way, as a person sees it: once this run has 3,000 results
    await expect(win.getByTestId("progress-text")).toHaveText(/^\s*[3-9]\d{3} \//, { timeout: 300_000 });
    if (process.env.BAR_SHOT)
      await win.getByTestId("progress-bar").screenshot({ path: `${process.env.BAR_SHOT}.${tab}.png` });
    await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 1_500_000 });
    await win.waitForTimeout(1_000); // the last transition ends
    await cdp.detach();
    const samples = await win.evaluate((t0) => {
      const w = window as unknown as { bar: { t: number }[]; barTimer: number };
      clearInterval(w.barTimer);
      return w.bar.map((s) => ({ ...s, t: s.t - t0 }));
    }, t0);
    result[tab] = samples;
    result[`${tab}Animations`] = started;
    console.log(tab, JSON.stringify(started));
  }
  appendFileSync(process.env.BAR_OUT!, JSON.stringify(result) + "\n");
});
