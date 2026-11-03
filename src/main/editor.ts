import { realpathSync, statSync } from "node:fs";
import path from "node:path";

/**
 * `file`'s real path if it is an existing file inside `root`'s real folder, links followed
 * (docs/specs/2026-11-03-open-in-editor.md, R3.3); null otherwise. A folder named `..data` is inside.
 */
export function projectFile(root: string, file: string): string | null {
  try {
    const real = realpathSync.native(file);
    const rel = path.relative(realpathSync.native(root), real);
    const inside = rel !== ".." && !rel.startsWith(`..${path.sep}`) && !path.isAbsolute(rel);
    return inside && statSync(real).isFile() ? real : null;
  } catch {
    return null;
  }
}

/** VS Code's link to `file` at `line` (R4.2); encodeURI leaves `#` and `?`, which would end the path. */
export const vscodeUrl = (file: string, line: number) =>
  `vscode://file/${encodeURI(file.replaceAll("\\", "/")).replaceAll("#", "%23").replaceAll("?", "%3F")}:${line}`;
