/// <reference types="@electron-forge/plugin-vite/forge-vite-env" />
declare module "*.css";

/** The app's and the bundled Karate's and Java's versions, set at build time by vite.renderer.config.mts. */
declare const __KK_VERSIONS__: { app: string; karate: string; java: string };

/** Exposed by src/preload.ts through contextBridge. */
interface Window {
  api: import("./shared/ipc").Api;
}
