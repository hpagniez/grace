// Option 3 probe: what a page holding the shell-java capability (tauri.plugins.conf.json) can run.
// Calls plugin:shell|execute through the page's own IPC, as any script in the page could.
//   SPIKE_EXE=<Grace.exe built with --features plugins> node e2e/shell-scope.mjs
import { execFileSync, spawn } from "node:child_process";
import { cpSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { chromium } from "playwright";

const exe = process.env.SPIKE_EXE;
const res = path.dirname(exe);
const dir = mkdtempSync(path.join(tmpdir(), "tauri-spike-shell-"));
const project = path.join(dir, "clean");
cpSync(path.join(import.meta.dirname, "../../../../test/fixtures/projects/clean"), project, { recursive: true });
const child = spawn(exe, [], {
  env: {
    ...process.env,
    SPIKE_PROJECT: project,
    SPIKE_DATA: path.join(dir, "data"),
    WEBVIEW2_USER_DATA_FOLDER: path.join(dir, "webview"),
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: "--remote-debugging-port=9555",
  },
  stdio: "ignore",
});
let browser;
while (!(browser = await chromium.connectOverCDP("http://127.0.0.1:9555").catch(() => null)))
  await new Promise((r) => setTimeout(r, 50));
let page;
while (!(page = browser.contexts()[0]?.pages()[0])) await new Promise((r) => setTimeout(r, 50));
await page.waitForSelector('[data-testid="tree"]', { timeout: 30_000 });

const karate = (cp, root) => [
  "-Dstdout.encoding=UTF-8", "-Dstderr.encoding=UTF-8", "-cp", cp, "io.karatelabs.Main", "run",
  "--no-color", "-w", root, "-o", path.join(dir, "out"), "-B", "false", "-f", "html,karate:jsonl",
  "-D", "-T", "1", root,
];
const cases = [
  ["Karate dry run of the project", "karate", karate(`${res}\\karate.jar;${project}`, project)],
  ["same shape, any folder and an extra classpath entry", "karate", karate(`C:\\Users\\Public\\karate.jar;C:\\Users\\Public`, "C:\\Users\\Public")],
  ["java -version (other shape)", "karate", ["-version"]],
  ["cmd.exe (not in scope)", "cmd", ["/c", "echo", "hi"]],
  ["taskkill of any PID", "taskkill", ["/PID", "999999", "/T", "/F"]],
];
for (const [label, program, args] of cases) {
  const r = await page.evaluate(
    ([program, args]) =>
      window.__TAURI_INTERNALS__
        .invoke("plugin:shell|execute", { program, args, options: {} })
        .then((o) => `allowed, exit ${o.code}`, (e) => `denied: ${String(e).slice(0, 120)}`),
    [program, args],
  );
  console.log(`${label}: ${r}`);
}
await browser.close().catch(() => {});
execFileSync("taskkill", ["/PID", String(child.pid), "/T", "/F"], { stdio: "ignore" });
await new Promise((r) => setTimeout(r, 1000));
rmSync(dir, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 });
