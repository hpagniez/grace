// Bumps one pin in vendor.json to the newest release of the same major (docs/specs/2026-10-27-bundled-dependency-updates.md, R3).
import { createHash } from "node:crypto";
import { appendFileSync } from "node:fs";
import { readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

export const PINS_PATH = join(dirname(fileURLToPath(import.meta.url)), "vendor.json");

const ADOPTIUM =
  "https://api.adoptium.net/v3/assets/latest/21/hotspot?os=windows&architecture=x64&image_type=jre";
const RELEASES = "https://api.github.com/repos/karatelabs/karate/releases?per_page=100";
const JRE_DOWNLOADS = "https://github.com/adoptium/temurin21-binaries/releases/download/";
const SHA256 = /^[0-9a-f]{64}$/;

/** `21.0.12.1+1` or `21.0.13+11` as [major, minor, security, patch, build]; null if not one. */
export function parseJreVersion(text) {
  const m = /^(\d+)\.(\d+)\.(\d+)(?:\.(\d+))?\+(\d+)$/.exec(text);
  return m ? [m[1], m[2], m[3], m[4] ?? "0", m[5]].map(Number) : null;
}

/** `2.1.4` as [major, minor, patch]; null if not one. */
export function parseKarateVersion(text) {
  const m = /^(\d+)\.(\d+)\.(\d+)$/.exec(text);
  return m ? m.slice(1).map(Number) : null;
}

/** Sign of the first differing part: positive when `a` is newer. */
export function compareVersions(a, b) {
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return a[i] - b[i];
  return 0;
}

function parsePinned(dep, pinned, parse) {
  const v = parse(pinned);
  if (!v) throw new Error(`vendor.json ${dep} version "${pinned}" does not parse`);
  return v;
}

/** The JRE pin from Adoptium's latest-release answer, or null when it is not newer. */
export function pickJre(assets, pinned) {
  const old = parsePinned("jre", pinned, parseJreVersion);
  const a = Array.isArray(assets) ? assets[0] : undefined;
  if (!a) throw new Error("Adoptium returned no Java 21 JRE");
  const version = String(a.release_name ?? "").replace(/^jdk-/, "");
  const v = parseJreVersion(version);
  if (!v) throw new Error(`Adoptium release_name "${a.release_name}" is not a Java version`);
  if (v[0] !== old[0] || compareVersions(v, old) <= 0) return null;
  const { checksum, link } = a.binary?.package ?? {};
  if (!SHA256.test(checksum ?? "")) throw new Error(`Adoptium ${version} has no SHA-256 checksum`);
  if (!String(link ?? "").startsWith(JRE_DOWNLOADS))
    throw new Error(`Adoptium ${version} link is not under ${JRE_DOWNLOADS}: ${link}`);
  return { version, url: link, sha256: checksum };
}

/** The Karate pin from GitHub's releases list, or null when nothing newer exists in the same major. */
export function pickKarate(releases, pinned) {
  const old = parsePinned("karate", pinned, parseKarateVersion);
  let best = null;
  for (const r of Array.isArray(releases) ? releases : []) {
    const m = /^v(\d+\.\d+\.\d+)$/.exec(r.tag_name ?? "");
    if (r.draft || r.prerelease || !m) continue;
    const v = parseKarateVersion(m[1]);
    if (v[0] === old[0] && (!best || compareVersions(v, best.v) > 0))
      best = { v, release: r, version: m[1] };
  }
  if (!best || compareVersions(best.v, old) <= 0) return null;
  const { version, release } = best;
  const file = `karate-${version}.jar`;
  const asset = release.assets?.find((x) => x.name === file);
  if (!asset) throw new Error(`Karate v${version} has no ${file} asset`);
  const sha256 = /^sha256:([0-9a-f]{64})$/.exec(asset.digest ?? "")?.[1];
  if (!sha256) throw new Error(`${file} has no sha256 digest`);
  const prefix = `https://github.com/karatelabs/karate/releases/download/v${version}/`;
  if (!String(asset.browser_download_url ?? "").startsWith(prefix))
    throw new Error(`${file} is not under ${prefix}: ${asset.browser_download_url}`);
  return { version, url: asset.browser_download_url, sha256, file };
}

async function get(fetchFn, url, headers = {}) {
  const res = await fetchFn(url, { headers });
  if (!res.ok) throw new Error(`GET ${url} failed: ${res.status} ${res.statusText}`);
  return res;
}

function githubHeaders() {
  const token = process.env.GITHUB_TOKEN;
  return {
    Accept: "application/vnd.github+json",
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
  };
}

/** Rewrites the `dep` pin when upstream has a newer release; null when it is up to date. Writes nothing on any error. */
export async function bump(dep, { pinsPath = PINS_PATH, fetch: fetchFn = fetch } = {}) {
  const pins = JSON.parse(await readFile(pinsPath, "utf8"));
  const from = pins[dep].version;
  if (dep === "jre") {
    const next = pickJre(await (await get(fetchFn, ADOPTIUM)).json(), from);
    if (!next) return null;
    Object.assign(pins.jre, next);
  } else {
    const next = pickKarate(await (await get(fetchFn, RELEASES, githubHeaders())).json(), from);
    if (!next) return null;
    const url = `https://raw.githubusercontent.com/karatelabs/karate/v${next.version}/LICENSE`;
    const body = Buffer.from(await (await get(fetchFn, url)).arrayBuffer());
    Object.assign(pins.karate, next);
    Object.assign(pins.license, { url, sha256: createHash("sha256").update(body).digest("hex") });
  }
  await writeFile(pinsPath, JSON.stringify(pins, null, 2) + "\n");
  return { from, to: pins[dep].version };
}

async function main() {
  const dep = process.argv[2];
  if (dep !== "jre" && dep !== "karate") {
    console.error("usage: node scripts/bump-vendor.mjs <jre|karate>");
    process.exit(2);
  }
  const from = JSON.parse(await readFile(PINS_PATH, "utf8"))[dep].version;
  const r = await bump(dep);
  console.log(r ? `${dep}: ${r.from} -> ${r.to}` : `${dep}: ${from} is the latest`);
  if (process.env.GITHUB_OUTPUT)
    appendFileSync(
      process.env.GITHUB_OUTPUT,
      `changed=${Boolean(r)}\nfrom=${from}\nto=${r ? r.to : from}\n`,
    );
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((e) => {
    console.error(e.message);
    process.exit(1);
  });
}
