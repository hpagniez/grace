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
import { writeBigProject } from "./big-project";

const apps: ElectronApplication[] = [];
const dirs: string[] = [];

/** A new temp folder, deleted after the test. */
export const tempDir = (prefix: string) => {
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

/** A generated project of 8 x 5 x `featuresPerModule` features (5: 2,000 tests), deleted after the test. */
export function bigProject(featuresPerModule = 5): string {
  const dir = tempDir("kk-big-");
  writeBigProject(dir, featuresPerModule);
  return dir;
}

/**
 * A fresh copy of the flaky sample project, deleted after the test. `coin(heads)` writes the `coin.json`
 * that `tests/coin.feature` reads from beside the project folder: outside the project, so it does not
 * change the project's fingerprint.
 */
export function flakyProject(): { dir: string; coin(heads: boolean): void } {
  const dir = copyFixture("flaky");
  dirs.push(path.dirname(dir));
  return {
    dir,
    coin: (heads) =>
      writeFileSync(path.join(path.dirname(dir), "coin.json"), JSON.stringify({ heads })),
  };
}

/** An empty folder, deleted after the test. */
export const emptyProject = () => tempDir("kk-empty-");

/**
 * Starts the built app (`.vite/build`) with its own fresh userData folder, or the given one (close the
 * first app before), `extraEnv` added to the app's environment, and an emulated colour scheme if given.
 * Playwright emulates light unless told otherwise: `colorScheme: null` turns the emulation off, so the
 * page follows `nativeTheme` (the Appearance tests).
 * Its update check reads an empty feed, never GitHub, unless `extraEnv` gives `GRACE_UPDATE_FEED`.
 * On a machine with more than one screen, the window opens centred on one that is not the main one,
 * out of the way of someone working there, unless `primaryScreen` keeps it where the app put it.
 */
export async function launch(
  userDataDir?: string,
  extraEnv: Record<string, string> = {},
  opts: { colorScheme?: "light" | "dark" | null; primaryScreen?: boolean } = {},
): Promise<{ app: ElectronApplication; win: Page; userData: string }> {
  const userData = userDataDir ?? tempDir("kk-userdata-");
  const noFeed = path.join(tempDir("kk-feed-"), "releases.json");
  writeFileSync(noFeed, "[]");
  const app = await _electron.launch({
    args: ["."],
    env: { ...process.env, GRACE_UPDATE_FEED: noFeed, ...extraEnv, GRACE_USER_DATA: userData },
    colorScheme: opts.colorScheme,
  });
  apps.push(app);
  const win = await app.firstWindow();
  // The real mouse of someone using the machine would reach the window between Playwright's moves and
  // end a hover (a tooltip test then fails). Windows passes it through to what lies below; Playwright's
  // input goes to the page directly and still arrives.
  await app.evaluate(({ BrowserWindow, screen }, move) => {
    // the app shows its window once painted: moved now, it first appears on the other screen
    const other = screen.getAllDisplays().find((d) => d.id !== screen.getPrimaryDisplay().id);
    for (const w of BrowserWindow.getAllWindows()) {
      w.setIgnoreMouseEvents(true);
      if (move && other) {
        const { width, height } = w.getBounds();
        const a = other.workArea;
        w.setPosition(
          a.x + Math.max(0, Math.round((a.width - width) / 2)),
          a.y + Math.max(0, Math.round((a.height - height) / 2)),
        );
      }
    }
  }, !opts.primaryScreen);
  return { app, win, userData };
}

/**
 * Starts the built app with a fresh userData folder whose settings.json already holds `settings`,
 * `extraEnv` added to its environment, and `opts` as `launch` takes them.
 */
export async function launchWithSettings(
  settings: Record<string, unknown>,
  extraEnv?: Record<string, string>,
  opts?: Parameters<typeof launch>[2],
) {
  const userData = tempDir("kk-userdata-");
  writeFileSync(path.join(userData, "settings.json"), JSON.stringify(settings));
  return launch(userData, extraEnv, opts);
}

/** The parsed settings.json of an app data folder. */
export const settingsOf = (userData: string): Record<string, unknown> =>
  JSON.parse(readFileSync(path.join(userData, "settings.json"), "utf8"));

/** Sets the window to its default 1200 × 800, which Windows fits to a smaller screen (1024 × 768 on CI). */
export async function defaultWindow(app: ElectronApplication, win: Page): Promise<void> {
  const inner = await app.evaluate(({ BrowserWindow }) => {
    const w = BrowserWindow.getAllWindows()[0];
    w.setSize(1200, 800);
    return w.getContentSize();
  });
  await expect.poll(() => win.evaluate(() => [innerWidth, innerHeight])).toEqual(inner);
}

/** Makes the folder picker return `dir`, then clicks Open. */
export async function openProject(app: ElectronApplication, dir: string): Promise<void> {
  await app.evaluate(({ dialog }, d) => {
    dialog.showOpenDialog = (async () => ({ canceled: false, filePaths: [d] })) as never;
  }, dir);
  const win = await app.firstWindow();
  await win.getByTestId("open-button").click();
}

/**
 * Replaces the folder picker with a recorder that cancels; the returned function reads each call's
 * parent window (whether one was passed) and starting folder.
 */
export async function recordOpenDialog(
  app: ElectronApplication,
): Promise<() => Promise<{ parent: boolean; defaultPath?: string }[]>> {
  await app.evaluate(({ dialog }) => {
    const calls: { parent: boolean; defaultPath?: string }[] = [];
    (globalThis as { openDialogCalls?: typeof calls }).openDialogCalls = calls;
    dialog.showOpenDialog = (async (...args: unknown[]) => {
      const options = args[args.length - 1] as { defaultPath?: string };
      calls.push({ parent: args.length === 2, defaultPath: options.defaultPath });
      return { canceled: true, filePaths: [] };
    }) as never;
  });
  return () =>
    app.evaluate(
      () =>
        (globalThis as { openDialogCalls?: { parent: boolean; defaultPath?: string }[] })
          .openDialogCalls ?? [],
    );
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
      threads: 1,
      fingerprint: null,
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

/** `data.threads` of the first `SUITE_ENTER` line of a run's event stream: the count Karate ran with. */
export function ranThreads(userData: string, root: string, id: string): number {
  const events = path.join(
    projectDataDir(userData, root),
    "runs",
    id,
    "karate-json",
    "karate-events.jsonl",
  );
  const suite = readFileSync(events, "utf8")
    .split(/\r?\n/)
    .find((line) => line.includes('"SUITE_ENTER"'));
  return JSON.parse(suite!).data.threads;
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
