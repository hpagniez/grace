// Helpers shared by the runner test files, src/main/runner.*.test.ts.
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { copyFixture } from "./fixtures/copy";
import type { AppEvent } from "../src/shared/events";
import type { RunRecord, StoredTest } from "../src/shared/history";
import type { TestNode } from "../src/shared/model";
import { NO_FILTER } from "../src/shared/tags";
import { createRunDir, writeRunRecord } from "../src/main/history";
import { readAllEvents } from "../src/main/karate/events";
import { leafKeys } from "../src/main/karate/tree";
import { Runner } from "../src/main/runner";
import { fakeSecretBox, type SecretBox } from "../src/main/secrets";
import { vendorPaths, type VendorPaths } from "../src/main/vendor";

export const vendor = vendorPaths(path.resolve("vendor"));
export const temps: string[] = [];
const runners: Runner[] = [];
export const T = { timeout: 120_000 };

/** Disposes the test's runners and removes its temp folders. Each runner test file passes it to `afterEach`. */
export async function cleanUp(): Promise<void> {
  await Promise.all(runners.splice(0).map((r) => r.dispose()));
  // Windows may hold a file briefly after a JVM exits
  for (const d of temps.splice(0)) rmSync(d, { recursive: true, force: true, maxRetries: 5 });
}

export type Of<K extends AppEvent["type"]> = Extract<AppEvent, { type: K }>;

export function setup(
  dataDir = mkdtempSync(path.join(tmpdir(), "kk-data-")),
  secrets: SecretBox = fakeSecretBox(),
  paths: VendorPaths = vendor,
  fingerprint?: (root: string) => Promise<string | null>,
) {
  temps.push(dataDir);
  const events: AppEvent[] = [];
  let listener = (_e: AppEvent) => {};
  const runner = new Runner({
    vendor: paths,
    dataDir,
    secrets,
    emit: (e) => {
      events.push(e);
      listener(e);
    },
    fingerprint,
  });
  runners.push(runner);
  const all = <K extends AppEvent["type"]>(type: K) =>
    events.filter((e): e is Of<K> => e.type === type);
  const last = <K extends AppEvent["type"]>(type: K) => all(type).at(-1)!;
  /** Calls `f` with each event as it is emitted, before the runner goes on. */
  const onEvent = (f: (e: AppEvent) => void) => (listener = f);
  return { runner, events, dataDir, all, last, onEvent };
}

export function fixture(
  name: "clean" | "broken" | "maven" | "edge" | "tags" | "env" | "history",
): string {
  const d = copyFixture(name);
  temps.push(d);
  return d;
}

export const printed = (events: AppEvent[]) =>
  events.flatMap((e) => (e.type === "console" ? e.lines : []));

export const leaves = (tree: TestNode | null) => (tree ? leafKeys(tree).length : -1);

export function byName(tree: TestNode, name: string): TestNode {
  const all = (n: TestNode): TestNode[] => [n, ...n.children.flatMap(all)];
  const hit = all(tree).find((n) => n.name === name);
  if (!hit) throw new Error(`no node named ${name}`);
  return hit;
}

export async function until(check: () => boolean, ms = 60_000): Promise<void> {
  const end = Date.now() + ms;
  while (!check()) {
    if (Date.now() > end) throw new Error("timed out waiting");
    await new Promise((r) => setTimeout(r, 50));
  }
}

export function hashTree(root: string): Map<string, string> {
  const out = new Map<string, string>();
  for (const e of readdirSync(root, { recursive: true, withFileTypes: true })) {
    if (!e.isFile()) continue;
    const full = path.join(e.parentPath, e.name);
    out.set(
      path.relative(root, full).split(path.sep).join("/"),
      createHash("sha1").update(readFileSync(full)).digest("hex"),
    );
  }
  return out;
}

/** `<dataDir>/projects/<hash>`, where the Runner keeps a project's runs and dry run. */
export const projectDir = (dataDir: string, root: string) =>
  path.join(dataDir, "projects", createHash("sha1").update(root).digest("hex").slice(0, 12));

/** The `-T` a Karate run was given, as its first SUITE_ENTER records it. */
export const suiteThreads = (file: string): number =>
  readAllEvents(file).find((e) => e.type === "SUITE_ENTER")!.data.threads;

/** Command lines of the running java.exe processes of the project at `root`. */
export const javaCommandLines = (root: string) =>
  execFileSync(
    "powershell",
    [
      "-NoProfile",
      "-NonInteractive",
      "-Command",
      "(Get-CimInstance Win32_Process -Filter \"Name='java.exe'\").CommandLine",
    ],
    { encoding: "utf8" },
  )
    .split(/\r?\n/)
    .filter((l) => l.includes(root));

/**
 * Makes the `env` fixture's `slow` scenario at `root` wait for the returned release instead of
 * sleeping 3 s, so a test can read the JVM's command line before the JVM exits.
 */
export function holdSlow(root: string): () => void {
  const gate = `${root}.release`; // outside the project, so no walk sees it
  temps.push(gate);
  const feature = path.join(root, "tests", "env.feature");
  const text = readFileSync(feature, "utf8");
  const held = text.replace(
    "java.lang.Thread.sleep(3000)",
    `eval while (!new java.io.File('${gate.replace(/\\/g, "/")}').exists()) java.lang.Thread.sleep(50)`,
  );
  if (held === text) throw new Error(`no slow step in ${feature}`);
  writeFileSync(feature, held);
  return () => writeFileSync(gate, "");
}

/** The run folder names of the project, oldest first. */
export const folders = (dataDir: string, root: string) => {
  try {
    return readdirSync(path.join(projectDir(dataDir, root), "runs")).sort();
  } catch {
    return [];
  }
};
export const recordFile = (dataDir: string, root: string, id: string) =>
  path.join(projectDir(dataDir, root), "runs", id, "run.json");
export const record = (dataDir: string, root: string, id: string): RunRecord =>
  JSON.parse(readFileSync(recordFile(dataDir, root, id), "utf8"));
/** Run All records, oldest first, with `over` applied to each. */
export const seed = (dataDir: string, root: string, count: number, over: Partial<RunRecord> = {}) =>
  Array.from({ length: count }, (_, i) => {
    const at = new Date(2020, 0, 1, 0, 0, i);
    const { id, dir } = createRunDir(projectDir(dataDir, root), at);
    writeRunRecord(dir, {
      version: 1,
      startedAt: at.getTime(),
      durationMillis: 1000,
      env: null,
      threads: 1,
      fingerprint: null,
      filter: NO_FILTER,
      selection: { kind: "folder", file: "", name: path.basename(root), line: null, key: "d:" },
      failedOf: null,
      outcome: "finished",
      exitCode: 0,
      tests: [],
      ...over,
    });
    return id;
  });
export const stored = (n: TestNode, status: StoredTest["status"]): StoredTest => ({
  kind: n.kind as StoredTest["kind"],
  file: n.file,
  refId: n.refId!,
  name: n.name,
  line: n.line,
  key: n.key,
  status,
});
