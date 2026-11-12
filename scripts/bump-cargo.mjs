// Moves GPUI Grace to the newest GPUI set, Rust toolchain and Cargo.lock (ADR 0032): the pins in Cargo.toml and
// rust-toolchain.toml, the patched gpui-pre-windows rebuilt from the new release with every patch of patches/series
// (patches/README.md), then `cargo update`. Prints what moved as Markdown. With --pr (cargo-updates.yml), it also
// commits on a new branch, opens the PR and starts checks, test and perf on it. With --merge, it waits for those runs
// and merges the PR when all three pass (ADR 0033). When a patch no longer applies, the GPUI set stays, an issue says
// why, and the toolchain and the lock still move.
//
//   node scripts/bump-cargo.mjs [--pr]
//   node scripts/bump-cargo.mjs --merge <pr> <sha>
import { execFileSync, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  appendFileSync,
  cpSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { BOT_EMAIL } from "./bump-pr.mjs";
import { compareVersions } from "./bump-vendor.mjs";

export const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");

const INDEX = "https://index.crates.io/";
const CRATES = "https://static.crates.io/crates/";
const RUST_STABLE = "https://static.rust-lang.org/dist/channel-rust-stable.toml";

/** GPUI's crates and GPUI Kit's, as Grace pins them: each set moves as one (ADR 0031). */
export const GPUI = ["gpui-pre", "gpui-pre-platform"];
export const KIT = ["gpui-component", "gpui-base", "gpui-kit-assets"];
/** The crate patches/ carries a patched copy of. */
export const PATCHED = "gpui-pre-windows";
/** What crates.io's .crate holds that the copy leaves out (patches/README.md). */
const DROPPED = [".cargo_vcs_info.json", ".cargo-ok", "Cargo.lock", "Cargo.toml.orig"];
/** The workspace's manifests, where Grace names its own dependencies. */
const MANIFESTS = [
  "Cargo.toml",
  "crates/grace/Cargo.toml",
  "crates/grace-core/Cargo.toml",
  "crates/grace-e2e/Cargo.toml",
];

/** Where each pin of the two sets sits in the workspace's Cargo.toml. */
const PINS = [
  ["gpui", /^(gpui = \{ package = "gpui-pre", version = "=)([^"]+)/m],
  ["gpui", /^(gpui_platform = \{ package = "gpui-pre-platform", version = "=)([^"]+)/m],
  ["kit", /^(gpui-component = "=)([^"]+)/m],
  ["kit", /^(gpui-base = "=)([^"]+)/m],
  ["kit", /^(gpui-kit-assets = "=)([^"]+)/m],
];

/** The issue every bump PR is part of: keeping the set current (ADR 0032). */
const ISSUE = 328;
/** The runs a bump PR merges on, started on its commit by hand: a PR opened with the workflow's token starts none. */
export const GATES = ["checks.yml", "test.yml", "perf.yml"];

export class PatchError extends Error {}

/** Runs a command without a shell and returns stdout; throws on a non-zero exit. */
const execRun = (cmd, args, input, cwd) =>
  execFileSync(cmd, args, { encoding: "utf8", input, cwd, maxBuffer: 64 << 20 });

/** A crate's file in crates.io's sparse index. */
export function indexPath(name) {
  const n = name.toLowerCase();
  if (n.length <= 2) return `${n.length}/${n}`;
  if (n.length === 3) return `3/${n[0]}/${n}`;
  return `${n.slice(0, 2)}/${n.slice(2, 4)}/${n}`;
}

/** `1.2.3` as numbers; null for a pre-release or anything else. */
export function parseVersion(text) {
  const m = /^(\d+)\.(\d+)\.(\d+)$/.exec(text ?? "");
  return m ? m.slice(1).map(Number) : null;
}

/** True when `a` is a later release than `b`. */
export const isNewer = (a, b) => compareVersions(parseVersion(a), parseVersion(b)) > 0;

/** A sparse index file as its entries, one per published version. */
export const parseIndex = (text) =>
  text
    .split("\n")
    .filter((line) => line.trim())
    .map((line) => JSON.parse(line));

const release = (entries, vers) => entries.find((e) => e.vers === vers && !e.yanked);

