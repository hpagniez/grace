// Watch mode (docs/specs/2026-11-03-watch-mode.md, R2): the OS's file events on the feature roots, never a poll.
import { existsSync, readdirSync, statSync, watch, type FSWatcher } from "node:fs";
import path from "node:path";
import { SKIPPED_DIRS } from "./fingerprint";

/** How long no counted event must come before a batch of saves is handed over (R2.3). */
export const QUIET_MS = 300;

/** A path as `fs.watch` gives it, relative to its root: a `.feature` outside the skipped folders (R2.2). */
export function counts(rel: string): boolean {
  const parts = rel.toLowerCase().split(/[\\/]/);
  return parts.at(-1)!.endsWith(".feature") && !parts.some((p) => SKIPPED_DIRS.includes(p));
}

/** Every feature under `dir` outside the skipped folders; symbolic links and junctions are not followed. */
function features(dir: string): string[] {
  try {
    return readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
      const p = path.join(dir, e.name);
      if (e.isDirectory()) return SKIPPED_DIRS.includes(e.name.toLowerCase()) ? [] : features(p);
      return e.isFile() && e.name.toLowerCase().endsWith(".feature") ? [p] : [];
    });
  } catch {
    return []; // a folder removed meanwhile
  }
}

export interface FeatureWatcher {
  close(): void;
}

/**
 * Watches each root, a folder recursively or a single file, and hands over the absolute paths of the features
 * saved once `quietMs` passed with no counted event. A path counts only if it is still a file whose modified time
 * is later than the start and than the last time it counted: Windows can report a read. A burst too big for
 * Windows' change buffer (a git checkout) loses its names: the batch then takes every feature of the folder roots
 * that this check keeps. Each folder root's parent is watched too, without its subfolders, as a renamed or moved
 * root reports nothing itself. Throws, having closed what it opened, if a root cannot be watched. After
 * `onError`, called once, nothing more is called.
 */
export function watchFeatures(
  roots: string[],
  onSaved: (files: string[]) => void,
  onError: (err: Error) => void,
  quietMs = QUIET_MS,
): FeatureWatcher {
  // Windows stamps a file from a clock that ticks every 15.6 ms (2 s on FAT): a save just after the start may
  // look older than it. A feature changed in the slack before the start can count once on a read; harmless.
  const started = Date.now() - 2000;
  const seen = new Map<string, number>();
  const pending = new Set<string>();
  const handles: FSWatcher[] = [];
  let timer: NodeJS.Timeout | undefined;
  let overflow = false;

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
  const later = () => {
    clearTimeout(timer);
    timer = setTimeout(flush, quietMs);
  };
  const flush = () => {
    // shortcut: a walk of the folder roots, on the main thread, only after an overflow; make it asynchronous if
    // a project's walk ever takes long
    if (overflow)
      for (const r of roots) if (!isFile.get(r)) for (const f of features(r)) pending.add(f);
    overflow = false;
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

  const isFile = new Map<string, boolean>();
  try {
    for (const root of roots) {
      const file = statSync(root).isFile();
      isFile.set(root, file);
      const gone = () => fail(new Error(`${root} was removed or renamed`));
      const h = watch(root, { recursive: !file }, (_event, name) => {
        if (name === null) {
          overflow = true; // libuv's sign that Windows' buffer overflowed
          return later();
        }
        // Windows reports the root's own path, in a loop, once the root is deleted
        if (path.isAbsolute(name)) return gone();
        if (!file && !counts(name)) return;
        pending.add(file ? root : path.join(root, name));
        later();
      });
      h.on("error", fail);
      handles.push(h);
      const parent = path.dirname(root);
      if (file || parent === root) continue; // a drive's root has no parent
      const base = path.basename(root).toLowerCase();
      const p = watch(parent, (_event, name) => {
        if (name?.toLowerCase() === base && !existsSync(root)) gone();
      });
      p.on("error", fail);
      handles.push(p);
    }
  } catch (err) {
    close();
    throw err;
  }
  return { close };
}
