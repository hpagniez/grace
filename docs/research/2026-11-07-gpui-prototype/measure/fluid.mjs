// The frame probe: the longest gap between frames that draw the tree while the mouse wheel turns over it, one notch
// every 16 ms for 60 notches (as bench/probes.mjs `scrollTree`), idle and during Run all. Both apps get the same X wheel
// events from xdotool. Electron's frames are its requestAnimationFrame callbacks (the bench's frame log); the
// prototype's are the frames in which its tree list lays out its rows. Same flags as measure.mjs, plus --rounds.
import { execFileSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import { setTimeout as sleep } from "node:timers/promises";
import { log, opt, sample, startElectron, startGpui } from "./measure.mjs";

const ROUNDS = Number(opt.rounds ?? 5);
const xdo = (...a) => execFileSync("xdotool", a.map(String));
/** How long xdotool took to send the notches: the input rate both apps got. */
const wheelMs = [];

async function wheel(notches = 60) {
  xdo("mousemove", 300, 400);
  const t = Date.now();
  xdo("click", "--repeat", notches, "--delay", 16, 5); // wheel down, one notch per 16 ms
  wheelMs.push(Date.now() - t);
  await sleep(300);
}

const stats = (gaps) => {
  const s = [...gaps].sort((a, b) => a - b);
  const pick = (q) => s[Math.min(s.length - 1, Math.floor(q * s.length))] ?? null;
  return { frames: gaps.length + 1, maxMs: s.at(-1) ?? null, p95Ms: pick(0.95), medianMs: pick(0.5) };
};

async function electronFrames(app, fn) {
  await app.page.evaluate(() => {
    const w = /** @type {any} */ (window);
    w.__frames = [];
    w.__on = true;
    const tick = (t) => {
      if (!w.__on) return;
      w.__frames.push(t);
      requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  });
  await fn();
  const t = await app.page.evaluate(() => {
    const w = /** @type {any} */ (window);
    w.__on = false;
    return w.__frames;
  });
  return t.slice(1).map((x, i) => x - t[i]);
}

/** The prototype's list lays out its rows once or more per frame: calls under 2 ms apart are one frame. */
async function gpuiFrames(app, fn) {
  app.send("frames-start");
  await sleep(50);
  await fn();
  return (await app.frames()).filter((g) => g >= 2);
}

const results = [];
for (let r = 0; r <= ROUNDS; r++) {
  for (const name of r % 2 ? ["gpui", "electron"] : ["electron", "gpui"]) {
    const app = name === "electron" ? await startElectron({ cdp: true }) : await startGpui();
    try {
      await app.waitTree();
      await sleep(1500);
      const frames = name === "electron" ? electronFrames : gpuiFrames;
      const cpu0 = sample(app.pid).appCpu;
      const idle = stats(await frames(app, () => wheel()));
      idle.appCpuMs = sample(app.pid).appCpu - cpu0;
      xdo("click", 4); // scroll back a little
      await app.startRun();
      await sleep(500);
      const run = stats(await frames(app, () => wheel()));
      await app.waitRunEnd();
      results.push({ round: r, warmup: r === 0, app: name, idle, run });
      results.at(-1).wheelMs = wheelMs.slice(-2);
      log(`round ${r} ${name}: wheel ${wheelMs.slice(-2)} ms, idle ${JSON.stringify(idle)} run ${JSON.stringify(run)}`);
    } finally {
      await app.stop();
    }
  }
  writeFileSync(opt.out ?? "fluid.json", JSON.stringify(results, null, 2));
}
