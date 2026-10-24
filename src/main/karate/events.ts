import fs from "node:fs";

export interface KarateEvent {
  type: string;
  timeStamp: number;
  threadId: string | null;
  data: Record<string, any>;
}

const NEWLINE = 0x0a;

/** Reads new complete lines of a growing JSONL file. A partial last line waits for its newline. */
export class JsonlTail {
  private offset = 0;
  private lineNo = 0;

  constructor(private readonly file: string) {}

  read(): KarateEvent[] {
    let fd: number;
    try {
      fd = fs.openSync(this.file, "r");
    } catch (err) {
      if ((err as NodeJS.ErrnoException).code === "ENOENT") return [];
      throw err;
    }
    let buf: Buffer;
    try {
      buf = Buffer.alloc(Math.max(0, fs.fstatSync(fd).size - this.offset));
      fs.readSync(fd, buf, 0, buf.length, this.offset);
    } finally {
      fs.closeSync(fd);
    }
    // Cut at the last newline on bytes, so a split multi-byte character is never decoded early.
    const end = buf.lastIndexOf(NEWLINE);
    if (end < 0) return [];
    this.offset += end + 1;
    const events: KarateEvent[] = [];
    for (const raw of buf.subarray(0, end).toString("utf8").split("\n")) {
      this.lineNo++;
      const text = raw.trim();
      if (!text) continue;
      try {
        events.push(JSON.parse(text));
      } catch (err) {
        throw new Error(
          `${this.file}:${this.lineNo}: invalid JSON event (${(err as Error).message})`,
        );
      }
    }
    return events;
  }
}

export function readAllEvents(file: string): KarateEvent[] {
  return new JsonlTail(file).read();
}
