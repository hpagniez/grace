import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import path from "node:path";
import type { AppEvent, Project } from "../shared/events";
import type { TestNode } from "../shared/model";
import {
  matches,
  matchingLeaves,
  NO_FILTER,
  projectTags,
  pruneFilter,
  type FilterState,
} from "../shared/tags";
import { buildArgs, detectLayout, selectionTargets, tagArgs } from "./karate/command";
import { JsonlTail, readAllEvents } from "./karate/events";
import { spawnKarate, within, type KarateProcess } from "./karate/process";
import { RunState } from "./karate/run-state";
import { buildTree, findNode, parseRefId, resolveNode, rowLines } from "./karate/tree";
import { loadProjectFilter, saveProjectFilter } from "./settings";
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

/**
 * Sets each row's line from its feature file; null when the table has no such row (dynamic
 * Examples), since running a guessed line could run another row or nothing.
 */
function setRowLines(tree: TestNode, root: string): void {
  const texts = new Map<string, string>();
  const walk = (n: TestNode) => {
    if (n.kind === "row") {
      const { example, line } = parseRefId(n.refId!);
      if (!texts.has(n.file)) texts.set(n.file, readFileSync(path.join(root, n.file), "utf8"));
      n.line = rowLines(texts.get(n.file)!, line)[example! - 1] ?? null;
    }
    n.children.forEach(walk);
  };
  walk(tree);
}

/** Owns the project, its test tree, its tag filter and the one Karate process (dry run or run) at a time. */
export class Runner {
  private project: Project | null = null;
  private tree: TestNode | null = null;
  private last: Extract<AppEvent, { type: "project" }> | null = null;
  private filter: FilterState = { ...NO_FILTER, include: [], exclude: [] };
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

  /** True only if the folder is now the open project. */
  async open(root: string): Promise<boolean> {
    let opened = false;
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
      const report = this.report;
      this.report = null; // a report of the project open before
      opened = await this.rediscover(
        { root, layout: detectLayout(root).layout },
        loadProjectFilter(this.o.dataDir, root),
      );
      if (!opened) this.report = report;
    });
    return opened;
  }

  async refresh(): Promise<void> {
    if (this.project) await this.guarded(() => this.rediscover(this.project!, this.filter));
  }

  /** Refused while busy (the filter in force is re-sent); ignored with no project. */
  setFilter(f: FilterState): void {
    if (this.busy) {
      this.o.emit({ type: "notice", message: BUSY });
      this.sendFilter();
      return;
    }
    if (!this.project) return;
    this.storeFilter({ ...f, include: [...f.include], exclude: [...f.exclude] });
    this.sendFilter();
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

  /** What a window that just loaded needs: events sent before it subscribed are lost. */
  replay(): AppEvent[] {
    const busy: AppEvent = { type: "busy", busy: this.busy };
    return this.last ? [this.last, { type: "filter", filter: this.filter }, busy] : [busy];
  }

  /** Bounded to 10 s: the app quits after it whatever happens. */
  async dispose(): Promise<void> {
    this.cancelled = true;
    if (this.proc) await within(this.proc.kill(), 10_000);
    rmSync(this.pidFile, { force: true });
  }

  private publish(e: Extract<AppEvent, { type: "project" }>) {
    this.last = e;
    this.o.emit(e);
  }

  private sendFilter() {
    this.o.emit({ type: "filter", filter: this.filter });
  }

  /** Applies `f` even if it cannot be saved: a disk error must not undo the change. */
  private storeFilter(f: FilterState) {
    this.filter = f;
    try {
      saveProjectFilter(this.o.dataDir, this.project!.root, f);
    } catch (err) {
      this.o.emit({
        type: "notice",
        message: `The filter could not be saved: ${(err as Error).message}`,
      });
    }
  }

  private async guarded(work: () => Promise<unknown>): Promise<void> {
    if (this.busy) {
      this.o.emit({ type: "notice", message: BUSY });
      return;
    }
    this.busy = true;
    this.cancelled = false;
    this.o.emit({ type: "busy", busy: true });
    try {
      await work();
    } finally {
      this.busy = false;
      this.o.emit({ type: "busy", busy: false });
    }
  }

  /**
   * Discovery of open and refresh, then `filter` without the tags the new tree lacks (a failed dry
   * run drops nothing). Cancel puts the project and tree from before back and re-sends them and
   * the filter, which it leaves unchanged.
   */
  private async rediscover(project: Project, filter: FilterState): Promise<boolean> {
    const before = { project: this.project, tree: this.tree };
    this.project = project;
    const tree = await this.discover();
    if (!this.cancelled) {
      this.filter = filter;
      const pruned = tree && pruneFilter(filter, projectTags(tree));
      if (
        pruned &&
        (pruned.include.length !== filter.include.length ||
          pruned.exclude.length !== filter.exclude.length)
      )
        this.storeFilter(pruned);
      this.sendFilter();
      return true;
    }
    ({ project: this.project, tree: this.tree } = before);
    this.o.emit(this.last ?? { type: "project", project: null, tree: null, error: null });
    this.sendFilter();
    this.o.emit({ type: "notice", message: "Cancelled" });
    return false;
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
    // discovery lists every test; a run gets the filter
    const tags = dryRun ? [] : tagArgs(this.filter);
    const args = buildArgs({ jar, classpath, root, outDir, dryRun, targets, tags });
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
          setRowLines(tree, project.root);
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
    if (node.kind === "row" && node.line === null) {
      this.o.emit({ type: "notice", message: "Run the outline to run its rows" });
      return;
    }
    // The filter as saved, not pruned: a vanished included tag shrinks the run, never widens it.
    const filter = this.filter;
    if ((node.kind === "scenario" || node.kind === "row") && !matches(node.tags, filter)) {
      this.o.emit({ type: "notice", message: "This test no longer matches the filter" });
      return;
    }
    if (matchingLeaves(node, filter).length === 0) {
      this.o.emit({ type: "notice", message: "Nothing to run" });
      return;
    }
    const { root } = this.project!;
    const targets = selectionTargets(node, root, detectLayout(root).featureRoots, filter);
    if (!targets) {
      // a matching row of a partly matching outline has no line (dynamic Examples)
      this.o.emit({ type: "notice", message: "Clear the filter to run this outline" });
      return;
    }

    this.report = null;
    const outDir = this.outDir("run");
    const state = new RunState(tree, node, filter);
    const tail = new JsonlTail(path.join(outDir, "karate-json", "karate-events.jsonl"));
    const proc = this.spawn(outDir, false, targets, (lines) =>
      this.o.emit({ type: "console", lines }),
    );
    try {
      if (proc.pid !== -1) writeFileSync(this.pidFile, String(proc.pid));
    } catch (err) {
      // without the pid file a crash of the app would leave this JVM running: do not run
      await proc.kill();
      this.proc = null;
      for (const e of state.finish(null, false, false)) this.o.emit(e);
      this.o.emit({ type: "notice", message: (err as Error).message });
      return;
    }
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
