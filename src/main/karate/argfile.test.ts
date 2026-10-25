import {
  cpSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
  existsSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { copyFixture } from "../../../test/fixtures/copy";
import { vendorPaths } from "../vendor";
import { argFileText, removeLeftoverArgFiles, writeArgFile, writeTargetFile } from "./argfile";
import { buildArgs } from "./command";
import { readAllEvents } from "./events";
import { spawnKarate } from "./process";

const vendor = vendorPaths(path.resolve("vendor"));
const temps: string[] = [];
const tmp = () => {
  const d = mkdtempSync(path.join(tmpdir(), "kk-arg-"));
  temps.push(d);
  return d;
};
afterEach(() => {
  for (const d of temps.splice(0)) rmSync(d, { recursive: true, force: true, maxRetries: 5 });
});

const TRICKY = String.raw`x y "q" C:\Users\me %P% $x 'z'`;

describe("argFileText", () => {
  it("escapes backslashes and quotes", () => {
    const a = String.raw`x y "q" C:\Users\me %P%`;
    expect(argFileText([{ name: "a", value: a }])).toBe(
      String.raw`"-Da=x y \"q\" C:\\Users\\me %P%"` + "\r\n",
    );
  });
  it("writes one CRLF-ended line per row, and nothing for no rows", () => {
    expect(
      argFileText([
        { name: "a", value: "1" },
        { name: "b.c", value: "" },
      ]),
    ).toBe('"-Da=1"\r\n"-Db.c="\r\n');
    expect(argFileText([])).toBe("");
  });
});

describe("writeArgFile", () => {
  it("writes the text as latin1", () => {
    const file = path.join(tmp(), "run-args.txt");
    writeArgFile(file, [{ name: "a", value: TRICKY }]);
    expect(readFileSync(file).toString("latin1")).toBe(argFileText([{ name: "a", value: TRICKY }]));
  });
});

describe("writeTargetFile", () => {
  it("quotes, doubles backslashes and writes UTF-8", () => {
    const file = path.join(tmp(), "run-targets.txt");
    const targets = [String.raw`C:\Users\hé llo\a.feature:3:15`, String.raw`D:\p "q"\b.feature:7`];
    writeTargetFile(file, targets);
    const text =
      String.raw`"C:\\Users\\hé llo\\a.feature:3:15"` +
      "\r\n" +
      String.raw`"D:\\p \"q\"\\b.feature:7"` +
      "\r\n";
    expect(readFileSync(file).equals(Buffer.from(text, "utf8"))).toBe(true);
  });
  it("writes an empty file for no targets", () => {
    const file = path.join(tmp(), "run-targets.txt");
    writeTargetFile(file, []);
    expect(readFileSync(file)).toHaveLength(0);
  });
});

describe("removeLeftoverArgFiles", () => {
  it("also deletes run-targets.txt", () => {
    const dataDir = tmp();
    const project = path.join(dataDir, "projects", "h1");
    mkdirSync(project, { recursive: true });
    for (const f of ["run-args.txt", "run-targets.txt", "env.json"])
      writeFileSync(path.join(project, f), "x");
    removeLeftoverArgFiles(dataDir);
    expect(existsSync(path.join(project, "run-args.txt"))).toBe(false);
    expect(existsSync(path.join(project, "run-targets.txt"))).toBe(false);
    expect(existsSync(path.join(project, "env.json"))).toBe(true);
  });
  it("deletes every run-args.txt and nothing else", () => {
    const dataDir = tmp();
    for (const p of ["h1", "h2", "h3"])
      mkdirSync(path.join(dataDir, "projects", p), { recursive: true });
    for (const p of ["h1", "h2"])
      writeFileSync(path.join(dataDir, "projects", p, "run-args.txt"), "x");
    writeFileSync(path.join(dataDir, "projects", "h2", "env.json"), "{}");
    writeFileSync(path.join(dataDir, "settings.json"), "{}");
    removeLeftoverArgFiles(dataDir);
    expect(existsSync(path.join(dataDir, "projects", "h1", "run-args.txt"))).toBe(false);
    expect(existsSync(path.join(dataDir, "projects", "h2", "run-args.txt"))).toBe(false);
    expect(existsSync(path.join(dataDir, "projects", "h2", "env.json"))).toBe(true);
    expect(existsSync(path.join(dataDir, "settings.json"))).toBe(true);
  });
  it("never throws, whatever is there", () => {
    const dataDir = tmp();
    expect(() => removeLeftoverArgFiles(path.join(dataDir, "missing"))).not.toThrow();
    mkdirSync(path.join(dataDir, "projects", "h1", "run-args.txt"), { recursive: true }); // a folder
    writeFileSync(path.join(dataDir, "projects", "file"), "x"); // a file where a folder is expected
    expect(() => removeLeftoverArgFiles(dataDir)).not.toThrow();
  });
});

describe("argument file through Java", () => {
  it("hands every value to karate.properties unchanged", { timeout: 120_000 }, async () => {
    const root = tmp();
    writeFileSync(
      path.join(root, "p.feature"),
      [
        "Feature: argument file",
        "Scenario: print the run properties",
        "* print 'A<' + karate.properties['a'] + '>'",
        "* print 'B<' + karate.properties['b'] + '>'",
        "",
      ].join("\n"),
    );
    const argFile = path.join(tmp(), "run-args.txt");
    writeArgFile(argFile, [
      { name: "a", value: TRICKY },
      { name: "b", value: "plain" },
    ]);
    const args = buildArgs({
      jar: vendor.jar,
      classpath: [root],
      root,
      outDir: tmp(),
      dryRun: false,
      tags: [],
      targets: [root],
      env: null,
      argFile,
    });
    const lines: string[] = [];
    expect(await spawnKarate(vendor.java, args, root, (l) => lines.push(...l)).exit).toBe(0);
    const printed = (key: string) =>
      lines.map((l) => l.match(new RegExp(`${key}<(.*)>$`))?.[1]).find((v) => v !== undefined);
    expect(printed("A")).toBe(TRICKY);
    expect(printed("B")).toBe("plain");
  });
});

describe("target file through Java", () => {
  it(
    "runs exactly the targets of the file, in a path with a space and é",
    { timeout: 120_000 },
    async () => {
      const root = mkdtempSync(path.join(tmpdir(), "kk hist é-"));
      temps.push(root);
      cpSync(copyFixture("edge"), root, { recursive: true });
      // the rows `| 1 |` and `| 3 |` of the outline `add <a>` (rows.feature lines 10 and 13)
      const targetFile = path.join(tmp(), "run-targets.txt");
      writeTargetFile(targetFile, [`${path.join(root, "tests", "rows.feature")}:10:13`]);
      const outDir = tmp();
      const args = buildArgs({
        jar: vendor.jar,
        classpath: [root],
        root,
        outDir,
        dryRun: false,
        tags: [],
        targets: [`@${targetFile}`],
        env: null,
        argFile: null,
      });
      expect(await spawnKarate(vendor.java, args, root, () => {}).exit).toBe(0);
      const ran = readAllEvents(path.join(outDir, "karate-json", "karate-events.jsonl"))
        .filter((e) => e.type === "SCENARIO_EXIT" && e.data.callDepth === 0)
        .map((e) => e.data.refId);
      expect(ran).toEqual(["[1.1:7]", "[1.3:7]"]); // rows 1 and 3, not 2 (refIds carry the Examples line)
    },
  );
});
