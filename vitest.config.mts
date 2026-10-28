import { defineConfig } from "vitest/config";
import solid from "@solidjs/vite-plugin";
import { define } from "./vite.renderer.config.mjs";

// spike tests under docs/ are reference only; e2e/ belongs to Playwright
const exclude = [
  "docs/**",
  "node_modules/**",
  "out/**",
  ".vite/**",
  "e2e/**",
  ".superpowers/**",
  ".claude/**",
  // CI's first unit job skips the files its second job runs (.github/workflows/test.yml)
  ...(process.env.UNIT_EXCLUDE?.split(" ").filter(Boolean) ?? []),
];

export default defineConfig({
  test: {
    projects: [
      {
        test: { name: "node", include: ["**/*.test.ts"], exclude, environment: "node" },
      },
      {
        plugins: [solid()],
        define,
        // jsdom tests need Solid's browser build, not its server build
        resolve: { conditions: ["browser", "development"] },
        test: {
          name: "dom",
          include: ["**/*.test.tsx"],
          exclude,
          environment: "jsdom",
          setupFiles: ["test/jsdom-setup.ts"],
          // externalized deps resolve with Node rules; inline Solid packages so they get the browser build too
          server: { deps: { inline: [/solid/] } },
        },
      },
    ],
  },
});
