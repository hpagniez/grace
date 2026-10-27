import { defineConfig } from "vite";
import solid from "@solidjs/vite-plugin";
import tailwindcss from "@tailwindcss/vite";
import pkg from "./package.json" with { type: "json" };
import { VERSIONS } from "./scripts/fetch-vendor.mjs";

// Shown by the console banner; vitest.config.mts gives the jsdom tests the same values.
export const define = { __KK_VERSIONS__: JSON.stringify({ app: pkg.version, ...VERSIONS }) };

// https://vitejs.dev/config
export default defineConfig({
  plugins: [solid(), tailwindcss()],
  define,
});
