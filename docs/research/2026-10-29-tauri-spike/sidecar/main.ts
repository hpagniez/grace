// Option 2: Grace's real main process (src/main, unchanged) as a Node sidecar of the Tauri shell.
// The shell writes one JSON request per line on stdin: {"id", "channel", "args"}. This writes one
// JSON line per answer {"id", "result"} or per app event {"event"} on stdout. stdin closing quits.
// The handlers are src/main.ts's, minus Electron: the shell shows the folder picker and opens
// reports, so project:open gets the chosen folder and the report channels answer with a path.
import path from "node:path";
import { createInterface } from "node:readline";
import { removeLeftoverRuns } from "../../../../src/main/history";
import { removeLeftoverArgFiles } from "../../../../src/main/karate/argfile";
import { killOrphan } from "../../../../src/main/karate/process";
import { Runner } from "../../../../src/main/runner";
import { fakeSecretBox } from "../../../../src/main/secrets";
import { loadLastProject } from "../../../../src/main/settings";
import { vendorPaths } from "../../../../src/main/vendor";
import { parsePropertiesEdit } from "../../../../src/shared/env";
import type { AppEvent } from "../../../../src/shared/events";
import { RUN_ID } from "../../../../src/shared/history";
import { CH } from "../../../../src/shared/ipc";
import { parseFilterState } from "../../../../src/shared/tags";

const dataDir = process.env.GRACE_DATA!;
const vendor = vendorPaths(process.env.GRACE_VENDOR!);
const write = (msg: unknown) => void process.stdout.write(JSON.stringify(msg) + "\n");
const emit = (e: AppEvent) => write({ event: e });
const invalid = () => emit({ type: "notice", message: "Invalid request" });

// shortcut: no safeStorage in Node, so secret run properties are unavailable; a real port would
// encrypt them in the Rust shell (DPAPI) and pass them over this pipe.
const runner = new Runner({ vendor, dataDir, secrets: fakeSecretBox(false), emit });

const isRoot = (root: unknown): root is string => typeof root === "string" && root.length <= 1000;
const isRunId = (id: unknown): id is string => typeof id === "string" && RUN_ID.test(id);

const handlers: Record<string, (...args: unknown[]) => unknown> = {
  [CH.ready]: () => {
    for (const e of runner.replay()) emit(e);
    void runner.sendRecent();
  },
  [CH.open]: (root) => (isRoot(root) ? runner.open(root) : undefined),
  [CH.openRecent]: (root) => (isRoot(root) ? runner.openRecent(root) : invalid()),
  [CH.removeRecent]: (root) => (isRoot(root) ? runner.removeRecent(root) : invalid()),
  [CH.close]: () => runner.close(),
  [CH.refresh]: () => runner.refresh(),
  [CH.run]: (key) =>
    typeof key === "string" && key.length <= 1000 && /^[dfos]:/.test(key)
      ? runner.run(key)
      : invalid(),
  [CH.filter]: (f) => {
    const filter = parseFilterState(f);
    return filter ? runner.setFilter(filter) : invalid();
  },
  [CH.threads]: (n) => runner.setThreads(n),
  [CH.env]: (name) =>
    name === null || (typeof name === "string" && name.length <= 64)
      ? runner.setEnv(name)
      : invalid(),
  [CH.properties]: (e) => {
    const edit = parsePropertiesEdit(e);
    if (edit) return runner.saveProperties(edit);
    invalid();
    return false;
  },
  [CH.cancel]: () => runner.cancel(),
  [CH.report]: () => runner.reportPath(),
  [CH.rerun]: (id) => (isRunId(id) ? runner.rerun(id) : invalid()),
  [CH.rerunFailed]: (id) => (isRunId(id) ? runner.rerunFailed(id) : invalid()),
  [CH.runReport]: (id) => (isRunId(id) ? runner.runReportPath(id) : invalid()),
  [CH.clearHistory]: () => runner.clearHistory(),
};

createInterface({ input: process.stdin })
  .on("line", async (line) => {
    let id: unknown = null;
    try {
      const req = JSON.parse(line);
      id = req.id;
      const fn = handlers[req.channel];
      if (!fn || !Array.isArray(req.args)) throw new Error("Invalid request");
      write({ id, result: (await fn(...req.args)) ?? null });
    } catch (err) {
      // Handlers never reject to the renderer: a failure becomes a notice, as in src/main.ts.
      emit({ type: "notice", message: err instanceof Error ? err.message : String(err) });
      write({ id, result: null });
    }
  })
  .on("close", () => {
    runner
      .dispose()
      .catch(() => {})
      .then(() => process.exit(0));
  });

// No top-level await: a Node single executable application runs a CommonJS bundle.
void (async () => {
  try {
    await killOrphan(path.join(dataDir, "running.pid"), vendor.jreDir);
  } catch (err) {
    console.error(err);
  }
  removeLeftoverArgFiles(dataDir);
  removeLeftoverRuns(dataDir);
  const last = process.env.SPIKE_PROJECT || loadLastProject(dataDir);
  if (last) await runner.open(last).catch((err) => emit({ type: "notice", message: String(err) }));
})();