/** The newest release that is neither yanked nor a pre-release. */
export function newest(entries) {
  let best = null;
  for (const e of entries)
    if (!e.yanked && parseVersion(e.vers) && (!best || isNewer(e.vers, best.vers))) best = e;
  if (!best) throw new Error(`no release of ${entries[0]?.name ?? "a crate"} on crates.io`);
  return best;
}

/**
 * The GPUI set to move to: the newest GPUI Kit and the gpui-pre it pins exactly. `waiting` is a newer gpui-pre that no
 * GPUI Kit takes yet. `sha256` is the checksum crates.io gives for the patched crate's .crate.
 * @param {Record<string, any[]>} index the entries of every crate of GPUI, KIT and PATCHED
 */
export function pickSet(index) {
  const kit = newest(index[KIT[0]]);
  for (const name of KIT.slice(1))
    if (!release(index[name], kit.vers))
      throw new Error(`GPUI Kit ${kit.vers}: ${name} ${kit.vers} is not on crates.io`);
  const reqs = [
    ...new Set(
      kit.deps
        .filter((d) => (d.package ?? d.name) === GPUI[0] && (d.kind ?? "normal") === "normal")
        .map((d) => d.req),
    ),
  ];
  const gpui = reqs.length === 1 ? /^=\s*(\d+\.\d+\.\d+)$/.exec(reqs[0])?.[1] : undefined;
  if (!gpui)
    throw new Error(`GPUI Kit ${kit.vers} does not pin one gpui-pre exactly: ${reqs.join(", ")}`);
  for (const name of [...GPUI, PATCHED])
    if (!release(index[name], gpui))
      throw new Error(`gpui-pre ${gpui}: ${name} ${gpui} is not on crates.io`);
  const latest = newest(index[GPUI[0]]).vers;
  return {
    gpui,
    kit: kit.vers,
    sha256: release(index[PATCHED], gpui).cksum,
    waiting: isNewer(latest, gpui) ? latest : null,
  };
}

/** Rust's newest stable release, from its channel manifest. */
export function parseRustStable(manifest) {
  const v = /^\[pkg\.rustc\]\r?\nversion = "(\d+\.\d+\.\d+) /m.exec(manifest)?.[1];
  if (!v) throw new Error("Rust's stable channel names no rustc version");
  return v;
}

/** The GPUI set, GPUI Kit and toolchain as pinned. Throws when a set is pinned at two versions. */
export function readPins(cargoToml, toolchain) {
  /** @type {Record<string, string>} */
  const pins = {};
  for (const [set, re] of PINS) {
    const v = re.exec(cargoToml)?.[2];
    if (!v) throw new Error(`Cargo.toml has no pin matching ${re.source}`);
    if (pins[set] && pins[set] !== v)
      throw new Error(`Cargo.toml pins the ${set} set at both ${pins[set]} and ${v}`);
    pins[set] = v;
  }
  const rust = /^channel = "([^"]+)"/m.exec(toolchain)?.[1];
  if (!rust) throw new Error("rust-toolchain.toml names no channel");
  return { gpui: pins.gpui, kit: pins.kit, rust };
}

/** Cargo.toml with both sets pinned at the given versions. */
export function setPins(cargoToml, { gpui, kit }) {
  let out = cargoToml;
  for (const [set, re] of PINS) out = out.replace(re, `$1${set === "gpui" ? gpui : kit}`);
  return out;
}

export const setToolchain = (toolchain, rust) =>
  toolchain.replace(/^channel = "[^"]+"/m, `channel = "${rust}"`);

/** check.sh with the crate's version and checksum. */
export const setCheck = (script, { version, sha256 }) =>
  script.replace(/^version=.*$/m, `version=${version}`).replace(/^sha256=.*$/m, `sha256=${sha256}`);

/** The patch files of patches/series, in the order they apply. */
export const parseSeries = (text) =>
  text
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith("#"));

/** The Zed commit a gpui-pre crate is a snapshot of, from its Cargo.toml. */
export const zedRev = (cargoToml) => /^zed-rev = "([0-9a-f]+)"/m.exec(cargoToml)?.[1] ?? null;

/**
 * @typedef {object} CargoMoves `cargo update --verbose`'s report
 * @property {{ name: string, from: string, to: string }[]} updated
 * @property {{ name: string, version: string }[]} added
 * @property {{ name: string, version: string }[]} removed
 * @property {{ name: string, version: string, latest: string }[]} behind Held by a requirement below the newest release.
 */

