import { defineConfig } from "vite";

// https://vitejs.dev/config
export default defineConfig({
  // Electron's fs without asar support; the updater needs it (#131)
  build: { rollupOptions: { external: ["original-fs"] } },
});
