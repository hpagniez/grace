// Retakes the README's screenshots in docs/images/ from the built app: `npm run screens:readme`.
// Never run it with the unit tests: the Java leak checks of the e2e helpers see every java.exe.
import type { Page } from "@playwright/test";
import { cpSync, mkdirSync, rmSync } from "node:fs";
import path from "node:path";
import { expect, launch, node, openProject, test } from "./helpers";

const WIDTH = 1440;
const HEIGHT = 900;
const IMAGES = path.resolve("docs", "images");

// The projects and app data folders go to the git-ignored out/, under a readable project name: their
// paths show in the toolbar and the console, and the public README must not hold the local user name.
const tmp = path.resolve("out", "readme-shots");
mkdirSync(tmp, { recursive: true });
process.env.TEMP = process.env.TMP = tmp;

/** A fresh copy of a fixture project at `out/readme-shots/<fixture>/petstore-api`. */
function petstore(fixture: "clean" | "http") {
  const dir = path.join(tmp, fixture, "petstore-api");
  rmSync(dir, { recursive: true, force: true });
  cpSync(path.resolve("test", "fixtures", "projects", fixture), dir, { recursive: true });
  return dir;
}

async function runAll(win: Page) {
  await win.getByTestId("run-all-button").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 90_000 });
}

for (const theme of ["light", "dark"] as const) {
  test(`README shots in the ${theme} theme`, async () => {
    test.setTimeout(300_000);
    const start = async (dir: string) => {
      const { app, win } = await launch(undefined, {}, { colorScheme: theme });
      await app.evaluate(
        ({ BrowserWindow }, [w, h]) => BrowserWindow.getAllWindows()[0].setContentSize(w, h),
        [WIDTH, HEIGHT],
      );
      await openProject(app, dir);
      await expect(win.getByTestId("run-all-button")).toBeEnabled({ timeout: 30_000 });
      return win;
    };
    const shoot = async (win: Page, name: string) => {
      await win.mouse.move(WIDTH / 2, 2); // above the toolbar's buttons: no hover, no tooltip
      await win.screenshot({
        path: path.join(IMAGES, `${name}-${theme}.png`),
        animations: "disabled",
        caret: "hide",
        scale: "css",
      });
    };

    const clean = await start(petstore("clean"));
    await runAll(clean);
    await node(clean, "failing match")
      .getByRole("button", { name: "failing match", exact: true })
      .click();
    await expect(clean.getByTestId("details-error")).toBeVisible();
    await shoot(clean, "run");

    await clean.getByTestId("rerun-failed-button").click();
    await expect(clean.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 90_000 });
    await clean.getByTestId("tab-history").click();
    await expect(clean.getByTestId("history-row")).toHaveCount(2);
    await shoot(clean, "history");

    const http = await start(petstore("http"));
    await runAll(http);
    await http.getByTestId("tab-http").click();
    await http
      .getByTestId("http-call")
      .filter({ has: http.getByTestId("http-call-status").getByText("404") })
      .first()
      .click();
    await expect(http.getByTestId("call-details")).toBeVisible();
    await shoot(http, "http");
  });
}
