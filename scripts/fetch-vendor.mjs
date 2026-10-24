// Downloads, verifies and unpacks the bundled JRE and Karate jar into vendor/ (git-ignored).
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { createReadStream, existsSync, readdirSync } from "node:fs";
import { copyFile, mkdir, readdir, rename, rm, stat, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const vendor = join(root, "vendor");
const download = join(vendor, ".download");

const JRE = {
  url: "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.12.1%2B1/OpenJDK21U-jre_x64_windows_hotspot_21.0.12.1_1.zip",
  sha256: "d35f31e712f0fcf6ac5a093edc90204fbff22f720ba3950bd09d331d5e621636",
  file: "jre.zip",
};
const JAR = {
  url: "https://github.com/karatelabs/karate/releases/download/v2.1.4/karate-2.1.4.jar",
  sha256: "1f743f4acc56526b39bd88c84bfe4491c46d5e94f4ab5a38c437c9400dbc2488",
  file: "karate-2.1.4.jar",
};
const LICENSE = {
  url: "https://raw.githubusercontent.com/karatelabs/karate/v2.1.4/LICENSE",
  sha256: "28890ebdc037407a0e1168ce3b01880d5fbfbc6381f37629d25c070482ea0cea",
  file: "karate-LICENSE.txt",
};

export async function verifySha256(file, expected) {
  const hash = createHash("sha256");
  for await (const chunk of createReadStream(file)) hash.update(chunk);
  const actual = hash.digest("hex");
  if (actual !== expected) {
    throw new Error(`SHA-256 mismatch for ${file}: expected ${expected}, got ${actual}`);
  }
}

// Downloads to vendor/.download/ unless a verified copy is already there; returns the path.
async function fetchVerified({ url, sha256, file }) {
  const dest = join(download, file);
  if (existsSync(dest)) {
    try {
      await verifySha256(dest, sha256);
      return dest;
    } catch {
      await rm(dest);
    }
  }
  console.log(`downloading ${url}`);
  const res = await fetch(url);
  if (!res.ok) throw new Error(`GET ${url} failed: ${res.status} ${res.statusText}`);
  await writeFile(dest, Buffer.from(await res.arrayBuffer()));
  await verifySha256(dest, sha256);
  return dest;
}

async function setupJre() {
  const jreDir = join(vendor, "jre");
  if (existsSync(join(jreDir, "bin", "java.exe"))) return;
  const zip = await fetchVerified(JRE);
  const tmp = join(download, "jre-unpacked");
  await rm(tmp, { recursive: true, force: true });
  await rm(jreDir, { recursive: true, force: true });
  await mkdir(tmp, { recursive: true });
  // System32 bsdtar explicitly: Git Bash puts GNU tar first on PATH and it misreads "D:\..." paths
  const tar = join(process.env.SystemRoot ?? "C:\\Windows", "System32", "tar.exe");
  const r = spawnSync(tar, ["-xf", zip, "-C", tmp], { stdio: "inherit" });
  if (r.status !== 0) throw new Error("tar -xf failed for the JRE zip");
  const tops = readdirSync(tmp);
  if (tops.length !== 1) throw new Error(`expected one top folder in the JRE zip, got ${tops}`);
  await rename(join(tmp, tops[0]), jreDir);
  await rm(tmp, { recursive: true });
}

async function setupLicenses() {
  const out = join(vendor, "licenses");
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

async function main() {
  await mkdir(download, { recursive: true });
  await setupJre();
  const jar = join(vendor, "karate.jar");
  if (!existsSync(jar)) await copyFile(await fetchVerified(JAR), jar);
  await verifySha256(jar, JAR.sha256);
  await setupLicenses();
  console.log("vendor ready");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((e) => {
    console.error(e);
    process.exit(1);
  });
}
