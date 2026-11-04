import { mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { parseProperties, readPropertiesFile } from "./propertiesFile";

const entries = (lines: string[]) =>
  Object.fromEntries(parseProperties(lines.join("\n")).map((e) => [e.key, e.value]));

describe("parseProperties", () => {
  it("reads keys the way java.util.Properties.load does", () => {
    const text = [
      "# a comment",
      "! another",
      "",
      "   ",
      "a=1",
      "b : 2",
      "c 3",
      "d=x\\",
      "  y",
      "e=\\u0041",
      "t=a\\tb",
      "k\\=x=1",
      "  spaced   =   value  ",
      "empty=",
      "bare",
    ];
    expect(entries(text)).toEqual({
      a: "1",
      b: "2",
      c: "3",
      d: "xy",
      e: "A",
      t: "a\tb",
      "k=x": "1",
      spaced: "value  ",
      empty: "",
      bare: "",
    });
  });

  it("keeps a repeated key's last value at its first place", () => {
    expect(parseProperties("a=1\nb=2\na=3")).toEqual([
      { key: "a", value: "3" },
      { key: "b", value: "2" },
    ]);
  });

  it("drops a continuation at the end of the file", () => {
    expect(entries(["f=1\\"])).toEqual({ f: "1" });
  });

  it("does not continue a comment line, and splits on any line ending", () => {
    expect(parseProperties("# note \\\r\na=1\rb=2\r\nc=3")).toEqual([
      { key: "a", value: "1" },
      { key: "b", value: "2" },
      { key: "c", value: "3" },
    ]);
  });

  it("keeps an escaped backslash at a line's end without continuing", () => {
    expect(entries(["p=C:\\\\", "q=1"])).toEqual({ p: "C:\\", q: "1" });
  });

  it("refuses a malformed \\uXXXX escape with the line its property starts on", () => {
    expect(() => parseProperties("a=1\nk=\\u00G1")).toThrow("bad \\uXXXX escape on line 2");
    expect(() => parseProperties("k=\\u12")).toThrow("bad \\uXXXX escape on line 1");
  });
});

describe("readPropertiesFile", () => {
  const dirs: string[] = [];
  afterEach(() => {
    for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true });
  });
  const file = (name: string, content: Buffer | string) => {
    const d = mkdtempSync(path.join(tmpdir(), "kk-props-"));
    dirs.push(d);
    const f = path.join(d, name);
    writeFileSync(f, content);
    return f;
  };

  it("gives the file's name and keys, and leaves the file as it was", () => {
    const f = file("secrets-dev.properties", "db.user=u\ndb.password=p\n");
    const before = statSync(f).mtimeMs;
    expect(readPropertiesFile(f)).toEqual({
      file: "secrets-dev.properties",
      entries: [
        { key: "db.user", value: "u" },
        { key: "db.password", value: "p" },
      ],
    });
    expect(readFileSync(f, "utf8")).toBe("db.user=u\ndb.password=p\n");
    expect(statSync(f).mtimeMs).toBe(before);
  });

  it("drops a UTF-8 byte order mark and reads the rest as ISO-8859-1", () => {
    const f = file(
      "s.properties",
      Buffer.concat([Buffer.from([0xef, 0xbb, 0xbf]), Buffer.from("db.user=u\nw=\xe9", "latin1")]),
    );
    expect(readPropertiesFile(f)).toEqual({
      file: "s.properties",
      entries: [
        { key: "db.user", value: "u" },
        { key: "w", value: "é" },
      ],
    });
  });

  it("refuses a file over 1 MB", () => {
    const f = file("big.properties", Buffer.alloc(1024 * 1024 + 1, "a"));
    expect(readPropertiesFile(f)).toEqual({ error: "big.properties is too large to import" });
  });

  it("says why a file cannot be read", () => {
    const f = path.join(tmpdir(), "kk-props-missing", "gone.properties");
    const r = readPropertiesFile(f);
    expect(r).toMatchObject({
      error: expect.stringMatching(/^gone\.properties could not be read: /),
    });
  });

  it("says a file has no properties", () => {
    const f = file("notes.properties", "# only a comment\n\n");
    expect(readPropertiesFile(f)).toEqual({ error: "notes.properties has no properties" });
  });

  it("refuses a malformed escape without a value in the message", () => {
    const f = file("s.properties", "secret=hunter2\\u00G1");
    const r = readPropertiesFile(f);
    expect(r).toEqual({
      error: "s.properties could not be imported: bad \\uXXXX escape on line 1",
    });
  });
});
