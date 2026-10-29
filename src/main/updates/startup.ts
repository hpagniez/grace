// What switch.ps1 leaves behind and what Grace does with it at the next start
// (docs/specs/2026-10-28-app-updates.md R9.5, R10.6 to R10.9, R12).
import { readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { rename, rm } from "node:fs/promises";
import path from "node:path";
import { setTimeout as sleep } from "node:timers/promises";
import type { Backup, UpdateSettings } from "../../shared/updates";
import { parseVersion } from "../../shared/version";
import { isObject, parseUpdateSettings, sameFolder } from "../settings";
import { isGraceFolder, ownStaging } from "./preflight";

/** `update-result.json`, as the script writes it (R10.5). */
export interface SwitchResult {
  from: string;
  to: string;
  rollback: boolean;
  ok: boolean;
  reverted: boolean;
  error: string | null;
  updates: UpdateSettings | null;
}

const staging = (installDir: string) => `${installDir}.update`;

/** A JSON file's object, or null when it is missing, unreadable or not an object. Tolerates a BOM. */
function readObject(file: string): Record<string, unknown> | null {
  try {
    // TextDecoder drops a leading byte order mark
    const x: unknown = JSON.parse(new TextDecoder().decode(readFileSync(file)));
    return isObject(x) ? x : null;
  } catch {
    return null;
  }
}

/** The version the switch in `<install>.update\plan.json` installs, or null. */
const planTo = (installDir: string): unknown =>
  readObject(path.join(staging(installDir), "plan.json"))?.to ?? null;

function pidAlive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch (e) {
    return (e as NodeJS.ErrnoException).code === "EPERM"; // alive, but not ours to signal
  }
}

// The script writes `switching` non-atomically, so an empty or unreadable one counts for 30 s. One
// older than 30 min is never a running script (its longest run is far shorter): a killed script's
// pid may since belong to an unrelated process.
const EMPTY_FILE_MS = 30_000;
const STALE_FILE_MS = 30 * 60_000;

/** True when `<install>.update\switching` names a live process, whatever version runs. Never throws. */
export function switchRunning(installDir: string): boolean {
  const file = path.join(staging(installDir), "switching");
  try {
    const age = Date.now() - statSync(file).mtimeMs;
    if (age > STALE_FILE_MS) return false;
    const text = readFileSync(file, "utf8").trim();
    const pid = Number(text);
    return /^\d+$/.test(text) && pid > 0 ? pidAlive(pid) : age < EMPTY_FILE_MS;
  } catch {
    return false;
  }
}

/**
 * R10.9: the switch script runs and `running` is not the version it installs, so this Grace must
 * quit. The new version is exempt: the script waits for it to start (R10.6). Never throws.
 */
export function switchInProgress(installDir: string, running: string): boolean {
  return switchRunning(installDir) && planTo(installDir) !== running;
}

/** R10.6: tells the script the new version started, once its window has shown. */
export function markStarted(installDir: string, running: string): void {
  try {
    if (planTo(installDir) === running)
      writeFileSync(path.join(staging(installDir), "started"), "");
  } catch {
    // the script then switches back, which is all that can be done
  }
}

/** R10.7: reads and deletes `<dataDir>\update-result.json`. Null when it is missing or malformed. */
export function takeResult(dataDir: string): SwitchResult | null {
  const file = path.join(dataDir, "update-result.json");
  const o = readObject(file);
  try {
    rmSync(file, { force: true });
  } catch {
    // held open for a moment: the notice then shows once more at the next start
  }
  if (!o) return null;
  // R10.5 does not list `reverted`: a script that omits it did not switch back
  const { from, to, rollback, ok, reverted = false, error } = o;
  if (typeof from !== "string" || !parseVersion(from)) return null;
  if (typeof to !== "string" || !parseVersion(to)) return null;
  if (typeof rollback !== "boolean" || typeof ok !== "boolean" || typeof reverted !== "boolean") {
    return null;
  }
  if (error !== null && typeof error !== "string") return null;
  const updates = isObject(o.updates) ? parseUpdateSettings(o.updates) : null;
  return { from, to, rollback, ok, reverted, error, updates };
}

/**
 * R10.7: the notice for a result. `running` tells a failure that left the old version in place
 * from one that left the new version installed, after a failed undo.
 */
export function resultNotice(r: SwitchResult, running: string): string {
  if (r.ok) return `${r.rollback ? "Rolled back" : "Updated"} to ${r.to}.`;
  const why = r.error ?? "an unknown error";
  return running === r.to
    ? `Couldn't switch to ${r.to} cleanly: ${why}. You are on ${r.to}.`
    : `Couldn't switch to ${r.to}: ${why}. You are still on ${r.from}.`;
}

/**
 * R10.6, R10.7: the update settings to save after a result. A success takes the carried settings,
 * since a restore replaced the saved ones; a rollback also turns auto-update off and skips `from`, so
 * it holds. A failure keeps `current`, and a switch that went back remembers `to` as failed.
 */
export function settingsAfter(r: SwitchResult, current: UpdateSettings): UpdateSettings {
  if (!r.ok) return r.reverted ? { ...current, failed: r.to } : current;
  const s = r.updates ?? current;
  return r.rollback ? { ...s, autoUpdate: false, skipped: r.from } : s;
}

/**
 * R10.8: once the script has ended, removes what a switch leaves next to the install folder: only
 * the folders a switch of this install made, never someone's own `Grace.old` or `C:\Tools.old`. A
 * script still alive after `timeoutMs` leaves them for the next start. Never rejects.
 */
export async function removeLeftovers(
  installDir: string,
  { pollMs = 1000, timeoutMs = 3 * 60_000 } = {},
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (switchRunning(installDir)) {
    if (Date.now() >= deadline) return;
    await sleep(pollMs);
  }
  const dir = staging(installDir);
  if (!ownStaging(dir)) return;
  // the plan proves a switch of this install made `.old` and `.failed`
  const planned = readObject(path.join(dir, "plan.json"))?.installDir;
  let kept = false;
  if (typeof planned === "string" && sameFolder(planned, installDir)) {
    for (const name of ["old", "failed"]) {
      const from = `${installDir}.${name}`;
      // moved in whole first: a removal cut short never leaves half a Grace folder outside
      if (isGraceFolder(from)) await rename(from, path.join(dir, name)).catch(() => (kept = true));
    }
  }
  // the plan stays while a folder could not be moved, so the next start tries again; the script
  // may hold its folder for a few ms after it deleted `switching`
  if (!kept)
    await rm(dir, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 }).catch(() => {});
}

/** R9: the saved copies of the data, newest first. A folder without a valid `backup.json` is not a copy. */
export function listBackups(dataDir: string): Backup[] {
  const root = path.join(dataDir, "backups");
  let names: string[];
  try {
    names = readdirSync(root, { withFileTypes: true }).flatMap((e) =>
      e.isDirectory() ? e.name : [],
    );
  } catch {
    return [];
  }
  const found: Backup[] = [];
  for (const name of names) {
    const meta = readObject(path.join(root, name, "backup.json"));
    const { version, savedAt } = meta ?? {};
    if (
      version === name &&
      parseVersion(name) &&
      typeof savedAt === "string" &&
      !Number.isNaN(Date.parse(savedAt))
    ) {
      found.push({ version: name, savedAt });
    }
  }
  return found.sort((a, b) => Date.parse(b.savedAt) - Date.parse(a.savedAt));
}
