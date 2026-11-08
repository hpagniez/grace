// `npm run package:gpui`: builds GPUI Grace (ADR 0031) in release and lays it out as the zip of ADR 0019, the layout
// Electron Grace's zip has: `Grace.exe` at the root, `resources\app-files.json` (the folder's top-level names, the
// updater's mark of a Grace folder), and the JRE, Karate jar, debug listener and licences in `resources\`. Windows only,
// after `npm run vendor`. Writes out/gpui/Grace-win32-x64/ and out/gpui/make/Grace-win32-x64-<version>.zip. Nothing
// releases this zip until phase P5 (migration guide, Phases).
//
//   node scripts/package-gpui.mjs [--no-build] [--no-zip]
import { spawnSync } from "node:child_process";
import {
  copyFileSync,
  cpSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");

/** The updater's list of the folder's top-level names (src/shared/updates.ts `APP_FILES`). */
export const APP_FILES = "app-files.json";

/** What Grace embeds besides the JRE and Karate, with the licence each ships under: forge.config.mts `NPM_LICENSES`. */
export const LICENSES = {
  "lucide-LICENSE.txt": "node_modules/lucide/LICENSE",
  "ibm-plex-sans-LICENSE.txt": "node_modules/@fontsource-variable/ibm-plex-sans/LICENSE",
  "ibm-plex-mono-LICENSE.txt": "node_modules/@fontsource/ibm-plex-mono/LICENSE",
};

/** The zip's name (src/shared/updates.ts `zipName`): part of the updater's contract. */
export const zipName = (/** @type {string} */ version) => `Grace-win32-x64-${version}.zip`;

/**
 * Lays out the install folder `out`, emptied first: `exe` as `Grace.exe`, and in `resources` the JRE, `karate.jar`,
 * `listener.jar` and `licenses` of `vendor`, the files of `licenses` (name to path) beside them, and `app-files.json`.
 * @param {{ exe: string, vendor: string, licenses: Record<string, string>, out: string }} opts
 */
export function layout({ exe, vendor, licenses, out }) {
  for (const need of [
    exe,
    ...["jre", "karate.jar", "listener.jar", "licenses"].map((f) => join(vendor, f)),
  ])
    if (!existsSync(need))
      throw new Error(`${need} is missing${need.startsWith(vendor) ? ": run npm run vendor" : ""}`);
  rmSync(out, { recursive: true, force: true });
  const resources = join(out, "resources");
  mkdirSync(resources, { recursive: true });
  copyFileSync(exe, join(out, "Grace.exe"));
  cpSync(join(vendor, "jre"), join(resources, "jre"), { recursive: true });
  for (const jar of ["karate.jar", "listener.jar"])
    copyFileSync(join(vendor, jar), join(resources, jar));
  cpSync(join(vendor, "licenses"), join(resources, "licenses"), { recursive: true });
  for (const [name, from] of Object.entries(licenses))
    copyFileSync(from, join(resources, "licenses", name));
  writeFileSync(join(resources, APP_FILES), JSON.stringify(readdirSync(out)));
}

function run(
  /** @type {string} */ cmd,
  /** @type {string[]} */ args,
  /** @type {string} */ cwd = root,
) {
  const r = spawnSync(cmd, args, { cwd, stdio: "inherit" });
  if (r.status !== 0)
    throw new Error(`${cmd} ${args.join(" ")} failed (${r.error?.message ?? r.status})`);
}

function main() {
  if (process.platform !== "win32") throw new Error("the zip is Windows': run this on Windows");
  const args = new Set(process.argv.slice(2));
  if (!args.has("--no-build")) run("cargo", ["build", "-p", "grace", "--release", "--locked"]);
  const out = join(root, "out", "gpui", "Grace-win32-x64");
  layout({
    exe: join(root, "target", "release", "Grace.exe"),
    vendor: join(root, "vendor"),
    licenses: Object.fromEntries(
      Object.entries(LICENSES).map(([n, from]) => [n, join(root, from)]),
    ),
    out,
  });
  console.log(`laid out ${out}`);
  if (args.has("--no-zip")) return;
  const { version } = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
  const make = join(root, "out", "gpui", "make");
  mkdirSync(make, { recursive: true });
  const zip = join(make, zipName(version));
  rmSync(zip, { force: true });
  // System32 bsdtar, as fetch-vendor.mjs: it writes a zip with -a, and the folder's own names go in at the zip's root
  const tar = join(process.env.SystemRoot ?? "C:\\Windows", "System32", "tar.exe");
  run(tar, ["-a", "-c", "-f", zip, ...readdirSync(out)], out);
  console.log(`wrote ${zip}`);
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  try {
    main();
  } catch (e) {
    console.error(/** @type {Error} */ (e).message);
    process.exit(1);
  }
}