/** `cargo update --verbose`'s report: what moved, and what stayed behind its newest release. */
export function parseCargoUpdate(text) {
  /** @type {CargoMoves} */
  const moves = { updated: [], added: [], removed: [], behind: [] };
  for (const line of text.split("\n")) {
    let m;
    if ((m = /^\s*(?:Updating|Downgrading) (\S+) v(\S+)(?: \([^)]*\))? -> v(\S+)/.exec(line)))
      moves.updated.push({ name: m[1], from: m[2], to: m[3] });
    else if ((m = /^\s*Adding (\S+) v(\S+)/.exec(line)))
      moves.added.push({ name: m[1], version: m[2] });
    else if ((m = /^\s*Removing (\S+) v(\S+)/.exec(line)))
      moves.removed.push({ name: m[1], version: m[2] });
    else if ((m = /^\s*Unchanged (\S+) v(\S+) \(available: v(\S+)\)/.exec(line)))
      moves.behind.push({ name: m[1], version: m[2], latest: m[3] });
  }
  return moves;
}

/**
 * The crates the workspace's manifests name in a dependencies table, each with the version an `=` pin holds it at
 * (null when it has none).
 * @param {string[]} manifests
 */
export function directDeps(manifests) {
  /** @type {Map<string, string | null>} */
  const deps = new Map();
  for (const text of manifests) {
    let inDeps = false;
    for (const line of text.split("\n")) {
      const header = /^\s*\[([^\]]+)\]/.exec(line);
      if (header) {
        inDeps = /(^|\.)(dev-|build-)?dependencies$/.test(header[1]);
        continue;
      }
      const m = inDeps ? /^([A-Za-z0-9_-]+)(?:\.workspace)?\s*=\s*(.*)$/.exec(line) : null;
      if (!m) continue;
      const name = /package\s*=\s*"([^"]+)"/.exec(m[2])?.[1] ?? m[1];
      const pin = /^"=([^"]+)"/.exec(m[2])?.[1] ?? /version\s*=\s*"=([^"]+)"/.exec(m[2])?.[1];
      deps.set(name, pin ?? deps.get(name) ?? null);
    }
  }
  return deps;
}

/** Cargo.lock's packages: every locked version of each crate, with its source (null for a path). */
export function lockPackages(lock) {
  /** @type {Map<string, { version: string, source: string | null }[]>} */
  const packages = new Map();
  for (const block of lock.split("[[package]]").slice(1)) {
    const name = /^name = "([^"]+)"/m.exec(block)?.[1];
    const version = /^version = "([^"]+)"/m.exec(block)?.[1];
    if (!name || !version) continue;
    const source = /^source = "([^"]+)"/m.exec(block)?.[1] ?? null;
    packages.set(name, [...(packages.get(name) ?? []), { version, source }]);
  }
  return packages;
}

/**
 * Crates Grace pins with `=` to the copy GPUI brings (so the exe holds one), where the lock now also has a newer one
 * that it did not have `before` the update: GPUI moved, and the pin may have to follow it.
 * @param {Map<string, string | null>} deps
 * @param {ReturnType<typeof lockPackages>} packages
 * @param {ReturnType<typeof lockPackages>} [before]
 */
export function strayPins(deps, packages, before = new Map()) {
  /** @type {{ name: string, pin: string, newer: string[] }[]} */
  const out = [];
  for (const [name, pin] of deps) {
    if (!pin || GPUI.includes(name) || KIT.includes(name)) continue;
    const newer = (packages.get(name) ?? [])
      .map((p) => p.version)
      .filter((v) => parseVersion(v) && parseVersion(pin) && isNewer(v, pin))
      .filter((v) => !(before.get(name) ?? []).some((p) => p.version === v));
    if (newer.length) out.push({ name, pin, newer });
  }
  return out;
}

async function getText(fetchFn, url) {
  const res = await fetchFn(url);
  if (!res.ok) throw new Error(`GET ${url} failed: ${res.status} ${res.statusText}`);
  return res.text();
}

/** The .rej files `patch` left in `dir`, as text. */
function rejects(dir) {
  return readdirSync(dir, { recursive: true, encoding: "utf8" })
    .filter((f) => f.endsWith(".rej"))
    .map((f) => `\n${f}:\n${readFileSync(join(dir, f), "utf8")}`)
    .join("");
}

