// The floor of Electron itself: one 1400 x 900 window with one line of text, nothing else.
const { app, BrowserWindow } = require("electron");
app.whenReady().then(() => {
  const w = new BrowserWindow({ x: 0, y: 0, width: 1400, height: 900, webPreferences: { sandbox: true } });
  w.loadURL("data:text/html,<body style='font-family:sans-serif'>Grace</body>");
});
