import { execFile } from "node:child_process";
import { readdirSync, rmSync } from "node:fs";
import { promisify } from "node:util";
import { TAR } from "./updates/fetchZip";

const run = promisify(execFile);

/** Grace's record and Karate's event stream: not part of the report. */
const LEFT_OUT = new Set(["run.json", "karate-json"]);

/**
 * Writes a run folder's report to `dest` as a zip with Windows' tar, names in UTF-8, whatever `dest`'s
 * extension (docs/specs/2026-11-03-export-reports.md R3). On failure, removes `dest` and rejects with tar's reason.
 */
export async function zipReport(dir: string, dest: string): Promise<void> {
  try {
    const entries = readdirSync(dir).filter((e) => !LEFT_OUT.has(e));
    await run(
      TAR,
      ["-c", "--format", "zip", "--options", "hdrcharset=UTF-8", "-f", dest, "-C", dir, ...entries],
      { windowsHide: true },
    );
  } catch (e) {
    rmSync(dest, { force: true });
    const { stderr } = e as { stderr?: string };
    throw new Error(stderr?.trim() || (e as Error).message);
  }
}
