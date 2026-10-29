import { defineConfig } from "vite";
import solid from "@solidjs/vite-plugin";
import tailwindcss from "@tailwindcss/vite";
import { define } from "../../../vite.renderer.config.mts";

// Builds the real renderer from ../../../src with the Tauri Api (src/main.tsx). Vite, Solid and
// Tailwind resolve from the repo's node_modules; this folder only adds the Tauri packages.
export default defineConfig({
  plugins: [solid(), tailwindcss()],
  define,
  clearScreen: false,
  server: { port: 5199, strictPort: true, fs: { allow: ["../../.."] } },
  build: { outDir: "dist", emptyOutDir: true },
});
