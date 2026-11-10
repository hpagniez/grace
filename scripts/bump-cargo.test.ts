import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  PATCHED,
  PatchError,
  ROOT,
  directDeps,
  indexPath,
  isNewer,
  lockPackages,
  newest,
  parseCargoUpdate,
  parseRustStable,
  parseSeries,
  pickSet,
  readPins,
  rebase,
  setCheck,
  setPins,
  setToolchain,
  strayPins,
  summary,
  titleFor,
  zedRev,
} from "./bump-cargo.mjs";
import { commitError } from "./check-pr.mjs";

const read = (file: string) => readFileSync(join(ROOT, file), "utf8");

type Dep = { name: string; req: string; kind?: string; package?: string };
const entry = (name: string, vers: string, deps: Dep[] = [], extra: object = {}) => ({
  name,
  vers,
  deps,
  cksum: `${vers}-sum`,
  yanked: false,
  ...extra,
});
const pinsGpui = (v: string): Dep[] => [
  { name: "gpui", package: "gpui-pre", req: `=${v}`, kind: "normal" },
  { name: "gpui", package: "gpui-pre", req: `=${v}`, kind: "dev" },
];

/** An index where GPUI Kit `kit` pins gpui-pre `gpui`, and `latest` is the newest gpui-pre. */
function index(kit: string, gpui: string, latest = gpui) {
  const gpuiVersions = [gpui, latest].map((v) => entry("x", v));
  return {
    "gpui-pre": gpuiVersions,
    "gpui-pre-platform": gpuiVersions,
    [PATCHED]: gpuiVersions,
    "gpui-component": [
      entry("gpui-component", "0.9.0", pinsGpui("0.3.11")),
      entry("gpui-component", kit, pinsGpui(gpui)),
    ],
    "gpui-base": [entry("gpui-base", kit)],
    "gpui-kit-assets": [entry("gpui-kit-assets", kit)],
  };
}

const result = (over: Record<string, unknown> = {}) => ({
  from: { gpui: "0.3.11", kit: "0.9.0", rust: "1.97.0" },
  to: { gpui: "0.3.12", kit: "0.9.1", rust: "1.99.1" },
  zed: {
    from: "f816e1f9263e3a8ef63c4baaa02e1441ce1c6038",
    to: "d2a4fb6ded7c470be25d1541281da3882495b65b",
  },
  fuzz: [] as string[],
  patchError: null as null | { gpui: string; kit: string; message: string },
  waiting: null as string | null,
  cargo: {
    updated: [{ name: "syn", from: "3.0.9", to: "3.0.10" }],
    added: [{ name: "multiversion", version: "0.8.1" }],
    removed: [{ name: "pulp", version: "0.22.3" }],
    behind: [],
  },
  behind: [] as { name: string; version: string; latest: string }[],
  stray: [] as { name: string; pin: string; newer: string[] }[],
  changed: true,
  ...over,
});

describe("indexPath", () => {
  it("files a crate as crates.io's sparse index does", () => {
    expect(indexPath("gpui-pre")).toBe("gp/ui/gpui-pre");
    expect(indexPath("Syn")).toBe("3/s/syn");
    expect(indexPath("cc")).toBe("2/cc");
    expect(indexPath("a")).toBe("1/a");
  });
});

describe("newest", () => {
  it("compares as numbers and skips yanked releases and pre-releases", () => {
    const entries = [
      entry("c", "0.3.9"),
      entry("c", "0.3.12"),
      entry("c", "0.3.13", [], { yanked: true }),
      entry("c", "0.4.0-alpha.1"),
    ];
    expect(newest(entries).vers).toBe("0.3.12");
    expect(isNewer("1.100.0", "1.99.1")).toBe(true);
    expect(() => newest([entry("c", "1.0.0-rc.1")])).toThrow(/no release/);
  });
});

describe("pickSet", () => {
  it("takes the newest GPUI Kit and the gpui-pre it pins", () => {
    expect(pickSet(index("0.9.1", "0.3.12"))).toEqual({
      gpui: "0.3.12",
      kit: "0.9.1",
      sha256: "0.3.12-sum",
      waiting: null,
    });
  });

  it("names a newer gpui-pre that no GPUI Kit takes yet", () => {
    expect(pickSet(index("0.9.1", "0.3.12", "0.3.13")).waiting).toBe("0.3.13");
  });

  it("refuses a GPUI Kit that does not pin gpui-pre exactly, or comes without its other crates", () => {
    const loose = index("0.9.1", "0.3.12");
    loose["gpui-component"][1].deps = [
      { name: "gpui", package: "gpui-pre", req: "^0.3.12", kind: "normal" },
    ];
    expect(() => pickSet(loose)).toThrow(/does not pin one gpui-pre exactly/);
    const partial = index("0.9.1", "0.3.12");
    partial["gpui-base"] = [entry("gpui-base", "0.9.0")];
    expect(() => pickSet(partial)).toThrow(/gpui-base 0.9.1 is not on crates.io/);
  });
});

