import { defineConfig } from "@playwright/test";

// Not part of `npm run e2e`: it takes the before/after screenshots of docs/checks (`npm run screens`).
export default defineConfig({
  testDir: "e2e",
  testMatch: "screens.shots.ts",
  timeout: 120_000,
  workers: 1,
  reporter: "list",
});
