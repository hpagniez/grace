import { spawn } from "node:child_process";
import { once } from "node:events";

/**
 * Holds `dir` the way Windows does: a process whose working folder it is. Resolves once the child
 * runs, not on Node's "spawn" event: the child opens its working folder while it starts, so a delete
 * right after "spawn" usually wins the race. Returns the release.
 */
export async function holdFolder(dir: string): Promise<() => Promise<void>> {
  const holder = spawn(
    process.execPath,
    ["-e", "process.stdout.write('ready'); setTimeout(() => {}, 60000)"],
    { cwd: dir, stdio: ["ignore", "pipe", "ignore"] },
  );
  await once(holder.stdout, "data");
  return async () => {
    holder.kill();
    await once(holder, "exit");
  };
}
