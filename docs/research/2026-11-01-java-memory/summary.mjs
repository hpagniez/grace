// Medians per project, threads and flags from a measure.mjs log.
import { readFileSync } from "node:fs";
const rows = readFileSync(process.argv[2], "utf8")
  .split("\n")
  .filter((l) => l.startsWith("{"))
  .map((l) => JSON.parse(l));
const med = (xs) => {
  const s = [...xs].sort((a, b) => a - b);
  return s.length % 2 ? s[(s.length - 1) / 2] : (s[s.length / 2 - 1] + s[s.length / 2]) / 2;
};
const groups = Map.groupBy(rows, (r) => `${r.size} T${r.threads} ${r.label}`);
console.log("case | n | peak MB (min-max) | java CPU ms | wall ms | max after GC MB | OOM/exit");
for (const [k, rs] of groups) {
  const peaks = rs.map((r) => r.javaPeakMB);
  console.log(
    `${k} | ${rs.length} | ${med(peaks)} (${Math.min(...peaks)}-${Math.max(...peaks)}) | ${med(rs.map((r) => r.javaCpuMs))} | ${med(rs.map((r) => r.wallMs))} | ${Math.max(...rs.map((r) => r.maxAfterGcMB))} | ${rs.map((r) => (r.oom ? "OOM" : r.exit)).join(",")}`,
  );
}
