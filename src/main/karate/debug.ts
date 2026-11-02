// The main process's end of the step debugger's socket (docs/specs/2026-11-02-step-debugger.md, R2).
import { randomBytes } from "node:crypto";
import { createServer, type AddressInfo, type Server, type Socket } from "node:net";
import { createInterface } from "node:readline";
import type { Breakpoint, DebugCommand, DebugVar, VarKind } from "../../shared/events";

export const DEBUG_LISTENER = "grace.DebugListener";

/** A pause as the listener sends it: the step, the depth-0 test it belongs to, and the variables. */
export interface ListenerPause {
  file: string;
  line: number;
  depth: number;
  prefix: string;
  keyword: string;
  text: string;
  test: { file: string; refId: string };
  vars: DebugVar[];
}

const COMMANDS: readonly string[] = ["continue", "over", "into", "pause"];
const KINDS: readonly string[] = [
  "string",
  "number",
  "boolean",
  "null",
  "json",
  "xml",
  "function",
  "other",
];

export const isDebugCommand = (x: unknown): x is DebugCommand =>
  typeof x === "string" && COMMANDS.includes(x);

const str = (x: unknown): x is string => typeof x === "string";
const int = (x: unknown): x is number => Number.isInteger(x);

function parseVar(x: unknown): DebugVar {
  const { name, kind, type, value, cut } = (x ?? {}) as Record<string, unknown>;
  if (!str(name) || !str(kind) || !KINDS.includes(kind) || !str(value))
    throw new Error("bad variable");
  if ((type !== undefined && !str(type)) || (cut !== undefined && !int(cut)))
    throw new Error("bad variable");
  return {
    name,
    kind: kind as VarKind,
    value,
    ...(type === undefined ? {} : { type }),
    ...(cut === undefined ? {} : { cut }),
  };
}

/** Throws on anything but the listener's pause line. */
export function parsePause(json: string): ListenerPause {
  const p = JSON.parse(json) as Record<string, unknown>;
  const { file, line, depth, prefix, keyword, text, test, vars } = p ?? {};
  const t = (test ?? {}) as Record<string, unknown>;
  if (![file, prefix, keyword, text, t.file, t.refId].every(str) || !int(line) || !int(depth))
    throw new Error("bad pause");
  if (!Array.isArray(vars)) throw new Error("bad pause");
  return {
    file: file as string,
    line: line as number,
    depth: depth as number,
    prefix: prefix as string,
    keyword: keyword as string,
    text: text as string,
    test: { file: t.file as string, refId: t.refId as string },
    vars: vars.map(parseVar),
  };
}

/**
 * A loopback server for one debug run. It serves the first connection whose first line is the token, then stops
 * listening; any other connection is closed. A bad line from the listener drops it: the run goes on unpaused.
 */
export class DebugSession {
  private socket: Socket | null = null;
  private pauseFirst = false;
  private closed = false;

  private constructor(
    private readonly server: Server,
    readonly port: number,
    readonly token: string,
    private breakpoints: Breakpoint[],
    private readonly onPause: (p: ListenerPause) => void,
  ) {}

  static open(
    breakpoints: Breakpoint[],
    onPause: (p: ListenerPause) => void,
  ): Promise<DebugSession> {
    const token = randomBytes(16).toString("hex");
    const server = createServer();
    return new Promise((resolve, reject) => {
      server.once("error", reject);
      server.listen(0, "127.0.0.1", () => {
        const { port } = server.address() as AddressInfo;
        const session = new DebugSession(server, port, token, breakpoints, onPause);
        server.on("connection", (s) => session.accept(s));
        resolve(session);
      });
    });
  }

  private accept(s: Socket) {
    s.on("error", () => {}); // a reset when Java exits is no error: the run says how it ended
    if (this.socket || this.closed) return void s.destroy();
    const lines = createInterface({ input: s });
    lines.on("error", () => {});
    let first = true;
    lines.on("line", (line) => {
      if (first) {
        first = false;
        if (line !== this.token || this.socket || this.closed) return void s.destroy();
        this.socket = s;
        this.server.close();
        this.write({ breakpoints: this.breakpoints, pause: this.pauseFirst });
        return;
      }
      let pause: ListenerPause;
      try {
        pause = parsePause(line);
      } catch {
        return void s.destroy();
      }
      this.onPause(pause);
    });
  }

  setBreakpoints(b: Breakpoint[]): void {
    this.breakpoints = b;
    this.write({ breakpoints: b });
  }

  /** Before the listener connects, only a pause counts: it goes with the first line. */
  command(c: DebugCommand): void {
    if (this.socket) this.write({ command: c });
    else if (c === "pause") this.pauseFirst = true;
  }

  close(): void {
    this.closed = true;
    this.server.close();
    this.socket?.destroy();
  }

  private write(msg: object) {
    if (this.socket && !this.socket.destroyed) this.socket.write(`${JSON.stringify(msg)}\n`);
  }
}