/**
 * Rebuilds patches/gpui-pre-windows from crates.io's `version` (checked against `sha256`) with each patch of
 * patches/series applied in order, writes each patch again as `git diff` writes it, and sets check.sh's version and
 * checksum. Throws a PatchError, having changed nothing, when a patch does not apply or the release already has it.
 * Returns the lines where `patch` applied a hunk with fuzz: worth a look by hand.
 */
export async function rebase(
  { version, sha256 },
  { root = ROOT, fetch: fetchFn = fetch, run = execRun } = {},
) {
  const work = mkdtempSync(join(tmpdir(), "grace-rebase-"));
  try {
    const url = `${CRATES}${PATCHED}/${PATCHED}-${version}.crate`;
    const res = await fetchFn(url);
    if (!res.ok) throw new Error(`GET ${url} failed: ${res.status} ${res.statusText}`);
    const body = Buffer.from(await res.arrayBuffer());
    if (createHash("sha256").update(body).digest("hex") !== sha256)
      throw new Error(`${PATCHED} ${version}: the .crate does not match crates.io's checksum`);
    writeFileSync(join(work, "crate.tar.gz"), body);
    run("tar", ["-xzf", "crate.tar.gz"], undefined, work);
    const dir = join(work, `${PATCHED}-${version}`);
    for (const f of DROPPED) rmSync(join(dir, f), { force: true });
    // a throwaway repository, so that each patch is written as `git diff` writes it
    const git = (...args) =>
      run(
        "git",
        [
          "-c",
          "user.name=bump-cargo",
          "-c",
          "user.email=bump-cargo@localhost",
          "-c",
          "core.autocrlf=false",
          "-c",
          "init.defaultBranch=main",
          "-c",
          "diff.noprefix=false",
          "-c",
          "diff.mnemonicPrefix=false",
          ...args,
        ],
        undefined,
        dir,
      );
    git("init", "-q");
    git("add", "-A", "--force");
    git("commit", "-q", "--no-gpg-sign", "--no-verify", "-m", "crates.io");
    const series = parseSeries(readFileSync(join(root, "patches", "series"), "utf8"));
    /** @type {string[]} */
    const fuzz = [];
    /** @type {Record<string, string>} */
    const written = {};
    for (const file of series) {
      const patch = join(root, "patches", file);
      let upstream = true;
      try {
        run(
          "patch",
          ["-p1", "-R", "--dry-run", "--force", "--silent", "-i", patch],
          undefined,
          dir,
        );
      } catch {
        upstream = false;
      }
      if (upstream)
        throw new PatchError(
          `${file}: ${PATCHED} ${version} already has it. Drop it (patches/README.md).`,
        );
      let out;
      try {
        out = run("patch", ["-p1", "--no-backup-if-mismatch", "-i", patch], undefined, dir);
      } catch (e) {
        throw new PatchError(
          `${file} does not apply to ${PATCHED} ${version}:\n${e.stdout ?? ""}${rejects(dir)}`,
        );
      }
      for (const line of out.split("\n"))
        if (/fuzz/.test(line)) fuzz.push(`${file}: ${line.trim()}`);
      git("add", "-A", "--force");
      written[file] = git("diff", "--cached", "--no-color", "--no-ext-diff");
      git("commit", "-q", "--no-gpg-sign", "--no-verify", "-m", file);
    }
    rmSync(join(dir, ".git"), { recursive: true, force: true });
    const copy = join(root, "patches", PATCHED);
    rmSync(copy, { recursive: true, force: true });
    cpSync(dir, copy, { recursive: true });
    for (const [file, text] of Object.entries(written))
      writeFileSync(join(root, "patches", file), text);
    const check = join(root, "patches", "check.sh");
    writeFileSync(check, setCheck(readFileSync(check, "utf8"), { version, sha256 }));
    return fuzz;
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
}

/** Runs `cargo update --verbose` in `root` and returns its report, which Cargo writes on stderr. */
function cargoUpdate(root) {
  const r = spawnSync("cargo", ["update", "--verbose"], {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 64 << 20,
  });
  if (r.status !== 0) throw new Error(`cargo update failed:\n${r.stderr}`);
  return r.stderr;
}

/**
 * Moves the pins, the patched copy and the lock to the newest releases. Changes nothing but the lock when everything
 * is current.
 */
export async function bump({
  root = ROOT,
  fetch: fetchFn = fetch,
  run = execRun,
  update = cargoUpdate,
} = {}) {
  const read = (file) => readFileSync(join(root, file), "utf8");
  const from = readPins(read("Cargo.toml"), read("rust-toolchain.toml"));
  const to = { ...from };
  /** @type {Record<string, any[]>} */
  const index = {};
  for (const name of [...GPUI, ...KIT, PATCHED])
    index[name] = parseIndex(await getText(fetchFn, `${INDEX}${indexPath(name)}`));
  const set = pickSet(index);
  const rust = parseRustStable(await getText(fetchFn, RUST_STABLE));
  /** @type {{ from: string | null, to: string | null }} */
  const zed = { from: zedRev(read(`patches/${PATCHED}/Cargo.toml`)), to: null };
  /** @type {string[]} */
  let fuzz = [];
  /** @type {{ gpui: string, kit: string, message: string } | null} */
  let patchError = null;
  const moves = set.gpui !== from.gpui || set.kit !== from.kit;
  if (moves && !isNewer(from.gpui, set.gpui) && !isNewer(from.kit, set.kit)) {
    try {
      fuzz = await rebase({ version: set.gpui, sha256: set.sha256 }, { root, fetch: fetchFn, run });
      writeFileSync(join(root, "Cargo.toml"), setPins(read("Cargo.toml"), set));
      Object.assign(to, { gpui: set.gpui, kit: set.kit });
    } catch (e) {
      if (!(e instanceof PatchError)) throw e;
      patchError = { gpui: set.gpui, kit: set.kit, message: e.message };
    }
  }
  zed.to = zedRev(read(`patches/${PATCHED}/Cargo.toml`));
  if (isNewer(rust, from.rust)) {
    writeFileSync(
      join(root, "rust-toolchain.toml"),
      setToolchain(read("rust-toolchain.toml"), rust),
    );
    to.rust = rust;
  }
  // the toolchain rust-toolchain.toml names, so that the lock is resolved by its Cargo
  run("rustup", ["toolchain", "install"], undefined, root);
  const before = lockPackages(read("Cargo.lock"));
  const cargo = parseCargoUpdate(update(root));
  const packages = lockPackages(read("Cargo.lock"));
  const patched = packages.get(PATCHED) ?? [];
  if (patched.length !== 1 || patched[0].source !== null || patched[0].version !== to.gpui)
    throw new Error(`Cargo.lock does not take ${PATCHED} ${to.gpui} from patches/: is it patched?`);
  const deps = directDeps(MANIFESTS.map(read));
  return {
    from,
    to,
    zed,
    fuzz,
    patchError,
    waiting: set.waiting,
    cargo,
    behind: cargo.behind.filter((b) => deps.has(b.name)),
    stray: strayPins(deps, packages, before),
    changed:
      to.gpui !== from.gpui ||
      to.kit !== from.kit ||
      to.rust !== from.rust ||
      cargo.updated.length + cargo.added.length + cargo.removed.length > 0,
  };
}

/** The bump branch of a day. */
export const branchFor = (date) => `feature/bump-cargo-${date}`;

/** The commit and PR title: what moved, in a header `cargo xtask check-pr` takes. */
export function titleFor(from, to) {
  const parts = [];
  if (to.gpui !== from.gpui) parts.push(`gpui-pre ${to.gpui}`);
  if (to.kit !== from.kit) parts.push(`gpui kit ${to.kit}`);
  if (to.rust !== from.rust) parts.push(`rust ${to.rust}`);
  if (!parts.length) return "build(deps): update cargo.lock";
  const title = `build(deps): bump ${parts.join(", ")}`;
  return title.length <= 72 ? title : "build(deps): bump the gpui set and rust";
}

const short = (rev) => rev.slice(0, 7);
const list = (items) => items.map((x) => `- ${x}`).join("\n");

/**
 * The PR body, or the local report: what moved, the patches, the lock, and what to check before merging.
 * @param {Awaited<ReturnType<typeof bump>>} r
 * @param {{ issue?: number | null, keep?: number[], branch?: string }} o
 */
export function summary(r, { issue = null, keep = [], branch } = {}) {
  const { from, to } = r;
  const rows = [];
  if (to.gpui !== from.gpui) {
    const changes =
      r.zed.from && r.zed.to
        ? `[zed@${short(r.zed.from)}...${short(r.zed.to)}](https://github.com/zed-industries/zed/compare/${r.zed.from}...${r.zed.to})`
        : "";
    rows.push(
      `| \`gpui-pre\`, \`gpui-pre-platform\`, patched \`${PATCHED}\` | ${from.gpui} | ${to.gpui} | ${changes} |`,
    );
  }
  if (to.kit !== from.kit)
    rows.push(
      `| GPUI Kit (\`gpui-component\`, \`gpui-base\`, \`gpui-kit-assets\`) | ${from.kit} | ${to.kit} | [notes](https://github.com/longbridge/gpui-kit/releases/tag/v${to.kit}), [changes](https://github.com/longbridge/gpui-kit/compare/v${from.kit}...v${to.kit}) |`,
    );
  if (to.rust !== from.rust)
    rows.push(
      `| Rust toolchain | ${from.rust} | ${to.rust} | [notes](https://github.com/rust-lang/rust/releases/tag/${to.rust}) |`,
    );
  const parts = [];
  parts.push(
    rows.length
      ? ["| | From | To | Upstream |", "|---|---|---|---|", ...rows].join("\n")
      : `The GPUI set (gpui-pre ${to.gpui}, GPUI Kit ${to.kit}) and Rust ${to.rust} are the newest releases: only Cargo.lock moves.`,
  );
  const patches = [];
  if (to.gpui !== from.gpui)
    patches.push(
      `Every patch of \`patches/series\` applied to \`${PATCHED}\` ${to.gpui} and was written again from it; \`patches/check.sh\` checks the copy.`,
    );
  if (r.fuzz.length)
    patches.push(
      `Applied with fuzz, next to lines upstream changed; \`test\` checks what each patch does (\`gpui-idle\` the idle patch, the Windows e2e the focus patch):\n${list(r.fuzz.map((f) => `\`${f}\``))}`,
    );
  if (r.patchError)
    patches.push(
      `gpui-pre ${r.patchError.gpui} and GPUI Kit ${r.patchError.kit} are out, but the patches do not move to them${issue ? ` (#${issue})` : ""}, so the GPUI set stays at ${to.gpui}:\n\n\`\`\`\n${r.patchError.message.trim()}\n\`\`\``,
    );
  if (r.waiting)
    patches.push(`gpui-pre ${r.waiting} is out, but no GPUI Kit release takes it yet.`);
  if (patches.length) parts.push(`### GPUI and its patches\n\n${patches.join("\n\n")}`);
  const { updated, added, removed } = r.cargo;
  const lock = [
    ...updated.map((u) => `\`${u.name}\` ${u.from} → ${u.to}`),
    ...added.map((a) => `\`${a.name}\` ${a.version} added`),
    ...removed.map((a) => `\`${a.name}\` ${a.version} removed`),
  ];
  if (lock.length)
    parts.push(
      `### Cargo.lock\n\n<details><summary>${updated.length} updated, ${added.length} added, ${removed.length} removed</summary>\n\n${list(lock)}\n\n</details>`,
    );
  const held = [
    ...r.behind.map(
      (b) =>
        `\`${b.name}\` stays at ${b.version}: ${b.latest} is out, beyond what its requirement allows.`,
    ),
    ...r.stray.map(
      (s) =>
        `\`${s.name}\` is pinned at =${s.pin}, and the lock now also has ${s.newer.join(", ")}: if GPUI moved to it, move the pin too, so the exe holds one copy.`,
    ),
  ];
  if (held.length) parts.push(`### Held back\n\n${list(held)}`);
  const ref = branch ?? "<branch>";
  parts.push(
    [
      "### Merging",
      "",
      `This PR merges itself once \`checks\`, \`test\` and \`perf\`, which \`cargo-updates\` starts on its commit, all pass (ADR 0033). \`test\` runs clippy, \`cargo test\`, \`patches/check.sh\`, GPUI Grace's e2e on Linux and Windows, and \`gpui-idle\`, which fails when GPUI Grace wakes more than 5 times a second while minimised. A failed run leaves the PR open with a comment, and \`cargo-updates\` fails. To run them again: \`gh workflow run test.yml --ref ${ref}\`, then merge by hand.`,
      "",
      `Part of #${ISSUE}.`,
    ].join("\n"),
  );
  if (keep.length)
    parts.push(
      keep
        .map((n) => `#${n} is still open and has changes by people: carry them over or close it.`)
        .join("\n"),
    );
  return parts.join("\n\n") + "\n";
}

