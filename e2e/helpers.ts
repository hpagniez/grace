// Never run unit tests and e2e tests at the same time: ourJavaPids() counts every java.exe from vendor\jre.
import {
  test as base,
  expect,
  _electron,
  type ElectronApplication,
  type Page,
} from "@playwright/test";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { createRunDir, writeRunRecord } from "../src/main/history";
import type { RunRecord } from "../src/shared/history";
import { NO_FILTER } from "../src/shared/tags";
import { copyFixture } from "../test/fixtures/copy";

const apps: ElectronApplication[] = [];
const dirs: string[] = [];

const tempDir = (prefix: string) => {
  const dir = mkdtempSync(path.join(tmpdir(), prefix));
  dirs.push(dir);
  return dir;
};

/** Closes every app, then deletes every temp folder of the test, whatever its outcome. */
export const test = base.extend<{ cleanup: void }>({
  cleanup: [
    // oxlint-disable-next-line no-empty-pattern -- Playwright fixtures must destructure their first argument
    async ({}, use) => {
      await use();
      for (const app of apps.splice(0)) await app.close().catch(() => {});
      for (const dir of dirs.splice(0))
        rmSync(dir, { recursive: true, force: true, maxRetries: 5 });
    },
    { auto: true },
  ],
});
export { expect };

/** A fresh copy of a fixture project, deleted after the test. */
export function project(
  name: "clean" | "broken" | "maven" | "tags" | "env" | "history" | "http",
): string {
  const dir = copyFixture(name);
  dirs.push(dir);
  return dir;
}

/** An empty folder, deleted after the test. */
export const emptyProject = () => tempDir("kk-empty-");

/**
 * Starts the built app (`.vite/build`) with its own fresh userData folder, or the given one (close the
 * first app before), and `extraEnv` added to the app's environment.
 */
export async function launch(
  userDataDir?: string,
  extraEnv: Record<string, string> = {},
): Promise<{ app: ElectronApplication; win: Page; userData: string }> {
  const userData = userDataDir ?? tempDir("kk-userdata-");
  const app = await _electron.launch({
    args: ["."],
    env: { ...process.env, ...extraEnv, KARATE_KID_USER_DATA: userData },
  });
  apps.push(app);
  const win = await app.firstWindow();
  return { app, win, userData };
}

/** Makes the folder picker return `dir`, then clicks Open. */
export async function openProject(app: ElectronApplication, dir: string): Promise<void> {
  await app.evaluate(({ dialog }, d) => {
    dialog.showOpenDialog = (async () => ({ canceled: false, filePaths: [d] })) as never;
  }, dir);
  const win = await app.firstWindow();
  await win.getByTestId("open-button").click();
}

/** Replaces `shell.openPath` with a recorder; the returned function reads the recorded paths. */
export async function stubOpenPath(app: ElectronApplication): Promise<() => Promise<string[]>> {
  await app.evaluate(({ shell }) => {
    const calls: string[] = [];
    (globalThis as { openPathCalls?: string[] }).openPathCalls = calls;
    shell.openPath = async (p: string) => {
      calls.push(p);
      return "";
    };
  });
  return () => app.evaluate(() => (globalThis as { openPathCalls?: string[] }).openPathCalls ?? []);
}

const jreDir = path.resolve("vendor", "jre").toLowerCase() + path.sep;

/** Running java.exe processes started from this repo's bundled JRE, with their command lines. */
function ourJavaProcesses(): { pid: number; commandLine: string }[] {
  const out = execFileSync(
    "powershell",
    [
      "-NoProfile",
      "-NonInteractive",
      "-Command",
      `Get-CimInstance Win32_Process -Filter "Name='java.exe'" | ForEach-Object { "$($_.ProcessId)|$($_.ExecutablePath)|$($_.CommandLine)" }`,
    ],
    { encoding: "utf8", windowsHide: true },
  );
  return out
    .split(/\r?\n/)
    .map((l) => l.split("|"))
    .filter(([, exe]) => exe && exe.toLowerCase().startsWith(jreDir))
    .map(([pid, , ...rest]) => ({ pid: Number(pid), commandLine: rest.join("|") }));
}

/** PIDs of running java.exe processes started from this repo's bundled JRE. */
export const ourJavaPids = (): number[] => ourJavaProcesses().map((p) => p.pid);

/** Command lines of running java.exe processes started from this repo's bundled JRE. */
export const ourJavaCommandLines = (): string[] => ourJavaProcesses().map((p) => p.commandLine);

/** The tree row of the node whose ▶ button is named `Run <name>`. */
export const node = (win: Page, name: string) =>
  win
    .getByTestId("tree-node")
    .filter({ has: win.getByRole("button", { name: `Run ${name}`, exact: true }) });

/** The tree row with this key. */
export const nodeByKey = (win: Page, key: string) =>
  win.locator(`[data-testid=tree-node][data-key="${key}"]`);

