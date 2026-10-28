import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  rmSync,
  truncateSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { holdFolder } from "../../test/hold";
import { HISTORY_LIMIT, type RunRecord } from "../shared/history";
import { NO_FILTER } from "../shared/tags";
import {
  clearHistory,
  createRunDir,
  listHistory,
  listRuns,
  pruneHistory,
  readRunRecord,
  removeLeftoverRuns,
  runId,
  writeRunRecord,
} from "./history";

const temps: string[] = [];
const tmp = () => {
  const d = mkdtempSync(path.join(tmpdir(), "kk-hist-"));
  temps.push(d);
  return d;
};
afterEach(() => {
  for (const d of temps.splice(0)) rmSync(d, { recursive: true, force: true, maxRetries: 5 });
});

const record = (startedAt: number, extra: Partial<RunRecord> = {}): RunRecord => ({
  version: 1,
  startedAt,
  durationMillis: 1000,
  env: null,
  threads: 1,
  fingerprint: null,
  filter: NO_FILTER,
  selection: { kind: "folder", file: "", name: "", line: null, key: "d:" },
  failedOf: null,
  outcome: "finished",
  exitCode: 1,
  tests: [
    {
      kind: "scenario",
      file: "a.feature",
      refId: "[1:3]",
      name: "ok",
      line: 3,
      key: "s:a#1",
      status: "passed",
    },
    {
      kind: "scenario",
      file: "a.feature",
      refId: "[2:6]",
      name: "bad",
      line: 6,
      key: "s:a#2",
      status: "failed",
    },
  ],
  ...extra,
});

/**
 * Holds a run folder the way Windows does: a process running in it, or in a subfolder of it named to
 * come after run.json, so that a recursive delete removes run.json before it fails. (An open file
 * holds nothing: Node deletes with POSIX semantics on Windows 11.) Returns the release.
 */
function hold(runDir: string, inSubfolder: boolean): Promise<() => Promise<void>> {
  const dir = inSubfolder ? path.join(runDir, "zz-held") : runDir;
  mkdirSync(dir, { recursive: true });
  return holdFolder(dir);
}

/** `runs/<id>` with the given `run.json` text (none: no file). */
function runFolder(projectDir: string, id: string, json?: string): string {
  const dir = path.join(projectDir, "runs", id);
  mkdirSync(dir, { recursive: true });
  if (json !== undefined) writeFileSync(path.join(dir, "run.json"), json);
  return dir;
}

/** The id of the n-th second of a day, so names sort like their order. */
const idAt = (n: number) => `20261025-${String(1000000 + n).slice(1)}-000`;

describe("runId", () => {
  it("formats local time", () => {
    expect(runId(new Date(2026, 9, 25, 14, 32, 5, 7), () => false)).toBe("20261025-143205-007");
  });
  it("appends a suffix when taken", () => {
    const taken = new Set(["20261025-143205-007"]);
    const now = new Date(2026, 9, 25, 14, 32, 5, 7);
    expect(runId(now, (id) => taken.has(id))).toBe("20261025-143205-007-2");
    taken.add("20261025-143205-007-2");
    expect(runId(now, (id) => taken.has(id))).toBe("20261025-143205-007-3");
  });
});

describe("createRunDir", () => {
  it("creates runs/<id>, and a new one when the id is taken", () => {
    const project = tmp();
    const now = new Date(2026, 9, 25, 14, 32, 5, 7);
    const a = createRunDir(project, now);
    const b = createRunDir(project, now);
    expect(a.id).toBe("20261025-143205-007");
    expect(a.dir).toBe(path.join(project, "runs", a.id));
    expect(b.id).toBe("20261025-143205-007-2");
    expect(existsSync(a.dir) && existsSync(b.dir)).toBe(true);
  });
});

