import type { ElectronApplication } from "@playwright/test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { emptyProject, expect, launch, test } from "./helpers";

interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** The first window's state. A minimized window is not visible. */
const state = (app: ElectronApplication) =>
  app.evaluate(({ BrowserWindow }) => {
    const w = BrowserWindow.getAllWindows()[0];
    return {
      bounds: w.getBounds(),
      maximized: w.isMaximized(),
      minimized: w.isMinimized(),
      visible: w.isVisible(),
    };
  });

/** The first window's state, once it is shown. */
async function shown(app: ElectronApplication) {
  await expect.poll(async () => (await state(app)).visible).toBe(true);
  return state(app);
}

const near = (actual: Rect, want: Rect) => {
  for (const k of ["x", "y", "width", "height"] as const)
    expect(Math.abs(actual[k] - want[k]), `${k}: ${actual[k]} ≈ ${want[k]}`).toBeLessThanOrEqual(1);
};

const setBounds = (app: ElectronApplication, b: Rect) =>
  app.evaluate(({ BrowserWindow }, b) => BrowserWindow.getAllWindows()[0].setBounds(b), b);

/** Ends a move or resize the way the person's drag does. Moves done in code fire no `moved`. */
const moved = (app: ElectronApplication) =>
  app.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows()[0].emit("moved"));

const call = (app: ElectronApplication, method: "maximize" | "minimize" | "unmaximize") =>
  app.evaluate(({ BrowserWindow }, m) => BrowserWindow.getAllWindows()[0][m](), method);

const savedWindow = (userData: string) => {
  try {
    return JSON.parse(readFileSync(path.join(userData, "settings.json"), "utf8")).window;
  } catch {
    return undefined; // not written yet
  }
};

/** A fresh app data folder whose settings.json holds this window state. */
function withSaved(window: unknown): string {
  const userData = emptyProject();
  writeFileSync(path.join(userData, "settings.json"), JSON.stringify({ window }));
  return userData;
}

let first: { bounds: Rect; workArea: Rect } | undefined;

/** The bounds of a first start with a fresh app data folder, and the main screen's work area. */
async function firstStart() {
  if (!first) {
    const { app } = await launch();
    const { bounds } = await shown(app);
    const workArea = await app.evaluate(({ screen }) => screen.getPrimaryDisplay().workArea);
    await app.close();
    first = { bounds, workArea };
  }
  return first;
}

/** 700×500, 40 px right and 30 px down from the work area's corner: fits CI's small screen. */
const inside = (wa: Rect): Rect => ({ x: wa.x + 40, y: wa.y + 30, width: 700, height: 500 });

async function restart(app: ElectronApplication, userData: string) {
  await app.close();
  return (await launch(userData)).app;
}

test("the window comes back where it was", async () => {
  const { app, userData } = await launch();
  await shown(app);
  const want = inside(await app.evaluate(({ screen }) => screen.getPrimaryDisplay().workArea));
  await setBounds(app, want);

  const s = await shown(await restart(app, userData));
  near(s.bounds, want);
  expect(s.maximized).toBe(false);
});

test("a maximized window comes back maximized", async () => {
  const { workArea } = await firstStart();
  const { app, userData } = await launch();
  await shown(app);
  await setBounds(app, inside(workArea));
  await call(app, "maximize");

  const again = await restart(app, userData);
  expect((await shown(again)).maximized).toBe(true);
  await call(again, "unmaximize");
  await expect.poll(async () => (await shown(again)).maximized).toBe(false);
  near((await shown(again)).bounds, inside(workArea));
});

test("a move by the person is saved before the app closes", async () => {
  const { workArea } = await firstStart();
  const { app, userData } = await launch();
  await shown(app);
  await setBounds(app, inside(workArea));
  await moved(app);
  await expect.poll(() => savedWindow(userData)).toEqual({ ...inside(workArea), maximized: false });
});

test("a window closed while minimized opens normal", async () => {
  const { workArea } = await firstStart();
  const { app, userData } = await launch();
  await shown(app);
  await setBounds(app, inside(workArea));
  await call(app, "maximize");
  await call(app, "minimize");
  await expect.poll(async () => (await state(app)).minimized).toBe(true);

  const s = await shown(await restart(app, userData));
  expect(s.minimized).toBe(false);
  expect(s.maximized).toBe(false);
  near(s.bounds, inside(workArea));
});

test("a failed save does not stop the app from closing", async () => {
  const { bounds, workArea } = await firstStart();
  const userData = emptyProject();
  mkdirSync(path.join(userData, "settings.json"));
  const { app } = await launch(userData);
  let stderr = "";
  app.process().stderr?.on("data", (d) => (stderr += d));
  near((await shown(app)).bounds, bounds);
  await setBounds(app, inside(workArea));
  await moved(app);
  await expect.poll(() => stderr).toContain("EISDIR");
  await app.close(); // an uncaught throw would leave Electron's error dialog open and time out here
});

test("a short saved window is held at the minimum height", async () => {
  const { workArea } = await firstStart();
  const userData = withSaved({ ...inside(workArea), height: 300, maximized: false });
  const { app } = await launch(userData);
  expect((await shown(app)).bounds.height).toBe(480);
});

test("a broken saved state opens the window as on a first start", async () => {
  const { bounds } = await firstStart();
  for (const window of [
    "abc",
    { x: 10, y: 10, width: -5, height: 500, maximized: false },
    { x: "a", y: 10, width: 700, height: 500, maximized: false },
    { x: 10, y: 10, width: 700, height: 500 },
  ]) {
    const { app } = await launch(withSaved(window));
    const s = await shown(app);
    near(s.bounds, bounds);
    expect(s.maximized).toBe(false);
    await app.close();
  }
});

test("a window partly off the screen is pulled onto it", async () => {
  const { workArea: wa } = await firstStart();
  const saved = { x: wa.x + wa.width - 350, y: wa.y + 30, width: 700, height: 500 };
  const { app } = await launch(withSaved({ ...saved, maximized: false }));
  near((await shown(app)).bounds, { ...saved, x: wa.x + wa.width - 700 });
});

test("a window bigger than the screen is shrunk to fit", async () => {
  const { workArea: wa } = await firstStart();
  const saved = { x: wa.x, y: wa.y, width: wa.width + 500, height: 500 };
  const { app } = await launch(withSaved({ ...saved, maximized: false }));
  near((await shown(app)).bounds, { ...saved, width: wa.width });
});

test("a window on a screen that is gone opens on the main screen", async () => {
  const { bounds } = await firstStart();
  const { app } = await launch(
    withSaved({ x: -30000, y: -30000, width: 700, height: 500, maximized: false }),
  );
  near((await shown(app)).bounds, bounds);
});

test("a maximized window on a screen that is gone opens maximized on the main screen", async () => {
  const { app } = await launch(
    withSaved({ x: -30000, y: -30000, width: 700, height: 500, maximized: true }),
  );
  expect((await shown(app)).maximized).toBe(true);
  expect(
    await app.evaluate(({ BrowserWindow, screen }) => {
      const w = BrowserWindow.getAllWindows()[0];
      return screen.getDisplayMatching(w.getBounds()).id === screen.getPrimaryDisplay().id;
    }),
  ).toBe(true);
});
