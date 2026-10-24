// Runs the bundled Karate on the fixture projects and records real event files into test/fixtures/events/.
import { spawn } from "node:child_process";
import { cp, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { homedir, tmpdir } from "node:os";
import { delimiter, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const java = join(root, "vendor", "jre", "bin", "java.exe");
const jar = join(root, "vendor", "karate.jar");
const spike = join(root, "docs", "research", "2026-10-24-karate-cli-spike");
const eventsDir = join(root, "test", "fixtures", "events");

// The spec command (docs/specs/2026-10-24-walking-skeleton.md, "Spawning Karate").
async function karate(projectRoot, targets, { dryRun = false } = {}) {
  const out = await mkdtemp(join(tmpdir(), "kk-out-"));
  const args = [
    "-Dstdout.encoding=UTF-8",
    "-Dstderr.encoding=UTF-8",
    "-cp",
    `${jar}${delimiter}${projectRoot}`,
    "io.karatelabs.Main",
    "run",
    "--no-color",
    "-w",
    projectRoot,
    "-o",
    out,
    "-B",
    "false",
    "-f",
    "html,karate:jsonl",
    ...(dryRun ? ["-D"] : []),
    ...targets,
  ];
  const env = { ...process.env };
  for (const k of ["KARATE_OPTIONS", "JAVA_TOOL_OPTIONS", "JDK_JAVA_OPTIONS", "_JAVA_OPTIONS"])
    delete env[k];
  const child = spawn(java, args, { cwd: projectRoot, env, shell: false, windowsHide: true });
  let log = "";
  child.stdout.on("data", (d) => (log += d));
  child.stderr.on("data", (d) => (log += d));
  const code = await new Promise((res) => child.on("close", res));
  console.log(`exit ${code}`);
  if (code !== 0 && code !== 1) throw new Error(`karate exited ${code}:\n${log}`);
  return { eventFile: join(out, "karate-json", "karate-events.jsonl"), out };
}

// JSON-escaped text of a Windows path, as it appears inside event lines.
const esc = (p) => JSON.stringify(p).slice(1, -1);
const variants = (p) => [esc(p), esc(p.replaceAll("\\", "/")), p.replaceAll("\\", "/")];

// Replaces the fixture root with C:\fixture, then any other profile/temp path, so no personal path is committed.
async function writeSanitised(eventFile, name, fixtureRoot) {
  const subs = [
    ...variants(fixtureRoot).map((v) => [v, esc("C:\\fixture")]),
    ...[tmpdir(), homedir()].flatMap((p) => variants(p).map((v) => [v, esc("C:\\sanitised")])),
  ];
  const lines = (await readFile(eventFile, "utf8")).split("\n").filter(Boolean);
  const cleaned = lines.map((line) => {
    for (const [from, to] of subs) line = line.replaceAll(from, to);
    if (/hugop|AppData/i.test(line))
      throw new Error(`personal path left in ${name}: ${line.slice(0, 200)}`);
    JSON.parse(line);
    return line;
  });
  await writeFile(join(eventsDir, name), cleaned.join("\n") + "\n");
  console.log(`${name}: ${cleaned.length} lines`);
}

async function main() {
  await mkdir(eventsDir, { recursive: true });
  const work = await mkdtemp(join(tmpdir(), "kk-fixture-"));
  try {
    const clean = join(work, "clean");
    await cp(join(root, "test", "fixtures", "projects", "clean"), clean, { recursive: true });
    let r = await karate(clean, [join(clean, "tests")], { dryRun: true });
    await writeSanitised(r.eventFile, "dryrun-clean.jsonl", clean);
    // "Run all" targets the feature root, so this also proves @ignore keeps start-mock and mocks out.
    r = await karate(clean, [clean]);
    await writeSanitised(r.eventFile, "run-clean.jsonl", clean);

    const spikeCopy = join(work, "spike");
    await cp(join(spike, "project"), spikeCopy, { recursive: true });
    r = await karate(spikeCopy, [join(spikeCopy, "extra", "outline-sizes.feature")], {
      dryRun: true,
    });
    await writeSanitised(r.eventFile, "dryrun-outline-sizes.jsonl", spikeCopy);

    for (const f of [
      "1-normal-run-failure",
      "2-outline-rows-and-called-feature",
      "3-dryrun-outline-feature",
      "4-after-taskkill-complete-file",
    ]) {
      await cp(join(spike, "samples", `${f}.jsonl`), join(eventsDir, `${f}.jsonl`));
    }
  } finally {
    await rm(work, { recursive: true, force: true });
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