describe("run.json", () => {
  it("write then list round-trips", () => {
    const project = tmp();
    const { id, dir } = createRunDir(project, new Date(2026, 9, 25, 14, 32, 5, 7));
    const r = record(1792937525123, { env: "qa" });
    writeRunRecord(dir, r);
    expect(readRunRecord(project, id)).toEqual(r);
    expect(listHistory(project)).toEqual([
      {
        id,
        startedAt: r.startedAt,
        durationMillis: 1000,
        env: "qa",
        threads: 1,
        filter: NO_FILTER,
        selection: r.selection,
        failedOf: null,
        outcome: "finished",
        exitCode: 1,
        passed: 1,
        failed: 1,
        total: 2,
        reportAvailable: false,
      },
    ]);
  });

  it("readRunRecord refuses an id with ..", () => {
    const project = tmp();
    writeRunRecord(project, record(1)); // projectDir/run.json is what runs/../run.json would reach
    expect(readRunRecord(project, "..")).toBeNull();
    expect(readRunRecord(project, "20261025-143205-007/../..")).toBeNull();
    expect(readRunRecord(project, "")).toBeNull();
  });

  it("readRunRecord reads null for a missing folder, a folder with no file and a file over 64 MB", () => {
    const project = tmp();
    expect(readRunRecord(project, idAt(1))).toBeNull();
    runFolder(project, idAt(2));
    expect(readRunRecord(project, idAt(2))).toBeNull();
    const text = JSON.stringify(record(3));
    // grown with truncate: writing 64 MB takes over 5 s on the CI runner
    truncateSync(path.join(runFolder(project, idAt(3), text), "run.json"), 64 * 1024 * 1024 + 1);
    expect(readRunRecord(project, idAt(3))).toBeNull();
    runFolder(project, idAt(4), text);
    expect(readRunRecord(project, idAt(4))).not.toBeNull();
    // a suite of 25k+ tests writes more than 5 MB: still listed
    runFolder(project, idAt(5), text + " ".repeat(6 * 1024 * 1024));
    expect(readRunRecord(project, idAt(5))).not.toBeNull();
    expect(
      listHistory(project)
        .map((e) => e.id)
        .sort(),
    ).toEqual([idAt(4), idAt(5)]);
  });
});

describe("listHistory", () => {
  it("sorts by startedAt, not by folder name", () => {
    const project = tmp();
    runFolder(project, idAt(1), JSON.stringify(record(300)));
    runFolder(project, idAt(2), JSON.stringify(record(200)));
    runFolder(project, idAt(3), JSON.stringify(record(100)));
    expect(listHistory(project).map((e) => e.startedAt)).toEqual([300, 200, 100]);
  });

  it("skips invalid and unknown-version files", () => {
    const project = tmp();
    runFolder(project, idAt(1), JSON.stringify(record(100)));
    runFolder(project, idAt(2), "{not json");
    runFolder(project, idAt(3), JSON.stringify({ ...record(300), version: 2 }));
    runFolder(project, idAt(4), JSON.stringify({ ...record(400), outcome: "exploded" }));
    runFolder(project, idAt(5)); // no run.json
    runFolder(project, "notes"); // not a run id
    writeFileSync(path.join(project, "runs", idAt(6)), "a file named like a run");
    expect(listHistory(project).map((e) => e.id)).toEqual([idAt(1)]);
  });

  it("is empty, and does not throw, when there is no runs folder", () => {
    expect(listHistory(path.join(tmp(), "missing"))).toEqual([]);
  });

  it("caps at 20", () => {
    const project = tmp();
    for (let i = 1; i <= 25; i++) runFolder(project, idAt(i), JSON.stringify(record(i)));
    const list = listHistory(project);
    expect(list).toHaveLength(HISTORY_LIMIT);
    expect(list[0].startedAt).toBe(25);
    expect(list.at(-1)!.startedAt).toBe(6);
  });

  it("reportAvailable follows karate-summary.html", () => {
    const project = tmp();
    const withReport = runFolder(project, idAt(1), JSON.stringify(record(100)));
    runFolder(project, idAt(2), JSON.stringify(record(200)));
    writeFileSync(path.join(withReport, "karate-summary.html"), "<html></html>");
    expect(listHistory(project).map((e) => [e.id, e.reportAvailable])).toEqual([
      [idAt(2), false],
      [idAt(1), true],
    ]);
  });
});

describe("listRuns", () => {
  it("returns what listHistory lists", () => {
    const project = tmp();
    const records = [record(200, { env: "qa" }), record(300), record(100)];
    records.forEach((r, i) => runFolder(project, idAt(i + 1), JSON.stringify(r)));
    runFolder(project, idAt(4), "{not json");
    writeFileSync(path.join(project, "runs", idAt(2), "karate-summary.html"), "<html></html>");
    const runs = listRuns(project);
    expect(runs.map((r) => r.id)).toEqual(listHistory(project).map((e) => e.id));
    expect(runs.map((r) => r.id)).toEqual([idAt(2), idAt(1), idAt(3)]);
    expect(runs.map((r) => r.record)).toEqual([records[1], records[0], records[2]]);
    expect(runs.map((r) => r.reportAvailable)).toEqual([true, false, false]);
  });
});

