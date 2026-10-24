import { defineConfig } from "vitest/config";
import solid from "@solidjs/vite-plugin";

// spike tests under docs/ are reference only; e2e/ belongs to Playwright
const exclude = ["docs/**", "node_modules/**", "out/**", ".vite/**", "e2e/**", ".superpowers/**"];

export default defineConfig({
  test: {
    projects: [
      {
        test: { name: "node", include: ["**/*.test.ts"], exclude, environment: "node" },
      },
      {
        plugins: [solid()],
        // jsdom tests need Solid's browser build, not its server build
        resolve: { conditions: ["browser", "development"] },
        test: {
          name: "dom",
          include: ["**/*.test.tsx"],
          exclude,
          environment: "jsdom",
          // externalized deps resolve with Node rules; inline Solid packages so they get the browser build too
          server: { deps: { inline: [/solid/] } },
        },
      },
    ],
  },
});
