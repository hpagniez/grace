import { test, expect, _electron as electron } from '@playwright/test';

// Built app (.vite/build + .vite/renderer from `electron-forge package`), run with the dev Electron binary.
test('built app: router, Kobalte click, IPC, sandbox', async () => {
  const app = await electron.launch({ args: ['.'] });
  const win = await app.firstWindow();
  await expect(win.locator('h1')).toBeVisible();
  const before = await win.locator('[data-test=doubled]').textContent();
  await win.locator('[data-test=kobalte]').click();
  await expect(win.locator('[data-test=doubled]')).not.toHaveText(before ?? '');
  await expect(win.locator('[data-test=ticks]')).not.toHaveText(/\b0\b/);
  await win.locator('[data-test=run-link]').click();
  await expect(win).toHaveURL(/#\/runs\/42$/);
  expect(await win.evaluate(() => typeof (globalThis as any).require !== 'undefined' || typeof (globalThis as any).process !== 'undefined')).toBe(false);
  expect(await app.evaluate(({ app }) => app.isReady())).toBe(true);
  await app.close();
});

