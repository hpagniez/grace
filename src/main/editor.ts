import { realpathSync, statSync } from "node:fs";
import path from "node:path";
import { inside } from "./karate/command";

/**
 * `file` under `root` as the project was opened, if it is an existing file whose real path, links
 * followed, is inside `root`'s real folder (docs/specs/2026-11-03-open-in-editor.md, R3.3); null otherwise.
 * A mapped drive keeps its letter: its real path is the share's.
 */
export function projectFile(root: string, file: string): string | null {
  try {
    const realRoot = realpathSync.native(root);
    const abs = path.resolve(file);
    // Checked by name before any read: reading a share or a device path would reach it.
    // Karate may write the project's path as given or as its real path (8.3 names expanded).
    if (!inside(root, abs) && !inside(realRoot, abs)) return null;
    const real = realpathSync.native(abs);
    return inside(realRoot, real) && statSync(real).isFile()
      ? path.join(root, path.relative(realRoot, real))
      : null;
  } catch {
    return null;
  }
}

/**
 * VS Code's link to `file` at `line` (R4.2); encodeURI leaves `#` and `?`, which would end the path.
 * A share's `//server` follows `file` directly: VS Code reads `file///server` as `\server`.
 */
export const vscodeUrl = (file: string, line: number) => {
  const p = encodeURI(file.replaceAll("\\", "/")).replaceAll("#", "%23").replaceAll("?", "%3F");
  return `vscode://file${p.startsWith("/") ? "" : "/"}${p}:${line}`;
};
