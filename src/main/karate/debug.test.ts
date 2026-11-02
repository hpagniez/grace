import { connect, type Socket } from "node:net";
import { createInterface } from "node:readline";
import { afterEach, describe, expect, it } from "vitest";
import { DebugSession, parsePause, type ListenerPause } from "./debug";

const PAUSE = {
  file: "debug.feature",
  line: 8,
  depth: 0,
  prefix: "*",
  keyword: "def",
  text: "user = { id: 7 }",
  test: { file: "debug.feature", refId: "[1:6]" },
  vars: [{ name: "name", kind: "string", value: "grace" }],
};

const sessions: DebugSession[] = [];
const sockets: Socket[] = [];
afterEach(() => {
  sessions.splice(0).forEach((s) => s.close());
  sockets.splice(0).forEach((s) => s.destroy());
});

async function open(onPause: (p: ListenerPause) => void = () => {}) {
  const s = await DebugSession.open([{ file: "debug.feature", line: 8 }], onPause);
  sessions.push(s);
  return s;
}

/** A client standing in for the Java listener: its lines, as they arrive. */
function client(port: number) {
  const socket = connect(port, "127.0.0.1");
  sockets.push(socket);
  socket.on("error", () => {});
  const lines: string[] = [];
  createInterface({ input: socket }).on("line", (l) => lines.push(l));
  const closed = new Promise<void>((r) => socket.on("close", () => r()));
  return { socket, lines, closed, send: (l: string) => socket.write(`${l}\n`) };
}

async function until(check: () => boolean) {
  for (let i = 0; i < 100 && !check(); i++) await new Promise((r) => setTimeout(r, 20));
  expect(check()).toBe(true);
}

describe("DebugSession", () => {
  it("sends the breakpoints and a pause asked before the listener connected", async () => {
    const s = await open();
    s.command("pause");
    s.command("over"); // ignored: nothing is paused yet
    const c = client(s.port);
    c.send(s.token);
    await until(() => c.lines.length === 1);
    expect(JSON.parse(c.lines[0])).toEqual({
      breakpoints: [{ file: "debug.feature", line: 8 }],
      pause: true,
    });
  });

  it("closes a connection with a wrong token and serves the right one", async () => {
    const s = await open();
    const bad = client(s.port);
    bad.send("not the token");
    await bad.closed;
    const good = client(s.port);
    good.send(s.token);
    await until(() => good.lines.length === 1);
  });

  it("stops listening once the listener is connected", async () => {
    const s = await open();
    const c = client(s.port);
    c.send(s.token);
    await until(() => c.lines.length === 1);
    const late = connect(s.port, "127.0.0.1");
    sockets.push(late);
    const code = await new Promise((r) =>
      late.on("error", (e: NodeJS.ErrnoException) => r(e.code)),
    );
    expect(code).toBe("ECONNREFUSED");
  });

  it("passes a pause on and writes commands and breakpoints", async () => {
    const paused: ListenerPause[] = [];
    const s = await open((p) => paused.push(p));
    const c = client(s.port);
    c.send(s.token);
    await until(() => c.lines.length === 1);
    c.send(JSON.stringify(PAUSE));
    await until(() => paused.length === 1);
    expect(paused[0]).toEqual(PAUSE);
    s.command("over");
    s.setBreakpoints([]);
    await until(() => c.lines.length === 3);
    expect(c.lines.slice(1).map((l) => JSON.parse(l))).toEqual([
      { command: "over" },
      { breakpoints: [] },
    ]);
  });

  it("drops a listener that sends a bad line", async () => {
    const paused: ListenerPause[] = [];
    const s = await open((p) => paused.push(p));
    const c = client(s.port);
    c.send(s.token);
    c.send(JSON.stringify({ file: 1 }));
    await c.closed;
    expect(paused).toEqual([]);
    expect(() => s.command("continue")).not.toThrow();
  });

  it("survives the listener resetting the connection, and says it ended", async () => {
    let ended = 0;
    const s = await DebugSession.open(
      [],
      () => {},
      () => ended++,
    );
    sessions.push(s);
    const c = client(s.port);
    c.send(s.token);
    await until(() => c.lines.length === 1);
    c.socket.resetAndDestroy();
    await until(() => ended === 1);
    expect(() => s.setBreakpoints([])).not.toThrow();
  });
});

describe("parsePause", () => {
  it("keeps a cut value and the type of another kind", () => {
    const vars = [
      { name: "big", kind: "string", value: "x", cut: 36_864 },
      { name: "d", kind: "other", type: "LocalDate", value: "2026-11-02" },
    ];
    expect(parsePause(JSON.stringify({ ...PAUSE, vars })).vars).toEqual(vars);
  });

  it("refuses a pause without variables or with an unknown kind", () => {
    expect(() => parsePause(JSON.stringify({ ...PAUSE, vars: undefined }))).toThrow();
    const odd = [{ name: "a", kind: "date", value: "x" }];
    expect(() => parsePause(JSON.stringify({ ...PAUSE, vars: odd }))).toThrow();
  });
});
