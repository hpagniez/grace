import { mkdir, readdir, readFile, rmdir, statfs } from "node:fs/promises";
import path from "node:path";

/** Written into `resources` by the Forge hook: the packaged folder's top-level names. */
export const APP_FILES = "app-files.json";

/** Windows adds these to any folder it shows. */
const IGNORED = new Set(["desktop.ini", "thumbs.db"]);

async function freeSpace(dir: string): Promise<number> {
  const s = await statfs(dir);
  return s.bavail * s.bsize;
}

/** The lower-cased names in the folder's list; none when the list is missing or unreadable. */
async function listed(installDir: string): Promise<Set<string>> {
  try {
    const names: unknown = JSON.parse(
      await readFile(path.join(installDir, "resources", APP_FILES), "utf8"),
    );
    return new Set(Array.isArray(names) ? names.map((n) => String(n).toLowerCase()) : []);
  } catch {
    return new Set();
  }
}

/** R8.1: null when the install folder can be updated, else the message to show. */
export async function preflight(
  installDir: string,
  zipSize: number,
  freeBytes: (dir: string) => Promise<number> = freeSpace,
): Promise<string | null> {
  const known = await listed(installDir);
  const other = (await readdir(installDir))
    .filter((n) => !known.has(n.toLowerCase()) && !IGNORED.has(n.toLowerCase()))
    .sort();
  if (other.length) {
    return `Grace's folder also holds other files (${other.slice(0, 3).join(", ")}). Put Grace's files in a folder of their own to update it.`;
  }

  const parent = path.dirname(installDir);
  const probe = path.join(parent, `.grace-write-test-${process.pid}`);
  try {
    await mkdir(probe);
    await rmdir(probe);
  } catch {
    return `Grace can't update itself in ${parent}. Move the Grace folder somewhere you can write to, such as your user folder, or download the new version from the release page.`;
  }

  const need = 4 * zipSize;
  if ((await freeBytes(installDir)) < need) {
    const drive = path.parse(path.resolve(installDir)).root.replace(/[\\/:]+$/, "");
    return `Not enough free space on ${drive}: Grace needs ${Math.ceil(need / 1e6)} MB to update.`;
  }
  return null;
}
