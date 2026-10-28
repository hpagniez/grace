import { spawnSync } from "node:child_process";
import { existsSync, writeFileSync } from "node:fs";
import path from "node:path";
import type { UpdateSettings } from "../../shared/updates";
import script from "./switch.ps1?raw";

/** What the switch script does (R10.1), written to `<stagingDir>\plan.json` next to it. */
export interface SwitchPlan {
  pid: number;
  from: string;
  to: string;
  rollback: boolean;
  restore: boolean;
  installDir: string;
  stagingDir: string;
  dataDir: string;
  updates: UpdateSettings;
  exitTimeoutSec: number;
  startTimeoutSec: number;
  keepBackups: number;
}

/** Windows PowerShell 5.1, by absolute path: never one found on PATH. */
export const POWERSHELL = path.join(
  process.env.SystemRoot ?? "C:\\Windows",
  "System32",
  "WindowsPowerShell",
  "v1.0",
  "powershell.exe",
);

/**
 * R8.5: writes the plan and the switch script into the staging folder and starts the script,
 * hidden. Returns once the script runs on its own, so Grace can quit; throws if it did not start.
 * `env` is the environment the script, and the Grace it starts, inherit.
 */
export function startSwitch(plan: SwitchPlan, env: NodeJS.ProcessEnv = process.env): void {
  const file = path.join(plan.stagingDir, "switch.ps1");
  writeFileSync(path.join(plan.stagingDir, "plan.json"), JSON.stringify(plan));
  writeFileSync(file, script);
  // Not `detached`: a detached powershell.exe has no console and exits without running anything.
  // This one would die with Grace (Node kills its children on exit), so the script starts a copy of
  // itself that outlives Grace, writes that copy's id to `switching` and exits (switch.ps1).
  // cwd: a process whose current folder is inside the install folder stops it being renamed, and
  // Grace started from Explorer runs in its install folder.
  const args = [
    "-NoProfile",
    "-NonInteractive",
    "-ExecutionPolicy",
    "Bypass",
    "-WindowStyle",
    "Hidden",
  ];
  spawnSync(POWERSHELL, [...args, "-File", file], {
    cwd: plan.stagingDir,
    env,
    windowsHide: true,
    stdio: "ignore",
    timeout: 60_000,
  });
  if (!existsSync(path.join(plan.stagingDir, "switching")))
    throw new Error("The switch script did not start");
}
