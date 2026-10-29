// Playwright over CDP to WebView2: the Tauri app opens a debugging port through
// WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS and Playwright attaches with connectOverCDP.
//   SPIKE_EXE=<Grace.exe> [SPIKE_BACKEND=node] npx playwright test -c e2e
import { test, expect, chromium } from "@playwright/test";
import { execFileSync, spawn } from "node:child_process";
import { cpSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

const exe = process.env.SPIKE_EXE;
const jre = path.join(path.dirname(exe), "jre");
let port = 9400;

/** Java processes started from the app's own JRE: the e2e leak check of e2e/helpers.ts. */
const ourJava = () =>
  execFileSync(
    "powershell.exe",
    [
      "-NoProfile",
      "-Command",
      `@(Get-CimInstance Win32_Process -Filter "Name='java.exe'" | Where-Object { $_.ExecutablePath -like '${jre}\\*' }).Count`,
    ],
    { encoding: "utf8" },
  ).trim();

async function launch(withProject = true) {
  const dir = mkdtempSync(path.join(tmpdir(), "tauri-spike-e2e-"));
  const project = path.join(dir, "clean");
  cpSync(path.join(import.meta.dirname, "../../../../test/fixtures/projects/clean"), project, { recursive: true });
  const p = port++;
  const child = spawn(exe, [], {
    env: {
      ...process.env,
      ...(withProject ? { SPIKE_PROJECT: project } : {}),
      SPIKE_DATA: path.join(dir, "data"),
      WEBVIEW2_USER_DATA_FOLDER: path.join(dir, "webview"),
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${p}`,
    },
    stdio: "ignore",
  });
  let browser;
  for (let i = 0; !browser && i < 200; i++) {
    browser = await chromium.connectOverCDP(`http://127.0.0.1:${p}`).catch(() => null);
    if (!browser) await new Promise((r) => setTimeout(r, 50));
  }
  let page;
  while (!(page = browser.contexts()[0]?.pages()[0])) await new Promise((r) => setTimeout(r, 50));
  const close = async () => {
    await browser.close().catch(() => {});
    execFileSync("taskkill", ["/PID", String(child.pid), "/T", "/F"], { stdio: "ignore" });
    await new Promise((r) => setTimeout(r, 1000));
    rmSync(dir, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 });
  };
  return { page, close };
}

test("shows the home screen when no project is open", async () => {
  const { page, close } = await launch(false);
  try {
    await expect(page.getByTestId("home")).toBeVisible({ timeout: 15_000 });
    await expect(page.locator("#startup")).toHaveCount(0);
  } finally {
    await close();
  }
});

test("runs the project with live progress", async () => {
  const { page, close } = await launch();
  try {
    await expect(page.getByTestId("tree")).toBeVisible({ timeout: 15_000 });
    await page.getByTestId("run-all-button").click();
    await expect(page.getByTestId("progress-text")).toHaveText(/^\d+ \/ 9$/, { timeout: 15_000 });
    await expect(page.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 45_000 });
    await expect(page.getByTestId("progress-text")).toHaveText("9 / 9");
  } finally {
    await close();
  }
  expect(ourJava()).toBe("0");
});

test("cancels a run and leaves no Java behind", async () => {
  const { page, close } = await launch();
  try {
    await expect(page.getByTestId("tree")).toBeVisible({ timeout: 15_000 });
    await page.getByTestId("run-all-button").click();
    await expect(page.getByTestId("progress-text")).toHaveText(/^\d+ \/ 9$/, { timeout: 15_000 });
    await page.getByTestId("cancel-button").click();
    await expect(page.getByTestId("run-outcome")).toHaveText("Cancelled", { timeout: 15_000 });
    expect(ourJava()).toBe("0");
  } finally {
    await close();
  }
});
