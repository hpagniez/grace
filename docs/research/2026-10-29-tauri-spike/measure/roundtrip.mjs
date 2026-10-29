// Page-to-backend round trip: 500 sequential no-op calls (Cancel while nothing runs) from the page.
//   node roundtrip.mjs <exe> <electron|tauri> <project folder>
// Tauri's backend follows SPIKE_BACKEND (unset: Rust; node: the sidecar next to the exe).
import { execFileSync, spawn } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { chromium } from "playwright";

const [exe, kind, project] = process.argv.slice(2);
const PORT = 9334;
const data = mkdtempSync(path.join(tmpdir(), "tauri-spike-rt-"));
const env = { ...process.env };
const args = [];
if (kind === "electron") {
  writeFileSync(path.join(data, "settings.json"), JSON.stringify({ lastProject: project }));
  env.GRACE_USER_DATA = data;
  args.push(`--remote-debugging-port=${PORT}`);
} else {
  Object.assign(env, {
    SPIKE_PROJECT: project,
    SPIKE_DATA: data,
    WEBVIEW2_USER_DATA_FOLDER: path.join(data, "webview"),
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${PORT}`,
  });
}
const child = spawn(exe, args, { env, stdio: "ignore" });
let browser;
while (!(browser = await chromium.connectOverCDP(`http://127.0.0.1:${PORT}`).catch(() => null)))
  await new Promise((r) => setTimeout(r, 50));
let page;
while (!(page = browser.contexts()[0]?.pages()[0])) await new Promise((r) => setTimeout(r, 50));
await page.waitForSelector('[data-testid="tree"]', { timeout: 30_000 });

const ms = await page.evaluate(async (kind) => {
  const call =
    kind === "electron"
      ? () => window.api.cancel()
      : () => window.__TAURI_INTERNALS__.invoke("api", { channel: "run:cancel", args: [] });
  for (let i = 0; i < 50; i++) await call(); // warm-up
  const out = [];
  for (let i = 0; i < 500; i++) {
    const t = performance.now();
    await call();
    out.push(performance.now() - t);
  }
  return out.sort((a, b) => a - b);
}, kind);
const at = (q) => ms[Math.floor(q * (ms.length - 1))].toFixed(2);
console.log(`${kind}${process.env.SPIKE_BACKEND ? ` (${process.env.SPIKE_BACKEND})` : ""}: median ${at(0.5)} ms, p95 ${at(0.95)} ms`);

await browser.close().catch(() => {});
execFileSync("taskkill", ["/PID", String(child.pid), "/T", "/F"], { stdio: "ignore" });
await new Promise((r) => setTimeout(r, 1000));
rmSync(data, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 });
