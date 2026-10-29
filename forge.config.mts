import type { ForgeConfig } from "@electron-forge/shared-types";
import { MakerZIP } from "@electron-forge/maker-zip";
import { VitePlugin } from "@electron-forge/plugin-vite";
import { FusesPlugin } from "@electron-forge/plugin-fuses";
import { FuseV1Options, FuseVersion } from "@electron/fuses";
import { readdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { APP_FILES } from "./src/main/updates/preflight";

const config: ForgeConfig = {
  packagerConfig: {
    asar: true,
    icon: "assets/icon", // .ico on Windows, embedded in Grace.exe
    extraResource: ["vendor/jre", "vendor/karate.jar", "vendor/licenses"],
  },
  rebuildConfig: {},
  hooks: {
    // The updater (R8.1) refuses a folder that holds anything not listed here.
    postPackage: async (_config, { outputPaths }) => {
      for (const out of outputPaths) {
        writeFileSync(path.join(out, "resources", APP_FILES), JSON.stringify(readdirSync(out)));
      }
    },
  },
  makers: [new MakerZIP({}, ["win32"])],
  plugins: [
    new VitePlugin({
      // `build` can specify multiple entry builds, which can be Main process, Preload scripts, Worker process, etc.
      // If you are familiar with Vite configuration, it will look really familiar.
      build: [
        {
          // `entry` is just an alias for `build.lib.entry` in the corresponding file of `config`.
          entry: "src/main.ts",
          config: "vite.main.config.mts",
          target: "main",
        },
        {
          entry: "src/preload.ts",
          config: "vite.preload.config.mts",
          target: "preload",
        },
      ],
      renderer: [
        {
          name: "main_window",
          config: "vite.renderer.config.mts",
        },
      ],
    }),
    // Fuses are used to enable/disable various Electron functionality
    // at package time, before code signing the application
    new FusesPlugin({
      version: FuseVersion.V1,
      [FuseV1Options.RunAsNode]: false,
      [FuseV1Options.EnableCookieEncryption]: true,
      [FuseV1Options.EnableNodeOptionsEnvironmentVariable]: false,
      [FuseV1Options.EnableNodeCliInspectArguments]: false,
      [FuseV1Options.EnableEmbeddedAsarIntegrityValidation]: true,
      [FuseV1Options.OnlyLoadAppFromAsar]: true,
    }),
  ],
};

export default config;