/** `<userData>/projects/<hash>`: where main keeps the dry run, the arguments and the run folders of the project at `root`. */
export const projectDataDir = (userData: string, root: string) =>
  path.join(
    userData,
    "projects",
    createHash("sha1").update(path.resolve(root)).digest("hex").slice(0, 12),
  );

/**
 * Writes `n` valid run folders, one minute apart and all in the past (the last is the newest), into the
 * history of the project at `root`, before the app opens it. Each run is "Run all" on the clean sample
 * project that finished with "failing match" failed; `record` replaces fields of every record. A
 * `karate-summary.html` is added when `report` is true. Returns the ids, oldest first.
 */
export function seedRuns(
  userData: string,
  root: string,
  n: number,
  opts: { report?: boolean; record?: Partial<RunRecord> } = {},
): string[] {
  const projectDir = projectDataDir(userData, root);
  const now = Date.now();
  return Array.from({ length: n }, (_, i) => {
    const startedAt = now - (n - i) * 60_000;
    const { id, dir } = createRunDir(projectDir, new Date(startedAt));
    writeRunRecord(dir, {
      version: 1,
      startedAt,
      durationMillis: 5_000,
      env: null,
      filter: { ...NO_FILTER },
      selection: { kind: "folder", file: "", name: path.basename(root), line: null, key: "d:" },
      failedOf: null,
      outcome: "finished",
      exitCode: 1,
      tests: [
        {
          kind: "scenario",
          file: "tests/basic.feature",
          refId: "[1:7]",
          name: "passing hello",
          line: 7,
          key: "s:tests/basic.feature#1",
          status: "passed",
          durationMillis: 400,
        },
        {
          kind: "scenario",
          file: "tests/basic.feature",
          refId: "[1:15]",
          name: "failing match",
          line: 15,
          key: "s:tests/basic.feature#2",
          status: "failed",
          durationMillis: 300,
        },
      ],
      ...opts.record,
    });
    if (opts.report) writeFileSync(path.join(dir, "karate-summary.html"), "<html></html>");
    return id;
  });
}

export const SECRET = "s3cret-token-123";

/** The settings entry of the project, as main saved it. */
export const saved = (userData: string, dir: string) => {
  try {
    return JSON.parse(readFileSync(path.join(userData, "settings.json"), "utf8")).projects?.[
      path.resolve(dir)
    ];
  } catch {
    return undefined; // not written yet, or caught mid-write
  }
};

/** Opens the env sample project and waits for its tree. */
export async function openEnv(extraEnv?: Record<string, string>) {
  const dir = project("env");
  const { app, win, userData } = await launch(undefined, extraEnv);
  await openProject(app, dir);
  await expect(node(win, "always")).toBeVisible({ timeout: 15_000 });
  return { dir, app, win, userData };
}

/** Types the environment and presses Enter. Waits until main has saved it. */
export async function chooseEnv(win: Page, userData: string, dir: string, name: string) {
  const input = win.getByTestId("env-input");
  await input.fill(name);
  await input.press("Enter");
  await expect.poll(() => saved(userData, dir)?.env, { timeout: 30_000 }).toBe(name);
}

/** The env sample project, open with this environment. */
export async function openEnvWith(env: string) {
  const opened = await openEnv();
  await chooseEnv(opened.win, opened.userData, opened.dir, env);
  return opened;
}

export async function openProperties(win: Page) {
  await win.getByTestId("properties-button").click();
  await expect(win.getByTestId("properties-dialog")).toBeVisible();
}

export const scopeItem = (win: Page, scope: string) =>
  win.locator(`[data-testid=scope-item][data-scope="${scope}"]`);

/** Adds a row to the list of `scope` ("" is All environments) in the open dialog. */
export async function addProperty(
  win: Page,
  scope: string,
  name: string,
  value: string,
  secret = false,
) {
  await scopeItem(win, scope).click();
  await win.getByTestId("add-property").click();
  const row = win.getByTestId("property-row").last();
  await row.getByTestId("prop-name").fill(name);
  if (secret) await row.getByTestId("prop-secret").check();
  await row.getByTestId("prop-value").fill(value);
}

export async function saveProperties(win: Page) {
  await win.getByTestId("save-properties").click();
  await expect(win.getByTestId("properties-dialog")).toBeHidden();
}

/** Runs "always" and waits for the run to finish. */
export async function runAlways(win: Page) {
  await node(win, "always").getByTestId("run-node").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 45_000 });
}

/** The env sample project in "qa" with the secret "api.token" saved for it. */
export async function openWithSecret() {
  const opened = await openEnvWith("qa");
  await openProperties(opened.win);
  await addProperty(opened.win, "qa", "api.token", SECRET, true);
  await saveProperties(opened.win);
  await expect
    .poll(() => saved(opened.userData, opened.dir)?.properties?.byEnv?.qa)
    .toEqual([{ name: "api.token", secret: expect.any(String) }]);
  return opened;
}
