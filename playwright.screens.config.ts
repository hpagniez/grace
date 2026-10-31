import { defineConfig } from "@playwright/test";

// Not part of `npm run e2e`: the screenshots of docs/checks (`npm run screens`) and of the README
// (`npm run screens:readme`).
export default defineConfig({
  testDir: "e2e",
  testMatch: "*.shots.ts",
  timeout: 120_000,
  workers: 1,
  reporter: "list",
});
