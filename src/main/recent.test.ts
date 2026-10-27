import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AppEvent } from "../shared/events";
import { checkRecent, recentSender } from "./recent";

const dirs: string[] = [];
const tmp = () => {
  const d = mkdtempSync(path.join(tmpdir(), "kk-recent-"));
  dirs.push(d);
  return d;
};

afterEach(() => {
  vi.useRealTimers();
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true });
});

describe("checkRecent", () => {
  it("checks each folder", async () => {
    const dir = tmp();
    const missing = path.join(dir, "missing");
    expect(await checkRecent([missing, dir])).toEqual([
      { path: missing, exists: false },
      { path: dir, exists: true },
    ]);
  });

  it("a file or a failing stat counts as not found", async () => {
    const file = path.join(tmp(), "a.txt");
    writeFileSync(file, "x");
    expect(await checkRecent([file])).toEqual([{ path: file, exists: false }]);

    const denied = () => Promise.reject(Object.assign(new Error("EPERM"), { code: "EPERM" }));
    expect(await checkRecent(["D:\\a"], denied)).toEqual([{ path: "D:\\a", exists: false }]);
  });

  it("a folder that does not answer within 1 s counts as existing", async () => {
    vi.useFakeTimers();
    const never = () => new Promise<never>(() => {});
    const result = checkRecent(["\\\\gone\\a", "\\\\gone\\b"], never);
    await vi.advanceTimersByTimeAsync(1000);
    expect(await result).toEqual([
      { path: "\\\\gone\\a", exists: true },
      { path: "\\\\gone\\b", exists: true },
    ]);
  });
});

describe("recentSender", () => {
  it("emits the checked list", async () => {
    const dir = tmp();
    const events: AppEvent[] = [];
    await recentSender(
      () => [dir],
      (e) => events.push(e),
    )();
    expect(events).toEqual([{ type: "recent", entries: [{ path: dir, exists: true }] }]);
  });

  it("drops a list whose checks end after a newer call's", async () => {
    const lists = [["D:\\a"], []];
    let release!: () => void;
    const gate = new Promise<{ isDirectory(): boolean }>((resolve) => {
      release = () => resolve({ isDirectory: () => true });
    });
    const events: AppEvent[] = [];
    const send = recentSender(
      () => lists.shift() ?? [],
      (e) => events.push(e),
      () => gate,
    );
    const first = send();
    await send();
    release();
    await first;
    expect(events).toEqual([{ type: "recent", entries: [] }]);
  });
});
