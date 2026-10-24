import { app, BrowserWindow, ipcMain } from 'electron';
import path from 'node:path';
import started from 'electron-squirrel-startup';
import { IPC } from './ipc';

// Handle creating/removing shortcuts on Windows when installing/uninstalling.
if (started) {
  app.quit();
}

const t0 = Date.now();
const log = (...a: unknown[]) => console.log(`[main +${Date.now() - t0}ms]`, ...a);

ipcMain.handle(IPC.getVersions, () => process.versions);

const createWindow = () => {
  const mainWindow = new BrowserWindow({
    width: 800,
    height: 600,
    webPreferences: {
      preload: path.join(__dirname, 'preload.cjs'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
    },
  });

  // Forward renderer console to the main process stdout.
  mainWindow.webContents.on('console-message', (e) => log(`[renderer:${e.level}]`, e.message));
  mainWindow.webContents.on('did-navigate', (_e, url) => log('did-navigate', url));
  mainWindow.webContents.on('did-navigate-in-page', (_e, url) => log('did-navigate-in-page', url));
  mainWindow.webContents.on('preload-error', (_e, p, err) => log('PRELOAD ERROR', p, err));

  let count = 0;
  const timer = setInterval(() => mainWindow.webContents.send(IPC.tick, ++count), 500);
  mainWindow.on('closed', () => clearInterval(timer));

  if (MAIN_WINDOW_VITE_DEV_SERVER_URL) {
    mainWindow.loadURL(MAIN_WINDOW_VITE_DEV_SERVER_URL);
  } else {
    mainWindow.loadFile(path.join(__dirname, `../renderer/${MAIN_WINDOW_VITE_NAME}/index.html`));
  }

  if (process.env.SPIKE_AUTOCHECK) {
    mainWindow.webContents.once('did-finish-load', () => void autocheck(mainWindow));
  }
};

// Spike-only: read DOM state, click the router link, read again, log, quit.
async function autocheck(win: BrowserWindow) {
  log('did-finish-load');
  const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));
  const snap = () =>
    win.webContents.executeJavaScript(`({
      hash: location.hash,
      h1: document.querySelector('h1')?.textContent,
      ticks: document.querySelector('[data-test=ticks]')?.textContent,
      versions: document.querySelector('[data-test=versions]')?.textContent,
      doubled: document.querySelector('[data-test=doubled]')?.textContent,
      kobalte: document.querySelector('[data-test=kobalte]')?.outerHTML ?? null,
      h1Color: document.querySelector('h1') && getComputedStyle(document.querySelector('h1')).color,
      h1Size: document.querySelector('h1') && getComputedStyle(document.querySelector('h1')).fontSize,
      nodeInRenderer: typeof require !== 'undefined' || typeof process !== 'undefined',
    })`);
  await wait(2000);
  log('home', JSON.stringify(await snap()));
  await win.webContents.executeJavaScript(`document.querySelector('[data-test=kobalte]').click()`);
  await wait(1000);
  log('after kobalte click', JSON.stringify(await snap()));
  await win.webContents.executeJavaScript(`document.querySelector('[data-test=run-link]').click()`);
  await wait(500);
  log('run', JSON.stringify(await snap()));
  await win.webContents.executeJavaScript(`history.back()`);
  await wait(1500);
  log('back', JSON.stringify(await snap()));
  app.quit();
}

app.whenReady().then(() => {
  log('ready');
  createWindow();

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow();
    }
  });
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});
