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

type Menus = { menus?: { items: { label: string; click(): void }[] }[] };

/** Replaces Windows' menu with one that keeps each menu shown; for the last one, its labels and a click on its item. */
async function stubMenu(app: ElectronApplication) {
  await app.evaluate(({ Menu }) => {
    const menus: unknown[] = [];
    (globalThis as { menus?: unknown[] }).menus = menus;
    Menu.prototype.popup = function () {
      menus.push(this);
    };
  });
  return {
    count: () => app.evaluate(() => ((globalThis as Menus).menus ?? []).length),
    labels: () =>
      app.evaluate(() => (globalThis as Menus).menus?.at(-1)?.items.map((i) => i.label)),
    choose: () => app.evaluate(() => (globalThis as Menus).menus!.at(-1)!.items[0].click()),
  };
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

test("Open a scenario from the tree", async () => {
  const dir = project("clean");
  const { app, win } = await launch();
  const opened = await stubVsCode(app, "Visual Studio Code");
  const menu = await stubMenu(app);
  await openProject(app, dir);
  const feature = nodeByKey(win, "f:tests/basic.feature");
  await expect(feature).toBeVisible({ timeout: 15_000 });
  const link = `vscode://file/${dir.replaceAll("\\", "/")}/tests/basic.feature`;
  const label = node(win, "failing match").getByRole("button", {
    name: "failing match",
    exact: true,
  });
  const item = node(win, "failing match").locator("..");

  await label.click({ button: "right" });
  await expect.poll(menu.labels).toEqual(["Open in VS Code"]);
  await expect(item).toHaveAttribute("aria-selected", "false");
  await menu.choose();
  await expect.poll(opened).toEqual([`${link}:15`]);

  await feature.getByTestId("run-node").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 30_000 });
  await label.click({ button: "right" });
  await expect.poll(menu.count).toBe(2);
  await menu.choose();
  await expect.poll(opened).toEqual([`${link}:15`, `${link}:20`]);

  // a tooltip lying on the row is the row's
  await node(win, "failing match").getByTestId("run-node").hover();
  await win.getByRole("tooltip").click({ button: "right" });
  await expect.poll(menu.count).toBe(3);
  await menu.choose();
  await expect.poll(opened).toHaveLength(3);
  expect((await opened())[2]).toBe(`${link}:20`);

  // the menu key's other form, on the focused row
  await item.focus();
  await win.keyboard.press("Shift+F10");
  await expect.poll(menu.count).toBe(4);
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

  const menu = await stubMenu(app);
  const askMenu = (args: unknown[]) =>
    win.evaluate(
      (a) =>
        (window.api as unknown as Record<string, (...a: unknown[]) => Promise<void>>).editorMenu(
          ...a,
        ),
      args,
    );
  for (const args of [
    ["tests/basic.feature", 0, 1, 1],
    ["tests/basic.feature", 1, "12", 4],
  ]) {
    await ask("C:\\Windows\\win.ini", 1);
    await expect(notice).toHaveText("Grace opens only files of the open project");
    await askMenu(args);
    await expect(notice).toHaveText("Invalid request");
  }
  expect(await menu.count()).toBe(0);
});
