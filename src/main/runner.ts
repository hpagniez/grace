import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import path from "node:path";
import {
  editErrors,
  effectiveRows,
  propertyValueError,
  sortedUnique,
  validEnvName,
  type PlainRow,
  type PropertiesEdit,
  type RowEdit,
  type RowView,
} from "../shared/env";
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
import { writeArgFile } from "./karate/argfile";
import { buildArgs, detectLayout, selectionTargets, tagArgs } from "./karate/command";
import { featureTagEnvs, overlayEnvs } from "./karate/environments";
import { JsonlTail, readAllEvents } from "./karate/events";
import { spawnKarate, within, type KarateProcess } from "./karate/process";
import { RunState } from "./karate/run-state";
import { buildTree, findNode, parseRefId, resolveNode, rowLines } from "./karate/tree";
import { makeMasker } from "./mask";
import type { SecretBox } from "./secrets";
import {
  emptyEnvSettings,
  loadProjectEnv,
  loadProjectFilter,
  saveProjectEnv,
  saveProjectFilter,
  type EnvSettings,
  type StoredLists,
  type StoredRow,
} from "./settings";
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

/** The event with every secret value in its run text replaced; events of other types carry no run text. */
function masked(e: AppEvent, mask: (text: string) => string): AppEvent {
  switch (e.type) {
    case "console":
      return { ...e, lines: e.lines.map(mask) };
    case "scenarios":
      return {
        ...e,
        updates: e.updates.map((u) => (u.error === undefined ? u : { ...u, error: mask(u.error) })),
      };
    case "steps":
      return {
        ...e,
        steps: e.steps.map((s) => ({
          ...s,
          text: mask(s.text), // an outline row's step text holds its Examples values
          log: s.log === null ? null : mask(s.log),
        })),
      };
    case "notice":
      return { ...e, message: mask(e.message) }; // an invalid event line can be quoted in it
    default:
      return e;
  }
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

/**
 * Owns the project, its test tree, its tag filter, its environment and run properties, and the one
 * Karate process (dry run or run) at a time.
 */
export class Runner {
  private project: Project | null = null;
  private tree: TestNode | null = null;
  private last: Extract<AppEvent, { type: "project" }> | null = null;
  private filter: FilterState = { ...NO_FILTER, include: [], exclude: [] };
  private envs: EnvSettings = emptyEnvSettings();
  /** Environments of overlay files and env tags, scanned at each discovery of open, refresh and setEnv. */
  private overlays: string[] = [];
  private tagEnvs: string[] = [];
  private busy = false;
  private proc: KarateProcess | null = null;
  private cancelled = false;
  private report: string | null = null;
  private readonly pidFile: string;

  constructor(
    private readonly o: {
      vendor: VendorPaths;
      dataDir: string;
      secrets: SecretBox;
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
        loadProjectEnv(this.o.dataDir, root),
      );
      if (!opened) this.report = report;
    });
    return opened;
  }

  async refresh(): Promise<void> {
    if (this.project)
      await this.guarded(() => this.rediscover(this.project!, this.filter, this.envs));
  }

  /**
   * Re-discovers with the new environment (env tags change which tests exist). It is saved once that
   * discovery ends, also on a parse error; a cancel keeps the environment before and saves nothing.
   * Refused while busy or for an invalid name (the environment in force is re-sent).
   */
  async setEnv(name: string | null): Promise<void> {
    if (!this.project) return;
    const invalid = name !== null && !validEnvName(name);
    if (this.busy || invalid) {
      this.o.emit({ type: "notice", message: this.busy ? BUSY : "Invalid environment name" });
      this.o.emit(this.envEvent());
      return;
    }
    if (name === this.envs.env) return;
    await this.guarded(async () => {
      const envs = { ...this.envs, env: name };
      if (!(await this.rediscover(this.project!, this.filter, envs))) return;
      this.report = null; // a result from another environment
      try {
        saveProjectEnv(this.o.dataDir, this.project!.root, envs);
      } catch (err) {
        this.o.emit({
          type: "notice",
          message: `The environment could not be saved: ${(err as Error).message}`,
        });
      }
    });
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

  /**
   * Replaces the run properties with a validated edit; a `keep` row keeps the ciphertext of the saved
   * secret of the same scope and name. Applied even if it cannot be saved, as the filter is.
   */
  saveProperties(edit: PropertiesEdit): void {
    if (this.busy) {
      this.o.emit({ type: "notice", message: BUSY });
      return;
    }
    if (!this.project) return;
    const { secrets } = this.o;
    const { shared, byEnv } = this.envs.properties;
    const cipher = (scope: string | null, name: string): string | null => {
      const rows = scope === null ? shared : Object.hasOwn(byEnv, scope) ? byEnv[scope] : [];
      const row = rows.find((r) => r.name === name);
      return row && "secret" in row ? row.secret : null;
    };
    const error = editErrors(edit, (scope, name) => cipher(scope, name) !== null)[0];
    if (error) {
      this.o.emit({ type: "notice", message: `Properties not saved: ${error}` });
      return;
    }
    const all = [edit.shared, ...Object.values(edit.byEnv)].flat();
    if (!secrets.available() && all.some((r) => r.secret && !("keep" in r))) {
      this.o.emit({ type: "notice", message: "Secret storage is not available on this system" });
      return;
    }
    const stored = (scope: string | null, rows: RowEdit[]): StoredRow[] =>
      rows.map((r) =>
        "keep" in r
          ? { name: r.name, secret: cipher(scope, r.name)! }
          : r.secret
            ? { name: r.name, secret: secrets.encrypt(r.value) }
            : { name: r.name, value: r.value },
      );
    const properties: StoredLists = {
      shared: stored(null, edit.shared),
      byEnv: Object.fromEntries(
        Object.entries(edit.byEnv).map(([e, rows]) => [e, stored(e, rows)]),
      ),
    };
    this.envs = { ...this.envs, properties };
    try {
      saveProjectEnv(this.o.dataDir, this.project.root, this.envs);
    } catch (err) {
      this.o.emit({
        type: "notice",
        message: `The properties could not be saved: ${(err as Error).message}`,
      });
    }
    this.o.emit(this.envEvent());
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
    return this.last
      ? [this.last, { type: "filter", filter: this.filter }, this.envEvent(), busy]
      : [busy];
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

  /** The plain value of a saved secret; null if it cannot be decrypted or Java cannot take it exactly. */
  private readSecret(cipher: string): string | null {
    const { secrets } = this.o;
    const value = secrets.available() ? secrets.decrypt(cipher) : null;
    return value !== null && propertyValueError(value) === null ? value : null;
  }

  /** A secret row carries only whether it can be read, never its value. */
  private envEvent(): AppEvent {
    const { secrets } = this.o;
    const { env, properties } = this.envs;
    const row = (r: StoredRow): RowView =>
      "secret" in r
        ? {
            name: r.name,
            secret: true,
            state: this.readSecret(r.secret) !== null ? "set" : "unreadable",
          }
        : r;
    return {
      type: "environment",
      view: {
        env,
        suggestions: sortedUnique([
          this.overlays,
          this.tagEnvs,
          Object.keys(properties.byEnv),
          env === null ? [] : [env],
        ]),
        overlays: this.overlays,
        properties: {
          shared: properties.shared.map(row),
          byEnv: Object.fromEntries(
            Object.entries(properties.byEnv).map(([e, rows]) => [e, rows.map(row)]),
          ),
        },
        secretsAvailable: secrets.available(),
      },
    };
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
   * Discovery of open, refresh and setEnv with `envs`, then `filter` without the tags the new tree
   * lacks (a failed dry run drops nothing), then `environment`. `envs` replaces the environment only
   * once the discovery succeeded, so nothing shows or uses it before. Cancel puts the project and tree
   * from before back and re-sends them, the filter, which it leaves unchanged, and the environment.
   */
  private async rediscover(
    project: Project,
    filter: FilterState,
    envs: EnvSettings,
  ): Promise<boolean> {
    const before = { project: this.project, tree: this.tree };
    this.project = project;
    let tree: TestNode | null;
    try {
      tree = await this.discover(envs.env);
    } catch (err) {
      ({ project: this.project, tree: this.tree } = before);
      this.o.emit(this.envEvent()); // the window may still show the name it sent
      throw err;
    }
    if (!this.cancelled) {
      this.envs = envs;
      this.filter = filter;
      const pruned = tree && pruneFilter(filter, projectTags(tree));
      if (
        pruned &&
        (pruned.include.length !== filter.include.length ||
          pruned.exclude.length !== filter.exclude.length)
      )
        this.storeFilter(pruned);
      this.sendFilter();
      const { classpath, featureRoots } = detectLayout(project.root);
      this.overlays = overlayEnvs([...new Set([project.root, ...classpath])]);
      this.tagEnvs = featureTagEnvs(featureRoots); // from the files: the tree lacks other envs' tests
      this.o.emit(this.envEvent());
      return true;
    }
    ({ project: this.project, tree: this.tree } = before);
    this.o.emit(this.last ?? { type: "project", project: null, tree: null, error: null });
    this.sendFilter();
    this.o.emit(this.envEvent());
    this.o.emit({ type: "notice", message: "Cancelled" });
    return false;
  }

  /** `<dataDir>/projects/<hash>`: the run folders and the argument file of the project. */
  private projectDir(): string {
    const hash = createHash("sha1").update(this.project!.root).digest("hex").slice(0, 12);
    return path.join(this.o.dataDir, "projects", hash);
  }

  /** Empties and returns `<dataDir>/projects/<hash>/<kind>`, so no file of an earlier spawn is ever read. */
  private outDir(kind: "run" | "dryrun"): string {
    const dir = path.join(this.projectDir(), kind);
    rmSync(dir, { recursive: true, force: true, maxRetries: 5 });
    mkdirSync(dir, { recursive: true });
    return dir;
  }

  private spawn(
    outDir: string,
    dryRun: boolean,
    targets: string[],
    env: string | null,
    onLines: (l: string[]) => void,
    argFile: string | null = null,
  ) {
    const { root } = this.project!;
    const { java, jar } = this.o.vendor;
    const { classpath } = detectLayout(root);
    // discovery lists every test; a run gets the filter
    const tags = dryRun ? [] : tagArgs(this.filter);
    const args = buildArgs({
      jar,
      classpath,
      root,
      outDir,
      dryRun,
      targets,
      tags,
      env,
      argFile,
    });
    return (this.proc = spawnKarate(java, args, root, onLines));
  }

  /** Dry run on all feature roots with `env`; emits and returns the new tree, or null on error. */
  private async discover(env: string | null): Promise<TestNode | null> {
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
      const proc = this.spawn(outDir, true, featureRoots, env, (l) => lines.push(...l));
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

  /**
   * The run's properties (R4) with secrets decrypted, and the secret values to mask; null, after a
   * notice, if a secret cannot be read.
   */
  private runProperties(): { rows: PlainRow[]; secrets: string[] } | null {
    const { env, properties } = this.envs;
    const rows: PlainRow[] = [];
    const secrets: string[] = [];
    for (const r of effectiveRows(properties.shared, properties.byEnv, env)) {
      if (!("secret" in r)) {
        rows.push(r);
        continue;
      }
      const value = this.readSecret(r.secret);
      if (value === null) {
        const scope = properties.shared.includes(r) ? "All environments" : env;
        this.o.emit({
          type: "notice",
          message: `Secret ${r.name} (${scope}) can't be read. Replace it in Properties.`,
        });
        return null;
      }
      rows.push({ name: r.name, value });
      secrets.push(value);
    }
    return { rows, secrets };
  }

  private async runGuarded(key: string): Promise<void> {
    // The rows depend on the environment and the saved lists only: refuse before any JVM starts.
    const props = this.runProperties();
    if (!props) return;
    const old = this.tree && findNode(this.tree, key);
    const tree = await this.discover(this.envs.env);
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
    const outDir = this.outDir("run"); // creates the project folder the argument file goes in
    const argFile = props.rows.length ? path.join(this.projectDir(), "run-args.txt") : null;
    const mask = makeMasker(props.secrets);
    const emit = (e: AppEvent) => this.o.emit(masked(e, mask));
    try {
      if (argFile) writeArgFile(argFile, props.rows);
      const state = new RunState(tree, node, filter);
      const tail = new JsonlTail(path.join(outDir, "karate-json", "karate-events.jsonl"));
      const proc = this.spawn(
        outDir,
        false,
        targets,
        this.envs.env,
        (lines) => emit({ type: "console", lines }),
        argFile,
      );
      try {
        if (proc.pid !== -1) writeFileSync(this.pidFile, String(proc.pid));
      } catch (err) {
        // without the pid file a crash of the app would leave this JVM running: do not run
        await proc.kill();
        this.proc = null;
        for (const e of state.finish(null, false, false)) emit(e);
        emit({ type: "notice", message: (err as Error).message });
        return;
      }
      emit({
        type: "runStarted",
        selection: node.key,
        keys: state.keys,
        total: state.keys.length,
        startedAt: Date.now(),
        env: this.envs.env,
      });

      // A bad event line or malformed data is fatal: the run's status can no longer be trusted.
      let fatal = null as Error | null; // assigned in poll(); keeps TS from narrowing it to null
      const poll = () => {
        if (fatal) return;
        try {
          for (const e of state.apply(tail.read())) emit(e);
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
        emit(e);
      }
      if (fatal) emit({ type: "notice", message: fatal.message });
    } finally {
      // An antivirus or an indexer can hold it a moment (Electron's Node 24.22 retries a file; Node
      // 24.15 does not, so the unit tests cover only the notice). A failure must not replace the run's outcome.
      try {
        if (argFile) rmSync(argFile, { force: true, maxRetries: 5 });
      } catch (err) {
        emit({
          type: "notice",
          message: `The run's argument file could not be deleted; the app deletes it when it next starts. ${(err as Error).message}`,
        });
      }
    }
  }
}
