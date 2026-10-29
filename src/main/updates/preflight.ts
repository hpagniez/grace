import { existsSync, readdirSync } from "node:fs";
import { mkdir, readdir, readFile, rmdir, statfs } from "node:fs/promises";
import path from "node:path";

/** Written into `resources` by the Forge hook: the packaged folder's top-level names. */
export const APP_FILES = "app-files.json";

/** A folder holding a Grace install: the build's file list is inside. */
export const isGraceFolder = (dir: string): boolean =>
  existsSync(path.join(dir, "resources", APP_FILES));

/** What Grace puts in its staging folder `<install>.update` (R8.2, R8.5, R10.6, R10.8, R10.9). */
const STAGING =
  /^(plan\.json|switch\.ps1|switching|started|app|old|failed|grace-win32-x64-.+\.zip)$/i;

/**
 * True when `dir` is missing or holds only what Grace puts in `<install>.update`, so Grace may
 * delete it: never a folder of someone else's that happens to have that name.
 */
export function ownStaging(dir: string): boolean {
  try {
    return readdirSync(dir).every((n) => STAGING.test(n));
  } catch (e) {
    return (e as NodeJS.ErrnoException).code === "ENOENT";
  }
}

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

  // an update removes these: never one of someone else's (R8.2, R10.3, R10.6)
  const leftover = [`${installDir}.old`, `${installDir}.failed`].find(
    (d) => existsSync(d) && !isGraceFolder(d),
  );
  const staging = `${installDir}.update`;
  const inTheWay = leftover ?? (ownStaging(staging) ? null : staging);
  if (inTheWay) {
    return `Grace can't update itself while ${inTheWay} exists: it is not a folder Grace made. Move or rename it, then try again.`;
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
