// Never run unit tests and e2e tests at the same time: ourJavaPids() counts every java.exe from vendor\jre.
import {
  test as base,
  expect,
  _electron,
  type ElectronApplication,
  type Page,
} from "@playwright/test";
import { execFileSync } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
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
export function project(name: "clean" | "broken" | "maven" | "tags" | "env"): string {
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
