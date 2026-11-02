// Downloads, verifies and unpacks the bundled JRE and Karate jar into vendor/ (git-ignored), and compiles the
// debug listener into vendor/listener.jar with ECJ on that JRE (ADR 0025).
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { createReadStream, existsSync, readFileSync, readdirSync } from "node:fs";
import { copyFile, mkdir, readdir, rename, rm, stat, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const vendor = join(root, "vendor");
const download = join(vendor, ".download");

// Pins live in vendor.json so scripts/bump-vendor.mjs can rewrite them (ADR 0012).
const {
  jre: JRE,
  karate: JAR,
  license: LICENSE,
  ecj: ECJ,
} = JSON.parse(readFileSync(join(root, "scripts", "vendor.json"), "utf8"));
// System32 bsdtar explicitly: Git Bash puts GNU tar first on PATH and it misreads "D:\..." paths
const tar = join(process.env.SystemRoot ?? "C:\\Windows", "System32", "tar.exe");
// The SHA-256 of the zip vendor/jre was unpacked from, so a changed pin replaces it.
const jreMarker = join(vendor, ".jre-sha256");

export async function verifySha256(file, expected) {
  const hash = createHash("sha256");
  for await (const chunk of createReadStream(file)) hash.update(chunk);
  const actual = hash.digest("hex");
  if (actual !== expected) {
    throw new Error(`SHA-256 mismatch for ${file}: expected ${expected}, got ${actual}`);
  }
}

async function hasSha256(file, sha256) {
  if (!existsSync(file)) return false;
  try {
    await verifySha256(file, sha256);
    return true;
  } catch {
    return false;
  }
}

// Downloads to vendor/.download/ unless a verified copy is already there; returns the path.
async function fetchVerified({ url, sha256, file }) {
  const dest = join(download, file);
  if (await hasSha256(dest, sha256)) return dest;
  await rm(dest, { force: true });
  console.log(`downloading ${url}`);
  const res = await fetch(url);
  if (!res.ok) throw new Error(`GET ${url} failed: ${res.status} ${res.statusText}`);
  await writeFile(dest, Buffer.from(await res.arrayBuffer()));
  await verifySha256(dest, sha256);
  return dest;
}

async function setupJre() {
  const jreDir = join(vendor, "jre");
  const unpacked =
    existsSync(join(jreDir, "bin", "java.exe")) &&
    existsSync(jreMarker) &&
    readFileSync(jreMarker, "utf8") === JRE.sha256;
  if (unpacked) return;
  const zip = await fetchVerified(JRE);
  const tmp = join(download, "jre-unpacked");
  await rm(tmp, { recursive: true, force: true });
  await rm(jreDir, { recursive: true, force: true });
  await mkdir(tmp, { recursive: true });
  const r = spawnSync(tar, ["-xf", zip, "-C", tmp], { stdio: "inherit" });
  if (r.status !== 0) throw new Error("tar -xf failed for the JRE zip");
  const tops = readdirSync(tmp);
  if (tops.length !== 1) throw new Error(`expected one top folder in the JRE zip, got ${tops}`);
  await rename(join(tmp, tops[0]), jreDir);
  await rm(tmp, { recursive: true });
  await writeFile(jreMarker, JRE.sha256);
}

async function setupLicenses() {
  const out = join(vendor, "licenses");
  await rm(out, { recursive: true, force: true });
  const temurin = join(out, "temurin");
  await mkdir(temurin, { recursive: true });
  const jreDir = join(vendor, "jre");
  for (const name of await readdir(jreDir)) {
    const p = join(jreDir, name);
    if (name === "legal") await copyDir(p, join(temurin, "legal"));
    else if (/^(NOTICE|LICENSE)/i.test(name) && (await stat(p)).isFile()) {
      await copyFile(p, join(temurin, name));
    }
  }
  await copyFile(await fetchVerified(LICENSE), join(out, "karate-LICENSE.txt"));
}

async function copyDir(from, to) {
  await mkdir(to, { recursive: true });
  for (const e of await readdir(from, { withFileTypes: true })) {
    const src = join(from, e.name);
    if (e.isDirectory()) await copyDir(src, join(to, e.name));
    else await copyFile(src, join(to, e.name));
  }
}

/** The zip of the listener's classes (`-a` would not take `.jar` for zip); a jar on the classpath needs no manifest. */
const zipArgs =(jar, classes) => ["--format", "zip", "-c", "-f", jar, "-C", classes, "grace"];

// Recompiled on every run, under a second: a changed source or a new Karate jar is always picked up.
async function compileListener(jar) {
  const ecj = join(vendor, "ecj.jar"); // beside karate.jar, so the CI vendor/ cache keeps it
  if (!(await hasSha256(ecj, ECJ.sha256))) await copyFile(await fetchVerified(ECJ), ecj);
  const classes = join(download, "listener-classes");
  await rm(classes, { recursive: true, force: true });
  const java = join(vendor, "jre", "bin", "java.exe");
  const source = join(root, "listener", "grace", "DebugListener.java");
  const c = spawnSync(java, ["-jar", ecj, "-21", "-nowarn", "-cp", jar, "-d", classes, source], {
    stdio: "inherit",
  });
  if (c.status !== 0) throw new Error("the debug listener does not compile");
  const out = join(vendor, "listener.jar");
  await rm(out, { force: true });
  const z = spawnSync(tar, zipArgs(out, classes), { stdio: "inherit" });
  if (z.status !== 0) throw new Error("tar could not zip the debug listener");
}

async function main() {
  await mkdir(download, { recursive: true });
  await setupJre();
  const jar = join(vendor, "karate.jar");
  if (!(await hasSha256(jar, JAR.sha256))) await copyFile(await fetchVerified(JAR), jar);
  await verifySha256(jar, JAR.sha256);
  await compileListener(jar);
  await setupLicenses();
  console.log("vendor ready");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((e) => {
    console.error(e);
    process.exit(1);
  });
}
