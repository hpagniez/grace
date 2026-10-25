import { execFile, spawn } from "node:child_process";
import { readFileSync, rmSync } from "node:fs";
import path from "node:path";
import { createInterface } from "node:readline";
import { childEnv } from "./command";

export interface KarateProcess {
  pid: number;
  exit: Promise<number | null>;
  kill(): Promise<void>;
}

const run = (file: string, args: string[]) =>
  new Promise<string>((resolve) =>
    execFile(file, args, { windowsHide: true }, (_err, stdout) => resolve(String(stdout ?? ""))),
  );

// Full paths: a taskkill.exe or powershell.exe earlier on PATH must not be run instead.
const system32 = path.join(process.env.SystemRoot || "C:\\Windows", "System32");
const TASKKILL = path.join(system32, "taskkill.exe");
const POWERSHELL = path.join(system32, "WindowsPowerShell", "v1.0", "powershell.exe");

const taskkill = (pid: number) => run(TASKKILL, ["/PID", String(pid), "/T", "/F"]);

/** True if `p` settles within `ms`. */
export const within = (p: Promise<unknown>, ms: number) =>
  new Promise<boolean>((resolve) => {
    const timer = setTimeout(() => resolve(false), ms);
    const done = () => {
      clearTimeout(timer);
      resolve(true);
    };
    p.then(done, done);
  });

/**
 * Never uses a shell. Lines are flushed to `onLines` every 100 ms and once more before `exit` resolves.
 * `kill()` gives up after `killWaitMs` twice (taskkill, then child.kill), so quitting never hangs.
 */
export function spawnKarate(
  java: string,
  args: string[],
  cwd: string,
  onLines: (lines: string[]) => void,
  killWaitMs = 5000,
): KarateProcess {
  const child = spawn(java, args, {
    cwd,
    env: childEnv(process.env),
    shell: false,
    windowsHide: true,
  });
  let buffer: string[] = [];
  const flush = () => {
    if (buffer.length === 0) return;
    const lines = buffer;
    buffer = [];
    onLines(lines);
  };
  const timer = setInterval(flush, 100);
  const pump = (stream: NodeJS.ReadableStream) =>
    new Promise<void>((resolve) => {
      const rl = createInterface({ input: stream });
      rl.on("line", (l) => buffer.push(l));
      rl.on("close", resolve);
    });
  const drained = Promise.all([pump(child.stdout), pump(child.stderr)]);

  let exited = false;
  const exit = new Promise<number | null>((resolve) => {
    const finish = async (code: number | null, drain = true) => {
      if (exited) return;
      exited = true;
      if (drain) await drained;
      clearInterval(timer);
      flush();
      resolve(code);
    };
    child.on("error", (err) => {
      buffer.push(err.message);
      void finish(null, false); // spawn failed: the pipes never produce 'end'
    });
    child.on("close", (code) => void finish(code));
  });

  return {
    pid: child.pid ?? -1,
    exit,
    async kill() {
      if (exited) return;
      if (child.pid !== undefined) await taskkill(child.pid);
      if (await within(exit, killWaitMs)) return;
      child.kill();
      await within(exit, killWaitMs);
    },
  };
}

const norm = (p: string) => path.resolve(p).toLowerCase();

/** Kills the tree of the PID in `pidFile` only if it is a process running from inside `jreDir`. Always deletes the file. */
export async function killOrphan(pidFile: string, jreDir: string): Promise<void> {
  let text: string;
  try {
    text = readFileSync(pidFile, "utf8").trim();
  } catch {
    return; // no file: nothing to do
  }
  try {
    if (!/^\d+$/.test(text)) return;
    const pid = Number(text);
    // pid is a validated integer, so interpolating it into the command is safe.
    const exePath = (
      await run(POWERSHELL, [
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        `(Get-CimInstance Win32_Process -Filter "ProcessId=${pid}").ExecutablePath`,
      ])
    ).trim();
    if (!exePath) return;
    const dir = norm(jreDir);
    if (norm(exePath).startsWith(dir + path.sep)) await taskkill(pid);
  } finally {
    rmSync(pidFile, { force: true });
  }
}
