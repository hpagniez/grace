import { createHash } from "node:crypto";
import { existsSync, mkdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import path from "node:path";
import type { AppEvent, Project } from "../shared/events";
import type { TestNode } from "../shared/model";
import { buildArgs, detectLayout, selectionTargets } from "./karate/command";
import { JsonlTail, readAllEvents } from "./karate/events";
import { spawnKarate, type KarateProcess } from "./karate/process";
import { RunState } from "./karate/run-state";
import { buildTree, findNode, leafKeys, resolveNode } from "./karate/tree";
import type { VendorPaths } from "./vendor";

const BUSY = "A run is already in progress";

const isDir = (p: string) => {
  try {
    return statSync(p).isDirectory();
  } catch {
    return false;
  }
};

/** Karate's own words for a failed dry run: lines naming an exception or a feature, else the tail. */
function dryRunError(lines: string[]): string {
  const hits = lines.filter((l) => l.includes("Exception") || l.includes("feature"));
  return (hits.length ? hits : lines.slice(-20)).join("\n");
}

/** Owns the project, its test tree and the one Karate process (dry run or run) at a time. */
export class Runner {
  private project: Project | null = null;
  private tree: TestNode | null = null;
  private last: Extract<AppEvent, { type: "project" }> | null = null;
  private busy = false;
  private proc: KarateProcess | null = null;
  private cancelled = false;
  private report: string | null = null;
  private readonly pidFile: string;

  constructor(
    private readonly o: {
      vendor: VendorPaths;
      dataDir: string;
      emit: (e: AppEvent) => void;
      pollMs?: number;
    },
  ) {
    this.pidFile = path.join(o.dataDir, "running.pid");
  }

  async open(root: string): Promise<void> {
    await this.guarded(async () => {
      root = path.resolve(root);
      if (!isDir(root)) {
        this.project = this.tree = null;
        this.publish({
          type: "project",
          project: null,
          tree: null,
          error: `Folder not found: ${root}`,
        });
        return;
      }
      this.project = { root, layout: detectLayout(root).layout };
      await this.discover();
    });
  }

  async refresh(): Promise<void> {
    if (this.project) await this.guarded(() => this.discover());
  }

  async run(key: string): Promise<void> {
    if (this.project) await this.guarded(() => this.runGuarded(key));
  }

  async cancel(): Promise<void> {
    if (!this.proc) return;
    this.cancelled = true; // also stops a run whose pre-run dry run is going
    await this.proc.kill();
  }

  reportPath(): string | null {
    return this.report && existsSync(this.report) ? this.report : null;
  }

  current(): AppEvent | null {
    return this.last;
  }

  async dispose(): Promise<void> {
    this.cancelled = true;
    await this.proc?.kill();
    rmSync(this.pidFile, { force: true });
  }

  private publish(e: Extract<AppEvent, { type: "project" }>) {
    this.last = e;
    this.o.emit(e);
  }

  private async guarded(work: () => Promise<unknown>): Promise<void> {
    if (this.busy) {
      this.o.emit({ type: "notice", message: BUSY });
      return;
    }
    this.busy = true;
    this.cancelled = false;
    try {
      await work();
    } finally {
      this.busy = false;
    }
  }

  /** Empties and returns `<dataDir>/projects/<hash>/<kind>`, so no file of an earlier spawn is ever read. */
  private outDir(kind: "run" | "dryrun"): string {
    const hash = createHash("sha1").update(this.project!.root).digest("hex").slice(0, 12);
    const dir = path.join(this.o.dataDir, "projects", hash, kind);
    rmSync(dir, { recursive: true, force: true, maxRetries: 5 });
    mkdirSync(dir, { recursive: true });
    return dir;
  }

  private spawn(
    outDir: string,
    dryRun: boolean,
    targets: string[],
    onLines: (l: string[]) => void,
  ) {
    const { root } = this.project!;
    const { java, jar } = this.o.vendor;
    const { classpath } = detectLayout(root);
    const args = buildArgs({ jar, classpath, root, outDir, dryRun, targets });
    return (this.proc = spawnKarate(java, args, root, onLines));
  }

  /** Dry run on all feature roots; emits and returns the new tree, or null on error. */
  private async discover(): Promise<TestNode | null> {
    const project = this.project!;
    const { featureRoots } = detectLayout(project.root);
    const rootName = path.basename(project.root);
    let tree: TestNode | null = null;
    let error: string | null = null;
    if (featureRoots.length === 0) {
      tree = buildTree([], rootName); // Karate with no paths would load a pom or print help
    } else {
      const outDir = this.outDir("dryrun");
      const lines: string[] = [];
      const proc = this.spawn(outDir, true, featureRoots, (l) => lines.push(...l));
      const code = await proc.exit;
      this.proc = null;
      if (this.cancelled) return null; // killed on purpose: keep the last tree
      if (code !== 0) {
        error = dryRunError(lines);
      } else {
        try {
          tree = buildTree(
            readAllEvents(path.join(outDir, "karate-json", "karate-events.jsonl")),
            rootName,
          );
        } catch (err) {
          error = (err as Error).message;
        }
      }
    }
    this.tree = tree;
    this.publish({ type: "project", project, tree, error });
    return tree;
  }

  private async runGuarded(key: string): Promise<void> {
    const old = this.tree && findNode(this.tree, key);
    const tree = await this.discover();
    if (this.cancelled) {
      this.o.emit({ type: "notice", message: "Run cancelled" });
      return;
    }
    if (!tree) return;
    const node = old ? resolveNode(tree, old) : findNode(tree, key);
    if (!node) {
      this.o.emit({ type: "notice", message: "This test changed; the tree was refreshed" });
      return;
    }
    const { root } = this.project!;
    const targets = selectionTargets(node, root, detectLayout(root).featureRoots);
    if (leafKeys(node).length === 0 || targets.length === 0) {
      this.o.emit({ type: "notice", message: "Nothing to run" });
      return;
    }

    this.report = null;
    const outDir = this.outDir("run");
    const state = new RunState(tree, node);
    const tail = new JsonlTail(path.join(outDir, "karate-json", "karate-events.jsonl"));
    const proc = this.spawn(outDir, false, targets, (lines) =>
      this.o.emit({ type: "console", lines }),
    );
    if (proc.pid !== -1) writeFileSync(this.pidFile, String(proc.pid));
    this.o.emit({
      type: "runStarted",
      selection: node.key,
      keys: state.keys,
      total: state.keys.length,
      startedAt: Date.now(),
    });

    // A bad event line or malformed data is fatal: the run's status can no longer be trusted.
    let fatal = null as Error | null; // assigned in poll(); keeps TS from narrowing it to null
    const poll = () => {
      if (fatal) return;
      try {
        for (const e of state.apply(tail.read())) this.o.emit(e);
      } catch (err) {
        fatal = err as Error;
        void proc.kill();
      }
    };
    const timer = setInterval(poll, this.o.pollMs ?? 100);
    const code = await proc.exit;
    clearInterval(timer);
    poll();
    this.proc = null;
    rmSync(this.pidFile, { force: true });

    const report = path.join(outDir, "karate-summary.html");
    // exit code null forces outcome 'failed'
    for (let e of state.finish(fatal ? null : code, this.cancelled && !fatal, false)) {
      if (e.type === "runFinished") {
        this.report = e.outcome === "finished" && existsSync(report) ? report : null;
        e = { ...e, reportAvailable: this.report !== null };
      }
      this.o.emit(e);
    }
    if (fatal) this.o.emit({ type: "notice", message: fatal.message });
  }
}