describe("parseRustStable", () => {
  it("reads rustc's version from the stable channel manifest", () => {
    const manifest =
      '[pkg.cargo]\nversion = "0.100.0 (5f94df478 2026-08-27)"\n\n[pkg.rustc]\nversion = "1.99.1 (58cf8f9c3 2026-10-13)"\n';
    expect(parseRustStable(manifest)).toBe("1.99.1");
    expect(() => parseRustStable("[pkg.cargo]\n")).toThrow();
  });
});

describe("pins", () => {
  it("reads and moves both sets and the toolchain in the repository's files", () => {
    const cargo = read("Cargo.toml");
    const toolchain = read("rust-toolchain.toml");
    const pins = readPins(cargo, toolchain);
    expect(Object.values(pins).every((v) => /^\d+\.\d+\.\d+$/.test(v))).toBe(true);
    const moved = setPins(cargo, { gpui: "9.9.9", kit: "8.8.8" });
    expect(readPins(moved, setToolchain(toolchain, "1.200.0"))).toEqual({
      gpui: "9.9.9",
      kit: "8.8.8",
      rust: "1.200.0",
    });
    expect(moved.split("\n").length).toBe(cargo.split("\n").length);
  });

  it("refuses a set pinned at two versions", () => {
    const cargo = read("Cargo.toml").replace(/^(gpui-base = "=)[^"]+/m, "$10.0.1");
    expect(() => readPins(cargo, read("rust-toolchain.toml"))).toThrow(/kit set at both/);
  });

  it("sets check.sh's version and checksum, and reads the series in order", () => {
    const check = setCheck(read("patches/check.sh"), { version: "0.3.99", sha256: "f".repeat(64) });
    expect(check).toMatch(/^version=0\.3\.99$/m);
    expect(check).toMatch(new RegExp(`^sha256=${"f".repeat(64)}$`, "m"));
    expect(parseSeries(read("patches/series"))).toEqual([
      "gpui-pre-windows-idle.patch",
      "gpui-pre-windows-a11y-focus.patch",
    ]);
    expect(zedRev(read(`patches/${PATCHED}/Cargo.toml`))).toMatch(/^[0-9a-f]{40}$/);
  });
});

describe("parseCargoUpdate", () => {
  it("reads what moved and what stays behind", () => {
    const text = [
      "    Updating crates.io index",
      "     Locking 39 packages to highest Rust 1.99.1 compatible versions",
      "    Updating gpui-pre v0.3.11 -> v0.3.12",
      "    Updating gpui-pre-windows v0.3.11 (/w/patches/gpui-pre-windows) -> v0.3.12",
      "      Adding multiversion v0.8.1",
      "    Removing pulp v0.22.3",
      "   Unchanged x11rb v0.13.2 (available: v0.14.0)",
    ].join("\n");
    expect(parseCargoUpdate(text)).toEqual({
      updated: [
        { name: "gpui-pre", from: "0.3.11", to: "0.3.12" },
        { name: "gpui-pre-windows", from: "0.3.11", to: "0.3.12" },
      ],
      added: [{ name: "multiversion", version: "0.8.1" }],
      removed: [{ name: "pulp", version: "0.22.3" }],
      behind: [{ name: "x11rb", version: "0.13.2", latest: "0.14.0" }],
    });
  });
});

