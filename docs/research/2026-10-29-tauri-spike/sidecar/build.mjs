// Bundles sidecar/main.ts into one CommonJS file, then makes it a Node single executable application
// (node.exe + the bundle) named the way Tauri's `externalBin` expects.
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "rolldown";

const here = path.dirname(fileURLToPath(import.meta.url));
const out = path.join(here, "out");
mkdirSync(out, { recursive: true });

await build({
  input: path.join(here, "main.ts"),
  platform: "node",
  external: ["electron"], // only src/main/secrets.ts's safeStorageBox requires it, never called here
  output: { file: path.join(out, "sidecar.cjs"), format: "cjs" },
});

const blob = path.join(out, "sea-prep.blob");
writeFileSync(
  path.join(out, "sea-config.json"),
  JSON.stringify({ main: path.join(out, "sidecar.cjs"), output: blob, disableExperimentalSEAWarning: true }),
);
execFileSync(process.execPath, ["--experimental-sea-config", path.join(out, "sea-config.json")], {
  stdio: "inherit",
});

const bin = path.join(here, "..", "src-tauri", "binaries");
mkdirSync(bin, { recursive: true });
const exe = path.join(bin, "grace-sidecar-x86_64-pc-windows-msvc.exe");
copyFileSync(process.execPath, exe);
execFileSync(
  process.execPath,
  [
    path.join(here, "..", "..", "..", "..", "node_modules", "postject", "dist", "cli.js"),
    exe,
    "NODE_SEA_BLOB",
    blob,
    "--sentinel-fuse",
    "NODE_SEA_FUSE_fce680ab2cc467b6e072b8b5df1996b2",
  ],
  { stdio: "inherit" },
);
console.log(`sidecar ready: ${exe}`);
