import { defineConfig } from "@playwright/test";

export default defineConfig({ testDir: ".", testMatch: "*.spec.mjs", timeout: 90_000, workers: 1, reporter: "list" });