describe("directDeps and strayPins", () => {
  it("finds the workspace's own dependencies and their exact pins", () => {
    const deps = directDeps(
      [
        "Cargo.toml",
        "crates/grace/Cargo.toml",
        "crates/grace-core/Cargo.toml",
        "crates/grace-e2e/Cargo.toml",
      ].map(read),
    );
    expect(deps.get("windows")).toMatch(/^\d+\.\d+\.\d+$/);
    expect(deps.get("gpui-pre")).toMatch(/^\d+\.\d+\.\d+$/);
    expect(deps.get("serde")).toBeNull();
    expect(deps.has("tempfile")).toBe(true);
    expect(deps.has("e2e")).toBe(false);
  });

  it("names a pinned crate the lock now also has in a newer version", () => {
    const lock = [
      "version = 4",
      '[[package]]\nname = "windows"\nversion = "0.62.2"\nsource = "registry+https://github.com/rust-lang/crates.io-index"',
      '[[package]]\nname = "windows"\nversion = "0.63.0"\nsource = "registry+https://github.com/rust-lang/crates.io-index"',
      '[[package]]\nname = "windows"\nversion = "0.48.0"\nsource = "registry+https://github.com/rust-lang/crates.io-index"',
      `[[package]]\nname = "${PATCHED}"\nversion = "0.3.12"`,
    ].join("\n\n");
    const packages = lockPackages(lock);
    expect(packages.get(PATCHED)).toEqual([{ version: "0.3.12", source: null }]);
    const deps = new Map<string, string | null>([
      ["windows", "0.62.2"],
      ["serde", null],
      ["gpui-pre", "0.3.11"],
    ]);
    expect(strayPins(deps, packages)).toEqual([
      { name: "windows", pin: "0.62.2", newer: ["0.63.0"] },
    ]);
    // a newer copy the lock already held, for another crate, is no news
    const before = lockPackages(lock.replace('version = "0.48.0"', 'version = "0.63.0"'));
    expect(strayPins(deps, packages, before)).toEqual([]);
  });
});

describe("titleFor", () => {
  const from = { gpui: "0.3.11", kit: "0.9.0", rust: "1.97.0" };

  it("names what moved, in a header the PR check takes", () => {
    const titles = [
      titleFor(from, { gpui: "0.3.12", kit: "0.9.1", rust: "1.99.1" }),
      titleFor(from, { ...from, rust: "1.100.0" }),
      titleFor(from, from),
      titleFor(from, { gpui: "0.3.1200", kit: "0.9.1000", rust: "1.1000.10" }),
    ];
    expect(titles).toEqual([
      "build(deps): bump gpui-pre 0.3.12, gpui kit 0.9.1, rust 1.99.1",
      "build(deps): bump rust 1.100.0",
      "build(deps): update cargo.lock",
      "build(deps): bump gpui-pre 0.3.1200, gpui kit 0.9.1000, rust 1.1000.10",
    ]);
    for (const t of titles) expect(commitError(t)).toBeNull();
  });
});

describe("summary", () => {
  it("lists what moved with its upstream changes, the lock and the checks", () => {
    const body = summary(result(), { branch: "feature/bump-cargo-2026-11-17" });
    expect(body).toContain(
      "| 0.3.11 | 0.3.12 | [zed@f816e1f...d2a4fb6](https://github.com/zed-industries/zed/compare/f816e1f9263e3a8ef63c4baaa02e1441ce1c6038...d2a4fb6ded7c470be25d1541281da3882495b65b) |",
    );
    expect(body).toContain("https://github.com/longbridge/gpui-kit/compare/v0.9.0...v0.9.1");
    expect(body).toContain("https://github.com/rust-lang/rust/releases/tag/1.99.1");
    expect(body).toContain("1 updated, 1 added, 1 removed");
    expect(body).toContain("gh workflow run test.yml --ref feature/bump-cargo-2026-11-17");
  });

  it("says when only the lock moves, and why the GPUI set stays", () => {
    const same = { gpui: "0.3.11", kit: "0.9.0", rust: "1.99.1" };
    const body = summary(
      result({
        from: same,
        to: same,
        patchError: {
          gpui: "0.3.12",
          kit: "0.9.1",
          message: "gpui-pre-windows-idle.patch does not apply",
        },
        waiting: "0.3.13",
        fuzz: [
          "gpui-pre-windows-idle.patch: Hunk #1 succeeded at 422 with fuzz 2 (offset -14 lines).",
        ],
        stray: [{ name: "windows", pin: "0.62.2", newer: ["0.63.0"] }],
      }),
      { issue: 401, keep: [399] },
    );
    expect(body).toContain("only Cargo.lock moves");
    expect(body).toContain("do not move to them (#401), so the GPUI set stays at 0.3.11");
    expect(body).toContain("gpui-pre 0.3.13 is out, but no GPUI Kit release takes it yet.");
    expect(body).toContain("with fuzz 2");
    expect(body).toContain("`windows` is pinned at =0.62.2");
    expect(body).toContain("#399 is still open");
  });
});