describe("pruneHistory", () => {
  it("keeps the 20 newest names, invalid ones included", () => {
    const project = tmp();
    // 3 is damaged and old; 12 and 24 are damaged and among the newest 20: names decide, not content
    for (let i = 1; i <= 25; i++)
      runFolder(project, idAt(i), [3, 12, 24].includes(i) ? "{" : JSON.stringify(record(i)));
    pruneHistory(project);
    const left = readdirSync(path.join(project, "runs")).sort();
    expect(left).toEqual(Array.from({ length: 20 }, (_, i) => idAt(i + 6)));
  });

  it("keeps what it should when there are fewer than 20, and never throws", () => {
    const project = tmp();
    expect(() => pruneHistory(path.join(project, "missing"))).not.toThrow();
    for (let i = 1; i <= 3; i++) runFolder(project, idAt(i), JSON.stringify(record(i)));
    pruneHistory(project);
    expect(readdirSync(path.join(project, "runs"))).toHaveLength(3);
  });

  it("skips a held folder and deletes the others, then tries it again", async () => {
    const project = tmp();
    for (let i = 1; i <= 23; i++) runFolder(project, idAt(i), JSON.stringify(record(i)));
    const release = await hold(path.join(project, "runs", idAt(1)), false);
    try {
      expect(() => pruneHistory(project)).not.toThrow();
      expect(readdirSync(path.join(project, "runs"))).toHaveLength(21); // the 20 newest and the held one
    } finally {
      await release();
    }
    pruneHistory(project);
    expect(readdirSync(path.join(project, "runs"))).toHaveLength(20);
  });
});

describe("clearHistory", () => {
  it("removes all", () => {
    const project = tmp();
    for (let i = 1; i <= 3; i++) runFolder(project, idAt(i), JSON.stringify(record(i)));
    expect(clearHistory(project)).toBe(true);
    expect(readdirSync(path.join(project, "runs"))).toEqual([]);
    expect(listHistory(project)).toEqual([]);
  });

  it.each([
    ["in the folder", false],
    ["in a subfolder", true],
  ])(
    "reports a folder held by a process %s and keeps that run listed",
    async (_where, inSubfolder) => {
      const project = tmp();
      for (let i = 1; i <= 3; i++) runFolder(project, idAt(i), JSON.stringify(record(i)));
      const held = path.join(project, "runs", idAt(2));
      writeFileSync(path.join(held, "karate-summary.html"), "<html></html>");
      mkdirSync(path.join(held, "karate-json"));
      writeFileSync(path.join(held, "karate-json", "karate-events.jsonl"), "{}");
      const release = await hold(held, inSubfolder);
      try {
        expect(clearHistory(project)).toBe(false);
        expect(readdirSync(path.join(project, "runs"))).toEqual([idAt(2)]);
        // its report may be gone, its record is what keeps it in the list
        expect(readRunRecord(project, idAt(2))).toEqual(record(2));
        expect(listHistory(project).map((e) => e.id)).toEqual([idAt(2)]);
      } finally {
        await release();
      }
      expect(clearHistory(project)).toBe(true);
      expect(readdirSync(path.join(project, "runs"))).toEqual([]);
      expect(listHistory(project)).toEqual([]);
    },
  );

  it("is true when there is nothing to delete", () => {
    expect(clearHistory(path.join(tmp(), "missing"))).toBe(true);
  });
});

describe("removeLeftoverRuns", () => {
  it("deletes folders without run.json and 1.0 run folders, keeps valid runs", () => {
    const dataDir = tmp();
    const p1 = path.join(dataDir, "projects", "h1");
    const p2 = path.join(dataDir, "projects", "h2");
    runFolder(p1, idAt(1), JSON.stringify(record(1)));
    runFolder(p1, idAt(2)); // a crash left it
    runFolder(p2, idAt(3), "{"); // damaged, but run.json is there: retention's business
    runFolder(p2, idAt(4));
    for (const p of [p1, p2]) {
      mkdirSync(path.join(p, "run", "karate-json"), { recursive: true }); // version 1.0
      mkdirSync(path.join(p, "dryrun"), { recursive: true });
    }
    writeFileSync(path.join(dataDir, "settings.json"), "{}");
    writeFileSync(path.join(dataDir, "projects", "stray-file"), "x");

    removeLeftoverRuns(dataDir);

    expect(readdirSync(path.join(p1, "runs"))).toEqual([idAt(1)]);
    expect(readdirSync(path.join(p2, "runs"))).toEqual([idAt(3)]);
    for (const p of [p1, p2]) {
      expect(existsSync(path.join(p, "run"))).toBe(false);
      expect(existsSync(path.join(p, "dryrun"))).toBe(true);
    }
    expect(existsSync(path.join(dataDir, "settings.json"))).toBe(true);
  });

  it("never throws, whatever is there", () => {
    const dataDir = tmp();
    expect(() => removeLeftoverRuns(path.join(dataDir, "missing"))).not.toThrow();
    mkdirSync(path.join(dataDir, "projects", "h1"), { recursive: true });
    writeFileSync(
      path.join(dataDir, "projects", "h1", "runs"),
      "a file where a folder is expected",
    );
    writeFileSync(path.join(dataDir, "projects", "file"), "x");
    expect(() => removeLeftoverRuns(dataDir)).not.toThrow();
  });
});
