import { defineConfig } from "@playwright/test";

// The project libraries check with a real Maven and Gradle (README.md). Never part of `npm run e2e`.
export default defineConfig({
  testDir: ".",
  timeout: 600_000,
  workers: 1,
  reporter: "list",
});
