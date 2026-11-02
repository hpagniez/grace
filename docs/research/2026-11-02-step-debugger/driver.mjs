// Spike driver: node driver.mjs <repo> <classes dir> <project dir> <hold ms>
// Opens a socket, starts Karate with the listener, answers every pause after <hold ms>.
import { spawn } from "node:child_process";
import { createServer } from "node:net";
import { createInterface } from "node:readline";
import path from "node:path";

const [repo, classes, project, hold = "0"] = process.argv.slice(2);
const pauses = [];
const server = createServer((socket) => {
  createInterface({ input: socket }).on("line", (line) => {
    const msg = JSON.parse(line);
    pauses.push({ at: Date.now(), ...msg });
    console.log(JSON.stringify(msg));
    setTimeout(() => socket.write("continue\n"), Number(hold));
  });
});
server.listen(0, "127.0.0.1", () => {
  const { port } = server.address();
  const java = path.join(repo, "vendor/jre/bin/java.exe");
  const cp = [path.join(repo, "vendor/karate.jar"), classes].join(";");
  const started = Date.now();
  const child = spawn(java, [
    `-Dgrace.debug.port=${port}`, "-Dstdout.encoding=UTF-8", "-cp", cp, "io.karatelabs.Main", "run",
    "--no-color", "-T", "1", "--listener", "grace.DebugListener", "-w", project,
    "-o", path.join(project, "target"), "-f", "karate:jsonl", project,
  ], { cwd: project, stdio: ["ignore", "inherit", "inherit"] });
  child.on("exit", (code) => {
    console.log(JSON.stringify({ exit: code, pauses: pauses.length, wallMs: Date.now() - started }));
    server.close();
  });
});
