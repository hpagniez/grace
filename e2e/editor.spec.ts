import type { ElectronApplication, Page } from "@playwright/test";
import { expect, launch, node, nodeByKey, openProject, project, test } from "./helpers";

/** Makes Windows' `vscode://` handler `name` ("" for none) and records the links handed to it. */
async function stubVsCode(app: ElectronApplication, name: string) {
  await app.evaluate(({ app: electronApp, shell }, name) => {
    const opened: string[] = [];
    (globalThis as { opened?: string[] }).opened = opened;
    shell.openExternal = async (url: string) => void opened.push(url);
    electronApp.getApplicationNameForProtocol = () => name;
  }, name);
  return () => app.evaluate(() => (globalThis as { opened?: string[] }).opened ?? []);
}

const select = (win: Page, name: string) =>
  node(win, name).getByRole("button", { name, exact: true }).click();

test("Open a scenario in VS Code", async () => {
  const dir = project("clean");
  const { app, win } = await launch();
  const opened = await stubVsCode(app, "Visual Studio Code");
  await openProject(app, dir);
  const feature = nodeByKey(win, "f:tests/basic.feature");
  await expect(feature).toBeVisible({ timeout: 15_000 });
  const link = `vscode://file/${dir.replaceAll("\\", "/")}/tests/basic.feature`;

  await select(win, "failing match");
  await win.getByTestId("open-in-editor").click();
  await expect.poll(opened).toEqual([`${link}:15`]);

  await feature.getByTestId("run-node").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 30_000 });
  await select(win, "failing match");
  // a long place that wraps reads as text, not centred
  await expect(win.getByTestId("error-place")).toHaveCSS("text-align", "left");
  await win.getByTestId("error-place").click();
  await expect.poll(opened).toEqual([`${link}:15`, `${link}:20`]);
});

test("main refuses what it must not open", async () => {
  const dir = project("clean");
  const { app, win } = await launch();
  const opened = await stubVsCode(app, "");
  await openProject(app, dir);
  await expect(nodeByKey(win, "f:tests/basic.feature")).toBeVisible({ timeout: 15_000 });
  const notice = win.getByTestId("notice");

  await select(win, "failing match");
  await win.getByTestId("open-in-editor").click();
  await expect(notice).toHaveText("Open in VS Code needs VS Code installed");

  const ask = (file: string, line: unknown) =>
    win.evaluate(
      ([f, l]) =>
        (window.api as unknown as Record<string, (...a: unknown[]) => Promise<void>>).openInEditor(
          f,
          l,
        ),
      [file, line] as const,
    );
  // Each refusal follows another notice, so a refusal that does not come fails its check.
  for (const line of [0, 1.5, "12"]) {
    await ask("C:\\Windows\\win.ini", 1);
    await expect(notice).toHaveText("Grace opens only files of the open project");
    await ask("tests/basic.feature", line);
    await expect(notice).toHaveText("Invalid request");
  }
  expect(await opened()).toEqual([]);
});
