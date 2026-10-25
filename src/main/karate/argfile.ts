import { readdirSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import type { PlainRow } from "../../shared/env";

/** Inside the quotes of a Java argument file, `\` and `"` are escapes. */
const quote = (s: string) => s.replace(/[\\"]/g, "\\$&");

/** One `"-D<name>=<value>"` line per row, each ended by CRLF. */
export function argFileText(rows: PlainRow[]): string {
  return rows.map((r) => `"${quote(`-D${r.name}=${r.value}`)}"\r\n`).join("");
}

/** Java reads the file in the Windows code page; the values are ASCII, so latin1 is exact. */
export function writeArgFile(file: string, rows: PlainRow[]): void {
  writeFileSync(file, argFileText(rows), "latin1");
}

/** picocli reads its `@file` as UTF-8 (unlike the Java launcher): one `"target"` line per target, CRLF. */
export function writeTargetFile(file: string, targets: string[]): void {
  writeFileSync(file, targets.map((t) => `"${quote(t)}"\r\n`).join(""), "utf8");
}

/** Deletes every `run-args.txt` and `run-targets.txt` under `<dataDir>/projects/<hash>/` that a crash left behind. Never throws. */
export function removeLeftoverArgFiles(dataDir: string): void {
  const projects = path.join(dataDir, "projects");
  let names: string[];
  try {
    names = readdirSync(projects);
  } catch {
    return;
  }
  for (const name of names) {
    for (const f of ["run-args.txt", "run-targets.txt"]) {
      try {
        rmSync(path.join(projects, name, f), { force: true });
      } catch {
        // one stuck file must not keep the others
      }
    }
  }
}
