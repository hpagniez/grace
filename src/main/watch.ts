// Watch mode (docs/specs/2026-11-03-watch-mode.md, R2): the OS's file events on the feature roots, never a poll.
import { statSync, watch, type FSWatcher } from "node:fs";
import path from "node:path";
import { SKIPPED_DIRS } from "./fingerprint";

/** How long no counted event must come before a batch of saves is handed over (R2.3). */
export const QUIET_MS = 300;

/** A path as `fs.watch` gives it, relative to its root: a `.feature` outside the skipped folders (R2.2). */
export function counts(rel: string): boolean {
  const parts = rel.toLowerCase().split(/[\\/]/);
  return parts.at(-1)!.endsWith(".feature") && !parts.some((p) => SKIPPED_DIRS.includes(p));
}

export interface FeatureWatcher {
  close(): void;
}

/**
 * Watches each root, a folder recursively or a single file, and hands over the absolute paths of the features
 * saved once `quietMs` passed with no counted event. A path counts only if it is still a file whose modified time
 * is later than the start and than the last time it counted: Windows can report a read. Throws, having closed
 * what it opened, if a root cannot be watched. After `onError`, called once, nothing more is called.
 */
export function watchFeatures(
  roots: string[],
  onSaved: (files: string[]) => void,
  onError: (err: Error) => void,
  quietMs = QUIET_MS,
): FeatureWatcher {
  const started = Date.now();
  const seen = new Map<string, number>();
  const pending = new Set<string>();
  const handles: FSWatcher[] = [];
  let timer: NodeJS.Timeout | undefined;

  const close = () => {
    for (const h of handles.splice(0)) h.close();
    clearTimeout(timer);
    pending.clear();
  };
  const fail = (err: Error) => {
    if (!handles.length) return;
    close();
    onError(err);
  };
  const flush = () => {
    const saved = [...pending].filter((p) => {
      let mtime: number;
      try {
        const st = statSync(p);
        if (!st.isFile()) return false;
        mtime = st.mtimeMs;
      } catch {
        return false; // deleted or renamed away
      }
      if (mtime <= Math.max(started, seen.get(p) ?? 0)) return false;
      seen.set(p, mtime);
      return true;
    });
    pending.clear();
    if (saved.length) onSaved(saved);
  };

  try {
    for (const root of roots) {
      const isFile = statSync(root).isFile();
      const h = watch(root, { recursive: !isFile }, (_event, name) => {
        if (name === null) return;
        // Windows reports the root's own path, in a loop, once the root is deleted or renamed
        if (path.isAbsolute(name)) return fail(new Error(`${root} was removed or renamed`));
        if (!isFile && !counts(name)) return;
        pending.add(isFile ? root : path.join(root, name));
        clearTimeout(timer);
        timer = setTimeout(flush, quietMs);
      });
      h.on("error", fail);
      handles.push(h);
    }
  } catch (err) {
    close();
    throw err;
  }
  return { close };
}
