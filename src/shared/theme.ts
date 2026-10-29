// Shared by main and renderer: keep this file free of Node imports.
// The Appearance setting (docs/specs/2026-10-29-premium-ui.md R1.8, R8).

/** The `ground` colour of src/index.css in each theme: main's window background and index.html's startup screen. */
export const GROUND = { light: "#f3efe6", dark: "#151821" } as const;

/** Follow Windows, or force one theme. */
export type Theme = "system" | "light" | "dark";
export const THEMES: readonly Theme[] = ["system", "light", "dark"];

/** `x` when it is a theme, else `"system"`. */
export const parseTheme = (x: unknown): Theme => THEMES.find((t) => t === x) ?? "system";