/** The issue for patches that no longer apply: an open one with the same title, or a new one. */
export function patchIssue(error, run = execRun) {
  const title = `Rebase the GPUI patches on gpui-pre ${error.gpui}`;
  const open = JSON.parse(
    run("gh", [
      "issue",
      "list",
      "--state",
      "open",
      "--search",
      `"${title}" in:title`,
      "--json",
      "number,title",
    ]),
  );
  const found = open.find((i) => i.title === title);
  if (found) return found.number;
  const body = [
    `gpui-pre ${error.gpui} and GPUI Kit ${error.kit} are out, but \`cargo-updates\` could not move GPUI Grace to them (ADR 0032):`,
    "",
    "```",
    error.message.trim(),
    "```",
    "",
    "Rebase the patches by hand, as `patches/README.md` says, or drop the ones upstream now has. Until then the weekly bump moves only Rust and Cargo.lock.",
  ].join("\n");
  const url = run("gh", ["issue", "create", "--title", title, "--body-file", "-"], body).trim();
  return Number(url.split("/").pop());
}

/**
 * Commits the bump on `feature/bump-cargo-<date>`, opens its PR into develop, starts the runs of GATES on it, and closes
 * older bump PRs that hold only the bot's commits. Returns the PR's number and commit, or null when the day's branch
 * already has a PR.
 * @param {Awaited<ReturnType<typeof bump>>} r
 * @param {{ date: string, run?: typeof execRun }} o
 * @returns {{ number: number, sha: string } | null}
 */
