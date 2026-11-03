import { chmodSync, mkdirSync, mkdtempSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import { counts, watchFeatures, type FeatureWatcher } from "./watch";

const QUIET = 50;
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const settle = () => sleep(400);

const temps: string[] = [];
const watchers: FeatureWatcher[] = [];
// Each test gets <parent>/root, so a test can rename the parent.
const tmp = () => {
  const parent = mkdtempSync(path.join(tmpdir(), "kk-watch-"));
  temps.push(parent);
  const root = path.join(parent, "root");
  mkdirSync(root);
  return root;
};
const write = (file: string, text = "Feature: x\n") => {
  mkdirSync(path.dirname(file), { recursive: true });
  writeFileSync(file, text);
};
afterEach(() => {
  for (const w of watchers.splice(0)) w.close();
  for (const d of temps.splice(0)) rmSync(d, { recursive: true, force: true, maxRetries: 5 });
});

function start(roots: string[]) {
  const onSaved = vi.fn<(files: string[]) => void>();
  const onError = vi.fn<(err: Error) => void>();
  const w = watchFeatures(roots, onSaved, onError, QUIET);
  watchers.push(w);
  return { w, onSaved, onError };
}

describe("counts (R2.2)", () => {
  it("takes a .feature in any case and folder", () => {
    for (const p of ["a.feature", "x\\B.FEATURE", "x/y/c.feature"]) expect(counts(p)).toBe(true);
  });

  it("leaves other files and the skipped folders", () => {
    for (const p of [
      "karate-config.js",
      "a.feature.bak",
      "node_modules\\x\\c.feature",
      "x\\.git\\a.feature",
      ".gradle/b.feature",
      "NODE_MODULES/c.feature",
    ])
      expect(counts(p)).toBe(false);
  });
});

describe("watchFeatures", () => {
  it("hands a saved feature over once, as an absolute path", async () => {
    const root = tmp();
    const { onSaved } = start([root]);
    write(path.join(root, "sub", "a.feature"));
    await settle();
    expect(onSaved.mock.calls).toEqual([[[path.join(root, "sub", "a.feature")]]]);
  });

  it("makes two saves within the quiet time one batch (R2.3)", async () => {
    const root = tmp();
    const { onSaved } = start([root]);
    write(path.join(root, "a.feature"));
    await sleep(20);
    write(path.join(root, "b.feature"));
    await settle();
    expect(onSaved).toHaveBeenCalledTimes(1);
    expect(onSaved.mock.calls[0][0].sort()).toEqual([
      path.join(root, "a.feature"),
      path.join(root, "b.feature"),
    ]);
  });

  it("does not count other files", async () => {
    const root = tmp();
    const { onSaved } = start([root]);
    write(path.join(root, "karate-config.js"));
    write(path.join(root, "data.json"));
    write(path.join(root, "node_modules", "x", "c.feature"));
    await settle();
    expect(onSaved).not.toHaveBeenCalled();
  });

  it("drops a feature deleted before its batch", async () => {
    const root = tmp();
    const { onSaved } = start([root]);
    write(path.join(root, "a.feature"));
    rmSync(path.join(root, "a.feature"));
    await settle();
    expect(onSaved).not.toHaveBeenCalled();
  });

  it("does not count an event with no new modified time", async () => {
    const root = tmp();
    const old = path.join(root, "old.feature");
    write(old);
    await sleep(50); // the watcher starts after the file's modified time
    const { onSaved } = start([root]);
    chmodSync(old, 0o444); // an attribute event, the modified time unchanged
    const file = path.join(root, "a.feature");
    write(file);
    await settle();
    expect(onSaved.mock.calls).toEqual([[[file]]]);
    chmodSync(file, 0o444);
    await settle();
    expect(onSaved.mock.calls).toEqual([[[file]]]);
    chmodSync(old, 0o666); // so afterEach can delete them
    chmodSync(file, 0o666);
  });

  it("watches a root that is a file", async () => {
    const root = tmp();
    const file = path.join(root, "one.feature");
    write(file);
    await sleep(50);
    const { onSaved } = start([file]);
    write(file, "Feature: changed\n");
    await settle();
    expect(onSaved.mock.calls).toEqual([[[file]]]);
  });

  it("throws for a root it cannot watch", () => {
    const root = tmp();
    expect(() => watchFeatures([root, path.join(root, "missing")], vi.fn(), vi.fn())).toThrow();
  });

  it("reports one error and stops when a watched root is deleted (R2.5)", async () => {
    const root = tmp();
    const { onSaved, onError } = start([root]);
    rmSync(root, { recursive: true });
    await settle();
    expect(onError).toHaveBeenCalledTimes(1);
    expect(onSaved).not.toHaveBeenCalled();
    // Windows then reports the root's own path in a loop: a stopped watcher burns no CPU
    const cpu = process.cpuUsage();
    await sleep(300);
    expect(process.cpuUsage(cpu).user).toBeLessThan(100_000);
  });

  it.each([
    ["renamed", (root: string) => renameSync(root, `${root}-renamed`)],
    ["moved away", (root: string) => renameSync(root, `${path.dirname(root)}-gone`)],
  ])("reports one error when a watched root is %s (R2.5)", async (_, move) => {
    const root = tmp();
    const { onError } = start([root]);
    move(root);
    await settle();
    expect(onError).toHaveBeenCalledTimes(1);
    temps.push(`${root}-renamed`, `${path.dirname(root)}-gone`);
  });

  it("hands over every feature of a burst the OS cannot report whole (R2.3)", async () => {
    const root = tmp();
    const { onSaved } = start([root]);
    const files = Array.from({ length: 300 }, (_, i) => path.join(root, "m", `f${i}.feature`));
    for (const f of files) write(f); // in one tick: Windows' change buffer overflows
    await sleep(1000);
    expect(onSaved.mock.calls.flatMap(([saved]) => saved).sort()).toEqual([...files].sort());
  });

  it("stops on close, and frees the root's parent folder", async () => {
    const root = tmp();
    const { w, onSaved } = start([root]);
    w.close();
    w.close();
    write(path.join(root, "a.feature"));
    await settle();
    expect(onSaved).not.toHaveBeenCalled();
    const parent = path.dirname(root);
    renameSync(parent, `${parent}-moved`);
    renameSync(`${parent}-moved`, parent);
  });
});
