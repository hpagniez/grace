import { spawn } from "node:child_process";
import { createWriteStream, readdirSync, renameSync, rmSync } from "node:fs";
import { pipeline } from "node:stream/promises";
import { TAR } from "./updates/fetchZip";

/** Grace's record and Karate's event stream: not part of the report. */
const LEFT_OUT = new Set(["run.json", "karate-json"]);

/**
 * Writes a run folder's report to `dest` as a zip with Windows' tar, names in UTF-8, whatever `dest`'s
 * extension (docs/specs/2026-11-03-export-reports.md R3). tar reads its arguments in the ANSI code page,
 * so no path goes on its command line: it runs in `dir` and writes to its output, which Node saves next to
 * `dest` and then moves onto it. On failure `dest` keeps what it had, and it rejects with tar's reason.
 */
export async function zipReport(dir: string, dest: string): Promise<void> {
  const part = `${dest}.${process.pid}-${Date.now()}.tmp`;
  try {
    // Karate's own names, all ASCII
    const entries = readdirSync(dir).filter((e) => !LEFT_OUT.has(e));
    const tar = spawn(
      TAR,
      ["-c", "--format", "zip", "--options", "hdrcharset=UTF-8", "-f", "-", ...entries],
      { cwd: dir, windowsHide: true, stdio: ["ignore", "pipe", "pipe"] },
    );
    let stderr = "";
    tar.stderr.setEncoding("utf8").on("data", (d: string) => (stderr += d));
    const [, code] = await Promise.all([
      pipeline(tar.stdout, createWriteStream(part)),
      new Promise<number | null>((resolve, reject) => {
        tar.on("error", reject);
        tar.on("close", resolve);
      }),
    ]);
    if (code !== 0) throw new Error(stderr.trim() || `tar exited with code ${code}`);
    renameSync(part, dest);
  } catch (e) {
    rmSync(part, { force: true });
    throw e;
  }
}