export function openPr(r, { date, run = execRun }) {
  const gh = (...args) => JSON.parse(run("gh", args));
  const branch = branchFor(date);
  const prs = gh("pr", "list", "--head", branch, "--state", "all", "--json", "number");
  if (prs.length) {
    console.log(`${branch}: already has a PR (#${prs[0].number})`);
    return null;
  }
  const issue = r.patchError ? patchIssue(r.patchError, run) : null;
  const title = titleFor(r.from, r.to);
  const exists =
    run("git", ["ls-remote", "--heads", "origin", `refs/heads/${branch}`]).trim() !== "";
  if (!exists) {
    run("git", ["switch", "-c", branch]);
    run("git", ["add", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "patches"]);
    run("git", [
      "-c",
      "user.name=github-actions[bot]",
      "-c",
      `user.email=${BOT_EMAIL}`,
      "commit",
      "-m",
      title,
    ]);
    run("git", ["push", "origin", branch]);
  }
  const sha = run("git", ["ls-remote", "origin", `refs/heads/${branch}`]).split(/\s/)[0];
  const open = gh(
    "pr",
    "list",
    "--state",
    "open",
    "--base",
    "develop",
    "--limit",
    "100",
    "--json",
    "number,headRefName",
  );
  const close = [];
  const keep = [];
  for (const pr of open) {
    if (!pr.headRefName.startsWith(branchFor("")) || pr.headRefName >= branch) continue;
    const { commits } = gh("pr", "view", String(pr.number), "--json", "commits");
    const botOnly = commits.every((c) => c.authors.every((a) => a.email === BOT_EMAIL));
    (botOnly ? close : keep).push(pr.number);
  }
  const body = summary(r, { issue, keep, branch });
  const url = run(
    "gh",
    ["pr", "create", "--base", "develop", "--head", branch, "--title", title, "--body-file", "-"],
    body,
  ).trim();
  console.log(`opened ${url}`);
  const number = Number(url.split("/").pop());
  // a PR opened with GITHUB_TOKEN starts no pull_request workflow, but a dispatch does
  for (const workflow of GATES) run("gh", ["workflow", "run", workflow, "--ref", branch]);
  for (const n of close)
    run("gh", [
      "pr",
      "close",
      String(n),
      "--comment",
      `Superseded by #${number}`,
      "--delete-branch",
    ]);
  return { number, sha };
}

