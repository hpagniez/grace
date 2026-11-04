// Shared by main and renderer: keep this file free of Node imports.
// The OpenAPI import's contract between the window and main (docs/specs/2026-11-04-openapi-import.md).

const BAD_URL = "Enter an http or https URL";

/** The message for a URL Grace will not fetch (R2.2), or null when it is fine. */
export function urlError(url: string): string | null {
  if (url.length > 2000 || !URL.canParse(url)) return BAD_URL;
  const { protocol, username, password } = new URL(url);
  if (protocol !== "http:" && protocol !== "https:") return BAD_URL;
  return username || password ? "Remove the user name and password from the URL" : null;
}

/** What the dialog shows before anything is written (R7). */
export interface OpenApiPreview {
  title: string;
  version: string;
  operations: number;
  features: number;
  /** Relative to the project with `/`; `""` for the project folder. */
  folder: string;
  files: { name: string; exists: boolean }[];
}

export type OpenApiRead = { preview: OpenApiPreview } | { errors: string[] };
