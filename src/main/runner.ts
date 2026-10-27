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
import type { RunRecord, StoredTest } from "../shared/history";
import type { TestNode } from "../shared/model";
import {
  matches,
  matchingLeaves,
  NO_FILTER,
  projectTags,
  pruneFilter,
  type FilterState,
} from "../shared/tags";
import { THREADS_ERROR, validThreads } from "../shared/threads";
import { projectFingerprint } from "./fingerprint";
import {
  clearHistory as removeRuns,
  createRunDir,
  listHistory,
  pruneHistory,
  readRunRecord,
  writeRunRecord,
} from "./history";
import { writeArgFile, writeTargetFile } from "./karate/argfile";
import { buildArgs, detectLayout, lineTargets, selectionTargets, tagArgs } from "./karate/command";
import { featureTagEnvs, overlayEnvs } from "./karate/environments";
import { JsonlTail, readAllEvents } from "./karate/events";
import { spawnKarate, within, type KarateProcess } from "./karate/process";
import { RunState } from "./karate/run-state";
import {
  buildTree,
  findNode,
  parseRefId,
  resolveNode,
  resolveTests,
  rowLines,
} from "./karate/tree";
import { makeMasker, maskEvent } from "./mask";
import type { SecretBox } from "./secrets";
import {
  emptyEnvSettings,
  loadProjectEnv,
  loadProjectFilter,
  loadProjectThreads,
  saveProjectEnv,
  saveProjectFilter,
  saveProjectThreads,
  skippedProperties,
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
 * What a run selects: a tree node (a click or a Re-run), found again after the pre-run discovery or
 * refused with `gone`; or stored tests (Re-run failed, or a Re-run of one).
 */
type RunTarget =
  | { key: string; old: Pick<TestNode, "key" | "kind" | "file" | "name"> | null; gone: string }
  | { tests: StoredTest[]; failedOf: number | null };

/** The tests of run.json: each selected leaf with the final status and duration its run gave it. */
function storedTests(leaves: TestNode[], results: ReturnType<RunState["results"]>): StoredTest[] {
  return leaves.map((n) => ({
    kind: n.kind as StoredTest["kind"],
    file: n.file,
    refId: n.refId!,
    name: n.name,
    line: n.line,
    key: n.key,
    ...results.get(n.key)!,
  }));
}

/**
 * Owns the project, its test tree, its tag filter, its thread count, its environment and run
 * properties, and the one Karate process (dry run or run) at a time.
 */
export class Runner {
  private project: Project | null = null;
  private tree: TestNode | null = null;
  private last: Extract<AppEvent, { type: "project" }> | null = null;
  private filter: FilterState = { ...NO_FILTER, include: [], exclude: [] };
  private threads = 1;
  private envs: EnvSettings = emptyEnvSettings();
  /** Environments of overlay files and env tags, scanned at each discovery of open, refresh and setEnv. */
  private overlays: string[] = [];
  private tagEnvs: string[] = [];
  private busy = false;
  /** The operation `guarded` is running, for `dispose` to wait on. */
  private running: Promise<unknown> = Promise.resolve();
  private proc: KarateProcess | null = null;
  private cancelled = false;
  /** Set by `dispose` alone (`cancel` leaves it): the app is quitting, so a run skips its second walk. */
  private disposing = false;
  private report: string | null = null;
  private readonly pidFile: string;

  constructor(
    private readonly o: {
      vendor: VendorPaths;
      dataDir: string;
      secrets: SecretBox;
      emit: (e: AppEvent) => void;
      pollMs?: number;
      /** A test seam; the default is `projectFingerprint`. */
      fingerprint?: (root: string) => Promise<string | null>;
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
      const skipped = skippedProperties(this.o.dataDir, root);
      opened = await this.rediscover(
        { root, layout: detectLayout(root).layout },
        loadProjectFilter(this.o.dataDir, root),
        loadProjectEnv(this.o.dataDir, root),
        loadProjectThreads(this.o.dataDir, root),
      );
      if (!opened) this.report = report;
      else if (skipped.length)
        this.o.emit({
          type: "notice",
          message: `Some saved properties were not loaded and are kept in settings.json: ${skipped.join(", ")}`,
        });
    });
    return opened;
  }

  async refresh(): Promise<void> {
    if (this.project)
      await this.guarded(() =>
        this.rediscover(this.project!, this.filter, this.envs, this.threads),
      );
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
    await this.guarded(() => this.switchEnv(name, this.filter));
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
   * Refused while busy or for anything but a whole number from 1 to 32 (the count in force is
   * re-sent); ignored with no project. Applied even if it cannot be saved, as the filter is.
   */
  setThreads(n: unknown): void {
    if (this.busy) {
      this.o.emit({ type: "notice", message: BUSY });
      this.sendThreads();
      return;
    }
    if (!this.project) return;
    if (!validThreads(n)) {
      this.o.emit({ type: "notice", message: THREADS_ERROR });
      this.sendThreads();
      return;
    }
    this.threads = n;
    try {
      saveProjectThreads(this.o.dataDir, this.project.root, n);
    } catch (err) {
      this.o.emit({
        type: "notice",
        message: `The thread count could not be saved: ${(err as Error).message}`,
      });
    }
    this.sendThreads();
  }

  /**
   * Replaces the run properties with a validated edit; a `keep` row keeps the ciphertext of the saved
   * secret of the same scope and name. Applied even if it cannot be saved, as the filter is. True
   * once applied; false, after a notice, if refused, so the window keeps its draft.
   */
  saveProperties(edit: PropertiesEdit): boolean {
    if (this.busy) {
      this.o.emit({ type: "notice", message: BUSY });
      return false;
    }
    if (!this.project) return false;
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
      return false;
    }
    const all = [edit.shared, ...Object.values(edit.byEnv)].flat();
    if (!secrets.available() && all.some((r) => r.secret && !("keep" in r))) {
      this.o.emit({ type: "notice", message: "Secret storage is not available on this system" });
      return false;
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
    return true;
  }

  async run(key: string): Promise<void> {
    if (this.project)
      await this.guarded(() =>
        this.runTarget(
          {
            key,
            old: this.tree && findNode(this.tree, key),
            gone: "This test changed; the tree was refreshed",
          },
          this.filter,
        ),
      );
  }

  /** Re-run (R5): the run's selection on its environment and filter; a Re-run failed run's tests, all of them. */
  async rerun(id: string): Promise<void> {
    if (this.project) await this.guarded(() => this.rerunGuarded(id, false));
  }

  /** Re-run failed (R6): the run's failed tests on its environment and filter. */
  async rerunFailed(id: string): Promise<void> {
    if (this.project) await this.guarded(() => this.rerunGuarded(id, true));
  }

  /** Deletes the open project's run folders (R7); the tree keeps its statuses. Refused while busy. */
  clearHistory(): void {
    if (this.busy) {
      this.o.emit({ type: "notice", message: BUSY });
      return;
    }
    if (!this.project) return;
    // the toolbar Report stays: reportPath() checks the file, which a held folder can keep
    if (!removeRuns(this.projectDir()))
      this.o.emit({ type: "notice", message: "Some runs could not be deleted" });
    this.o.emit(this.historyEvent());
  }

  /** The report of a listed run of the open project (R4); null if it has none. Never takes a path. */
  runReportPath(id: string): string | null {
    if (!this.project || !readRunRecord(this.projectDir(), id)) return null; // also checks the id
    const report = path.join(this.projectDir(), "runs", id, "karate-summary.html");
    return existsSync(report) ? report : null;
  }

  async cancel(): Promise<void> {
    if (!this.busy) return;
    // also between two JVMs of one operation: each start checks it (a re-run's switch, then its run)
    this.cancelled = true;
    await this.proc?.kill();
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
      ? [
          this.last,
          { type: "filter", filter: this.filter },
          { type: "threads", count: this.threads },
          this.envEvent(),
          this.historyEvent(),
          busy,
        ]
      : [busy];
  }

  /** Bounded to 10 s: the app quits after it whatever happens. */
  async dispose(): Promise<void> {
    this.disposing = true;
    this.cancelled = true;
    // The kill ends the run's process; the run then writes its record before `running` settles.
    await within(Promise.allSettled([this.proc?.kill(), this.running]), 10_000);
    rmSync(this.pidFile, { force: true });
  }

  private publish(e: Extract<AppEvent, { type: "project" }>) {
    this.last = e;
    this.o.emit(e);
  }

  private sendFilter() {
    this.o.emit({ type: "filter", filter: this.filter });
  }

  private sendThreads() {
    this.o.emit({ type: "threads", count: this.threads });
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

  /** The open project's runs, newest first; none with no project. */
  private historyEvent(): AppEvent {
    return { type: "history", runs: this.project ? listHistory(this.projectDir()) : [] };
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
      this.running = work(); // inside the try: a synchronous throw must not leave busy set
      await this.running;
    } finally {
      this.busy = false;
      this.o.emit({ type: "busy", busy: false });
    }
  }

  /**
   * Discovery of open, refresh and setEnv with `envs`, then `filter` without the tags the new tree
   * lacks (a failed dry run drops nothing), then `threads`, then `environment`. `envs` and `threads`
   * replace those in force only once the discovery succeeded, so nothing shows or uses them before.
   * Cancel puts the project and tree from before back and re-sends them, the filter and the thread
   * count, which it leaves unchanged, and the environment.
   */
  private async rediscover(
    project: Project,
    filter: FilterState,
    envs: EnvSettings,
    threads: number,
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
      this.threads = threads;
      const pruned = tree && pruneFilter(filter, projectTags(tree));
      if (
        pruned &&
        (pruned.include.length !== filter.include.length ||
          pruned.exclude.length !== filter.exclude.length)
      )
        this.storeFilter(pruned);
      this.sendFilter();
      this.sendThreads();
      const { classpath, featureRoots } = detectLayout(project.root);
      this.overlays = overlayEnvs([...new Set([project.root, ...classpath])]);
      this.tagEnvs = await featureTagEnvs(featureRoots); // from the files: the tree lacks other envs' tests
      this.o.emit(this.envEvent());
      this.o.emit(this.historyEvent());
      return true;
    }
    ({ project: this.project, tree: this.tree } = before);
    this.o.emit(this.last ?? { type: "project", project: null, tree: null, error: null });
    this.sendFilter();
    this.sendThreads();
    this.o.emit(this.envEvent());
    this.o.emit(this.historyEvent());
    this.o.emit({ type: "notice", message: "Cancelled" });
    return false;
  }

  /**
   * Environment switcher R1: discovery with `env` and `filter`, then the environment saved, also after a
   * parse error. False if cancelled, which keeps the environment before and saves nothing.
   */
  private async switchEnv(env: string | null, filter: FilterState): Promise<boolean> {
    const envs = { ...this.envs, env };
    if (!(await this.rediscover(this.project!, filter, envs, this.threads))) return false;
    this.report = null; // a result from another environment
    try {
      saveProjectEnv(this.o.dataDir, this.project!.root, envs);
    } catch (err) {
      this.o.emit({
        type: "notice",
        message: `The environment could not be saved: ${(err as Error).message}`,
      });
    }
    return true;
  }

  /** `<dataDir>/projects/<hash>`: the run folders, the dry run folder and the argument and targets files of the project. */
  private projectDir(): string {
    const hash = createHash("sha1").update(this.project!.root).digest("hex").slice(0, 12);
    return path.join(this.o.dataDir, "projects", hash);
  }

  /** Empties and returns `<dataDir>/projects/<hash>/dryrun`, so no file of an earlier dry run is ever read. */
  private dryRunDir(): string {
    const dir = path.join(this.projectDir(), "dryrun");
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
    argFile: string | null,
    filter: FilterState, // required: a run that forgot it would run unfiltered
    threads: number, // required: a run that forgot it would run on one thread
  ) {
    const { root } = this.project!;
    const { java, jar } = this.o.vendor;
    const { classpath } = detectLayout(root);
    // discovery lists every test; a run gets the filter
    const tags = dryRun ? [] : tagArgs(filter);
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
      threads,
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
      const outDir = this.dryRunDir();
      const lines: string[] = [];
      const proc = this.spawn(
        outDir,
        true,
        featureRoots,
        env,
        (l) => lines.push(...l),
        null,
        NO_FILTER,
        this.threads, // buildArgs passes 1 on a dry run
      );
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
   * The properties (R4) of a run on `env` with secrets decrypted, and the secret values to mask; null,
   * after a notice, if a secret cannot be read.
   */
  private runProperties(env: string | null): { rows: PlainRow[]; secrets: string[] } | null {
    const { properties } = this.envs;
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

  /**
   * Re-run and Re-run failed (R5, R6): the record's secrets checked, its environment switched to, its
   * filter shown and saved, then its selection or tests run with the filter as written.
   */
  private async rerunGuarded(id: string, failedOnly: boolean): Promise<void> {
    const record = readRunRecord(this.projectDir(), id);
    if (!record) {
      this.o.emit({ type: "notice", message: "This run is no longer in the history" });
      return;
    }
    if (!this.runProperties(record.env)) return; // before the switch: nothing changes
    const switching = record.env !== this.envs.env;
    if (switching && !(await this.switchEnv(record.env, record.filter))) return; // cancelled
    // As if chosen in the filter bar, without vanished tags (tag filter R5).
    this.storeFilter(
      this.tree ? pruneFilter(record.filter, projectTags(this.tree)) : record.filter,
    );
    this.sendFilter();
    if (switching && !this.tree) return; // a parse error: the environment is switched, nothing runs
    // A cancel after the switch's dry run stops runTarget before its JVM, once the filter shown is saved.
    const { selection, tests, failedOf, startedAt } = record;
    await this.runTarget(
      failedOnly
        ? { tests: tests.filter((t) => t.status === "failed"), failedOf: startedAt }
        : selection
          ? { key: selection.key, old: selection, gone: "The tests of this run no longer exist" }
          : { tests, failedOf },
      record.filter,
    );
  }

  /**
   * The pre-run checks of a target after the pre-run discovery, then its run. `filter` is the one
   * saved, not pruned: a vanished included tag shrinks the run, never widens it.
   */
  private async runTarget(target: RunTarget, filter: FilterState): Promise<void> {
    const runCancelled = () => {
      if (this.cancelled) this.o.emit({ type: "notice", message: "Run cancelled" });
      return this.cancelled;
    };
    // a cancel or dispose before the pre-run discovery, e.g. after a re-run's switch: start no JVM
    if (runCancelled()) return;
    const env = this.envs.env;
    // The rows depend on the environment and the saved lists only: refuse before any JVM starts.
    const props = this.runProperties(env);
    if (!props) return;
    const tree = await this.discover(env);
    if (runCancelled()) return;
    if (!tree) return;
    const notice = (message: string) => this.o.emit({ type: "notice", message });
    const { root } = this.project!;
    const run = { tree, filter, threads: this.threads, env, props };

    if ("tests" in target) {
      // file:LINE targets ignore -t: the found tests run whatever their tags are now (R6)
      const { found, missing, dynamic } = resolveTests(tree, target.tests);
      const said = [
        missing === 1 ? "1 failed test no longer exists" : "",
        missing > 1 ? `${missing} failed tests no longer exist` : "",
        dynamic
          ? `${dynamic} failed ${dynamic === 1 ? "row" : "rows"} of a dynamic Examples table can't be run alone. Run its outline.`
          : "",
        found.length ? "" : "Nothing to run",
      ].filter(Boolean);
      // one notice: the window shows only the last
      if (said.length)
        notice(said.map((m) => (said.length > 1 && !m.endsWith(".") ? `${m}.` : m)).join(" "));
      if (!found.length) return;
      await this.runBody({
        ...run,
        leaves: found,
        targets: lineTargets(found, root),
        targetFile: true,
        selection: null,
        failedOf: target.failedOf,
      });
      return;
    }

    const node = target.old ? resolveNode(tree, target.old) : findNode(tree, target.key);
    if (!node) {
      notice(target.gone);
      return;
    }
    if (node.kind === "row" && node.line === null) {
      notice("Run the outline to run its rows");
      return;
    }
    if ((node.kind === "scenario" || node.kind === "row") && !matches(node.tags, filter)) {
      notice("This test no longer matches the filter");
      return;
    }
    const leaves = matchingLeaves(node, filter);
    if (leaves.length === 0) {
      notice("Nothing to run");
      return;
    }
    const targets = selectionTargets(node, root, detectLayout(root).featureRoots, filter);
    if (!targets) {
      // a matching row of a partly matching outline has no line (dynamic Examples)
      notice("Clear the filter to run this outline");
      return;
    }
    await this.runBody({
      ...run,
      leaves,
      targets,
      targetFile: false,
      selection: node,
      failedOf: null,
    });
  }

  /**
   * Runs `leaves` with `targets`, passed as `@<file>` when `targetFile` (any number fits), and stores
   * the run in its folder. The argument and targets files are deleted whatever happens.
   */
  private async runBody(o: {
    tree: TestNode;
    leaves: TestNode[];
    targets: string[];
    targetFile: boolean;
    filter: FilterState;
    threads: number;
    env: string | null;
    props: { rows: PlainRow[]; secrets: string[] };
    selection: TestNode | null;
    failedOf: number | null;
  }): Promise<void> {
    const { tree, leaves, filter, threads, env, props, selection, failedOf } = o;
    const { root } = this.project!;
    const walk = this.o.fingerprint ?? projectFingerprint;
    // A file saved while Java runs could be what made a test flip: the run keeps its fingerprint
    // only if the walk after the run finds what this one does (F15).
    const before = await walk(root);
    if (this.cancelled) {
      this.o.emit({ type: "notice", message: "Run cancelled" });
      return;
    }
    this.report = null;
    const projectDir = this.projectDir();
    const argFile = props.rows.length ? path.join(projectDir, "run-args.txt") : null;
    const targetFile = o.targetFile ? path.join(projectDir, "run-targets.txt") : null;
    const mask = makeMasker(props.secrets);
    const emit = (e: AppEvent) => this.o.emit(maskEvent(e, mask));
    const created = new Date();
    // also creates the project folder the argument file goes in
    const { id, dir: outDir } = createRunDir(projectDir, created);
    let startedAt = created.getTime(); // the time of runStarted; the folder's if the run never starts
    let ended = false; // the run reached its end events, so it gets a run.json
    try {
      if (argFile) writeArgFile(argFile, props.rows);
      if (targetFile) writeTargetFile(targetFile, o.targets); // if it cannot be written, nothing runs
      const state = new RunState(
        tree,
        leaves.map((n) => n.key),
      );
      /** Writes run.json (R2), applies retention and sends `history`. A failure is a notice: it must not replace the run's outcome. */
      const store = (
        done: Extract<AppEvent, { type: "runFinished" }>,
        fingerprint: string | null,
      ) => {
        try {
          const record: RunRecord = {
            version: 1,
            startedAt,
            durationMillis: Date.now() - startedAt,
            env,
            threads,
            fingerprint,
            filter,
            selection: selection && {
              kind: selection.kind,
              file: selection.file,
              name: selection.name,
              line: selection.line,
              key: selection.key,
            },
            failedOf,
            outcome: done.outcome,
            exitCode: done.outcome === "cancelled" ? null : done.exitCode, // a kill's code is not Karate's
            tests: storedTests(leaves, state.results()),
          };
          writeRunRecord(outDir, record);
          pruneHistory(projectDir);
          this.o.emit(this.historyEvent());
        } catch (err) {
          emit({
            type: "notice",
            message: `The run could not be saved to the history: ${(err as Error).message}`,
          });
        }
      };
      const report = path.join(outDir, "karate-summary.html");
      /** Sends the end events and returns `runFinished`, for `store`. An exit code of null forces outcome 'failed'. */
      const end = (code: number | null, cancelled: boolean) => {
        ended = true;
        let done: Extract<AppEvent, { type: "runFinished" }> | undefined;
        for (let e of state.finish(code, cancelled, false)) {
          if (e.type === "runFinished") {
            this.report = e.outcome === "finished" && existsSync(report) ? report : null;
            e = { ...e, reportAvailable: this.report !== null };
            done = e;
          }
          emit(e);
        }
        return done!;
      };
      const tail = new JsonlTail(path.join(outDir, "karate-json", "karate-events.jsonl"));
      const proc = this.spawn(
        outDir,
        false,
        targetFile ? [`@${targetFile}`] : o.targets,
        env,
        (lines) => emit({ type: "console", lines }),
        argFile,
        filter,
        threads,
      );
      try {
        if (proc.pid !== -1) writeFileSync(this.pidFile, String(proc.pid));
      } catch (err) {
        // without the pid file a crash of the app would leave this JVM running: do not run
        await proc.kill();
        this.proc = null;
        store(end(null, false), null);
        emit({ type: "notice", message: (err as Error).message });
        return;
      }
      startedAt = Date.now();
      emit({
        type: "runStarted",
        selection: (selection ?? tree).key, // stored tests can span the tree
        runId: id,
        keys: state.keys,
        total: state.keys.length,
        startedAt,
        env,
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

      const done = end(fatal ? null : code, this.cancelled && !fatal);
      // dispose is quitting the app: no second walk
      const after = this.disposing ? null : await walk(root);
      store(done, before !== null && before === after ? before : null);
      if (fatal) emit({ type: "notice", message: fatal.message });
    } finally {
      // An antivirus or an indexer can hold it a moment (Electron's Node 24.22 retries a file; Node
      // 24.15 does not, so the unit tests cover only the notice). A failure must not replace the run's outcome.
      for (const file of [argFile, targetFile]) {
        try {
          if (file) rmSync(file, { force: true, maxRetries: 5 });
        } catch (err) {
          emit({
            type: "notice",
            message: `The run's argument file could not be deleted; the app deletes it when it next starts. ${(err as Error).message}`,
          });
        }
      }
      if (!ended) {
        // An error ended the run before its end events: nothing to list. Best effort, as the next start deletes it too.
        try {
          rmSync(outDir, { recursive: true, force: true, maxRetries: 5 });
        } catch {
          // left for removeLeftoverRuns
        }
      }
    }
  }
}