/**
 * Waits for the runs of GATES started on `sha` by hand, and merges PR `pr` with a merge commit once all pass, with a
 * comment that links them: the newest run of each counts, so a run started again by hand replaces a cancelled one. A failed run, or none done
 * within `timeout`, leaves the PR open with a comment and throws (ADR 0033).
 * @param {number} pr
 * @param {string} sha
 * @param {{ run?: typeof execRun, wait?: (ms: number) => Promise<void>, now?: () => number, interval?: number, timeout?: number }} o
 */
export async function mergeWhenGreen(
  pr,
  sha,
  {
    run = execRun,
    wait = (ms) => new Promise((done) => setTimeout(done, ms)),
    now = Date.now,
    interval = 60_000,
    timeout = 110 * 60_000,
  } = {},
) {
  const end = now() + timeout;
  const stop = (why) => {
    run("gh", ["pr", "comment", String(pr), "--body-file", "-"], `Not merged: ${why}`);
    throw new Error(`#${pr} not merged: ${why}`);
  };
  let runs;
  for (;;) {
    runs = GATES.map((workflow) => {
      const latest = run("gh", [
        "api",
        `repos/{owner}/{repo}/actions/workflows/${workflow}/runs?event=workflow_dispatch&head_sha=${sha}&per_page=1`,
        "--jq",
        ".workflow_runs[0] // {} | {status, conclusion, url: .html_url}",
      ]);
      return { workflow, ...JSON.parse(latest) };
    });
    const failed = runs.filter((r) => r.status === "completed" && r.conclusion !== "success");
    if (failed.length)
      stop(
        failed.map((r) => `\`${r.workflow}\` ended ${r.conclusion} (${r.url})`).join(", ") +
          ". Fix it on this branch, or run it again with `gh workflow run <workflow> --ref <branch>`, then merge by hand.",
      );
    if (runs.every((r) => r.status === "completed")) break;
    if (now() >= end) {
      const pending = runs.filter((r) => r.status !== "completed").map((r) => `\`${r.workflow}\``);
      stop(
        `${pending.join(", ")} had not finished on ${sha} after ${timeout / 60_000} minutes. Merge by hand once it passes.`,
      );
    }
    await wait(interval);
  }
  // the runs are not on the PR's checks tab: link them, perf's table included (its run's summary)
  run(
    "gh",
    ["pr", "comment", String(pr), "--body-file", "-"],
    `Passed on ${sha}: ${runs.map((r) => `[\`${r.workflow}\`](${r.url})`).join(", ")}. \`perf\`'s table is in its run's summary.`,
  );
  try {
    // --match-head-commit refuses the merge if a push landed after these runs
    run("gh", [
      "pr",
      "merge",
      String(pr),
      "--merge",
      "--delete-branch",
      "--match-head-commit",
      sha,
    ]);
  } catch (err) {
    stop(
      `the merge failed: ${String(err.stderr || err.message)
        .trim()
        .replace(/\.$/, "")}. Merge by hand.`,
    );
  }
  console.log(`merged #${pr}`);
}

