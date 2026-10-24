/// <reference types="@electron-forge/plugin-vite/forge-vite-env" />
declare module "*.css";

/** Exposed by src/preload.ts through contextBridge. */
interface Window {
  api: import("./shared/ipc").Api;
}