// The bump runs on Linux (cargo-updates.yml), with git, tar and patch.
describe.skipIf(process.platform === "win32")("rebase", () => {
  /** A repository with a patched copy of `PATCHED` 1.0.0 and one patch, and crates.io holding 1.0.1. */
  function setup(next: string) {
    const root = mkdtempSync(join(tmpdir(), "grace-bump-cargo-"));
    const patches = join(root, "patches");
    mkdirSync(join(patches, PATCHED, "src"), { recursive: true });
    writeFileSync(join(patches, PATCHED, "Cargo.toml"), 'version = "1.0.0"\n');
    writeFileSync(join(patches, PATCHED, "src", "lib.rs"), "fn a() {}\nfn b() {}\n// patched\n");
    writeFileSync(join(patches, "series"), "# in order\nx-one.patch\n");
    writeFileSync(
      join(patches, "x-one.patch"),
      "--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1,2 +1,3 @@\n fn a() {}\n fn b() {}\n+// patched\n",
    );
    writeFileSync(join(patches, "check.sh"), "#!/bin/sh\nversion=1.0.0\nsha256=old\n");
    const upstream = join(mkdtempSync(join(tmpdir(), "grace-bump-crate-")), `${PATCHED}-1.0.1`);
    mkdirSync(join(upstream, "src"), { recursive: true });
    writeFileSync(join(upstream, "Cargo.toml"), 'version = "1.0.1"\n');
    writeFileSync(join(upstream, "Cargo.toml.orig"), "dropped\n");
    writeFileSync(join(upstream, "src", "lib.rs"), next);
    execFileSync("tar", ["-czf", "c.crate", `${PATCHED}-1.0.1`], { cwd: join(upstream, "..") });
    const crate = readFileSync(join(upstream, "..", "c.crate"));
    const sha256 = createHash("sha256").update(crate).digest("hex");
    const fetchCrate = (async () => new Response(crate)) as unknown as typeof fetch;
    return { root, patches, sha256, fetch: fetchCrate };
  }

  it("rebuilds the copy from the new release with the patches, and writes them again", async () => {
    const { root, patches, sha256, fetch } = setup("// new\nfn a() {}\nfn b() {}\n");
    const fuzz = await rebase({ version: "1.0.1", sha256 }, { root, fetch });
    expect(fuzz).toEqual([]);
    expect(readFileSync(join(patches, PATCHED, "src", "lib.rs"), "utf8")).toBe(
      "// new\nfn a() {}\nfn b() {}\n// patched\n",
    );
    expect(readFileSync(join(patches, PATCHED, "Cargo.toml"), "utf8")).toBe('version = "1.0.1"\n');
    expect(() => readFileSync(join(patches, PATCHED, "Cargo.toml.orig"))).toThrow();
    const patch = readFileSync(join(patches, "x-one.patch"), "utf8");
    expect(patch).toMatch(
      /^diff --git a\/src\/lib\.rs b\/src\/lib\.rs\nindex [0-9a-f]{7}\.\.[0-9a-f]{7} 100644\n/,
    );
    expect(patch).toContain("@@ -1,3 +1,4 @@");
    expect(readFileSync(join(patches, "check.sh"), "utf8")).toBe(
      `#!/bin/sh\nversion=1.0.1\nsha256=${sha256}\n`,
    );
  });

  it("changes nothing when a patch does not apply, or the release already has it", async () => {
    for (const next of ["fn z() {}\n", "fn a() {}\nfn b() {}\n// patched\n"]) {
      const { root, patches, sha256, fetch } = setup(next);
      const before = readFileSync(join(patches, "x-one.patch"), "utf8");
      const err = await rebase({ version: "1.0.1", sha256 }, { root, fetch }).catch((e) => e);
      expect(err).toBeInstanceOf(PatchError);
      expect(err.message).toMatch(next.includes("patched") ? /already has it/ : /does not apply/);
      expect(readFileSync(join(patches, "x-one.patch"), "utf8")).toBe(before);
      expect(readFileSync(join(patches, PATCHED, "Cargo.toml"), "utf8")).toBe(
        'version = "1.0.0"\n',
      );
    }
  });

  it("refuses a crate that does not match crates.io's checksum", async () => {
    const { root, fetch } = setup("fn a() {}\nfn b() {}\n");
    await expect(
      rebase({ version: "1.0.1", sha256: "0".repeat(64) }, { root, fetch }),
    ).rejects.toThrow(/does not match/);
  });
});