async function main() {
  const [flag, ...args] = process.argv.slice(2);
  if (
    flag === "--merge" &&
    args.length === 2 &&
    /^\d+$/.test(args[0]) &&
    /^[0-9a-f]{40}$/.test(args[1])
  )
    return mergeWhenGreen(Number(args[0]), args[1]);
  if (flag && flag !== "--pr") {
    console.error(
      "usage: node scripts/bump-cargo.mjs [--pr]\n       node scripts/bump-cargo.mjs --merge <pr> <sha>",
    );
    process.exit(2);
  }
  const r = await bump();
  if (!r.changed) {
    console.log(
      `gpui-pre ${r.to.gpui}, GPUI Kit ${r.to.kit}, Rust ${r.to.rust} and Cargo.lock are current: nothing to open.`,
    );
    if (r.patchError) {
      console.log(`The GPUI set stays behind:\n${r.patchError.message}`);
      if (flag) console.log(`Issue #${patchIssue(r.patchError)}`);
    }
    return;
  }
  console.log(summary(r));
  if (!flag) return;
  const pr = openPr(r, { date: new Date().toISOString().slice(0, 10) });
  if (process.env.GITHUB_OUTPUT && pr)
    appendFileSync(process.env.GITHUB_OUTPUT, `pr=${pr.number}\nsha=${pr.sha}\n`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((e) => {
    console.error(e.message);
    process.exit(1);
  });
}
