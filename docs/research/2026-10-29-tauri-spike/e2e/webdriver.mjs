// The same run test through tauri-driver (W3C WebDriver, proxied to msedgedriver), with plain fetch:
// no WebdriverIO or Selenium. tauri-driver starts the app, so it gets SPIKE_PROJECT from here.
//   SPIKE_EXE=<Grace.exe> MSEDGEDRIVER=<msedgedriver.exe> node e2e/webdriver.mjs
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { cpSync, mkdtempSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import path from "node:path";

const WD = "http://127.0.0.1:4444";
const ELEMENT = "element-6066-11e4-a52e-4f735466cecf";
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function wd(method, url, body) {
  const res = await fetch(WD + url, {
    method,
    headers: { "content-type": "application/json" },
    body: body && JSON.stringify(body),
  });
  const json = await res.json();
  if (!res.ok) throw new Error(`${method} ${url}: ${JSON.stringify(json.value)}`);
  return json.value;
}

const dir = mkdtempSync(path.join(tmpdir(), "tauri-spike-wd-"));
const project = path.join(dir, "clean");
cpSync(path.join(import.meta.dirname, "../../../../test/fixtures/projects/clean"), project, { recursive: true });
const driver = spawn(
  path.join(homedir(), ".cargo", "bin", "tauri-driver.exe"),
  ["--native-driver", process.env.MSEDGEDRIVER],
  { env: { ...process.env, SPIKE_PROJECT: project, SPIKE_DATA: path.join(dir, "data") }, stdio: "inherit" },
);
for (let i = 0; i < 100 && !(await fetch(`${WD}/status`).catch(() => null)); i++) await sleep(100);

const started = Date.now();
const session = await wd("POST", "/session", {
  capabilities: {
    alwaysMatch: { browserName: "wry", "tauri:options": { application: process.env.SPIKE_EXE } },
  },
});
const s = `/session/${session.sessionId}`;
console.log(`session in ${Date.now() - started} ms`);

const byTestId = async (id) => {
  for (let i = 0; i < 300; i++) {
    const found = await wd("POST", `${s}/elements`, { using: "css selector", value: `[data-testid="${id}"]` });
    if (found.length) return found[0][ELEMENT];
    await sleep(100);
  }
  throw new Error(`no ${id}`);
};
const text = async (id) => wd("GET", `${s}/element/${await byTestId(id)}/text`);
const until = async (id, want, ms) => {
  for (const end = Date.now() + ms; Date.now() < end; await sleep(200)) {
    if (want.test(await text(id).catch(() => ""))) return;
  }
  throw new Error(`${id} never matched ${want}`);
};

try {
  await byTestId("tree");
  await wd("POST", `${s}/element/${await byTestId("run-all-button")}/click`, {});
  await until("run-outcome", /^Finished$/, 45_000);
  assert.equal(await text("progress-text"), "9 / 9");
  console.log("webdriver run test passed");
} finally {
  await wd("DELETE", s).catch(() => {});
  driver.kill();
}
