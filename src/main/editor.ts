import { realpathSync, statSync } from "node:fs";
import path from "node:path";

/** True for `dir` and anything under it, by the names alone; a folder named `..data` is under it. */
const under = (dir: string, abs: string) => {
  const rel = path.relative(dir, abs);
  return rel !== ".." && !rel.startsWith(`..${path.sep}`) && !path.isAbsolute(rel);
};

/**
 * `file`'s real path if it is an existing file inside `root`'s real folder, links followed
 * (docs/specs/2026-11-03-open-in-editor.md, R3.3); null otherwise.
 */
export function projectFile(root: string, file: string): string | null {
  try {
    const realRoot = realpathSync.native(root);
    const abs = path.resolve(file);
    // Checked by name before any read: reading a share or a device path would reach it.
    // Karate may write the project's path as given or as its real path (8.3 names expanded).
    if (!under(root, abs) && !under(realRoot, abs)) return null;
    const real = realpathSync.native(abs);
    return under(realRoot, real) && statSync(real).isFile() ? real : null;
  } catch {
    return null;
  }
}

/** VS Code's link to `file` at `line` (R4.2); encodeURI leaves `#` and `?`, which would end the path. */
export const vscodeUrl = (file: string, line: number) =>
  `vscode://file/${encodeURI(file.replaceAll("\\", "/")).replaceAll("#", "%23").replaceAll("?", "%3F")}:${line}`;
