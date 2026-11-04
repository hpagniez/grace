import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { NO_POM, pomStart, readPom } from "./pom";

const dirs: string[] = [];
function project(pom?: string, pomXml?: string): string {
  const d = mkdtempSync(path.join(tmpdir(), "kk-pom-"));
  dirs.push(d);
  if (pom !== undefined) writeFileSync(path.join(d, "karate-pom.json"), pom);
  if (pomXml !== undefined) writeFileSync(path.join(d, "pom.xml"), pomXml);
  return d;
}
afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true });
});

const broken = (error: string) => ({
  paths: null,
  configDir: null,
  junitXml: false,
  env: undefined,
  filter: null,
  threads: null,
  unused: [],
  error,
});

describe("readPom", () => {
  it("is empty without a file", () => {
    expect(readPom(project())).toEqual(NO_POM);
  });
  it("keeps paths and configDir as written and lists the other keys in file order", () => {
    const pom = '{"paths":["tests/a"],"configDir":"conf","env":"qa","dryRun":true}';
    expect(readPom(project(pom))).toEqual({
      paths: ["tests/a"],
      configDir: "conf",
      junitXml: false,
      env: "qa",
      filter: null,
      threads: null,
      unused: ["dryRun"],
      error: null,
    });
  });
  it("reads an empty paths list as none", () => {
    expect(readPom(project('{"paths":[]}')).paths).toBeNull();
  });
  it("gives the parser's message for a file that is not JSON", () => {
    const pom = readPom(project("{"));
    expect(pom).toMatchObject({ paths: null, configDir: null, unused: [] });
    expect(pom.error).toMatch(/\S/);
  });
  it("refuses a file that is not an object", () => {
    for (const text of ["null", "[]", "3"])
      expect(readPom(project(text))).toEqual(broken("not a JSON object"));
  });
  it("refuses paths that are not a list of strings", () => {
    for (const text of ['{"paths":"tests"}', '{"paths":[1]}'])
      expect(readPom(project(text))).toEqual(broken("paths is not a list of strings"));
  });
  it("refuses a configDir that is not a string", () => {
    expect(readPom(project('{"configDir":3}'))).toEqual(broken("configDir is not a string"));
  });
  it("takes output.junitXml and names output's other keys", () => {
    const pom = '{"env":"qa","output":{"dir":"x","junitXml":true,"html":false}}';
    expect(readPom(project(pom))).toMatchObject({
      junitXml: true,
      unused: ["output.dir", "output.html"],
    });
  });
  it("reads junitXml false or absent as off", () => {
    expect(readPom(project('{"output":{"junitXml":false}}'))).toMatchObject({
      junitXml: false,
      unused: [],
    });
    expect(readPom(project('{"output":{}}'))).toMatchObject({ junitXml: false, unused: [] });
  });
  it("names an output that is not an object", () => {
    for (const text of ['{"output":"x"}', '{"output":[true]}', '{"output":null}'])
      expect(readPom(project(text))).toMatchObject({ junitXml: false, unused: ["output"] });
  });
  it("refuses a junitXml that is not true or false", () => {
    expect(readPom(project('{"output":{"junitXml":"yes"}}'))).toEqual(
      broken("output.junitXml is not true or false"),
    );
  });

  it("takes env, tags and threads", () => {
    expect(readPom(project('{"env":"qa","tags":["@smoke","~@slow"],"threads":3}'))).toMatchObject({
      env: "qa",
      filter: { include: ["smoke"], exclude: ["slow"] },
      threads: 3,
      unused: [],
    });
  });
  it.each([
    [["@smoke"], ["smoke"], []],
    [["~@slow"], [], ["slow"]],
    [["@a,@b"], ["a", "b"], []],
    [
      [" @a , @b ", "~@c", "~@d"],
      ["a", "b"],
      ["c", "d"],
    ],
  ])("shows the tag entries %j as chips", (tags, include, exclude) => {
    expect(readPom(project(JSON.stringify({ tags })))).toMatchObject({
      filter: { include, exclude },
      unused: [],
    });
  });
  it.each([
    [["anyOf('@a','@b')"], null, [`tags "anyOf('@a','@b')"`]],
    [["@a", "@b"], { include: ["a"], exclude: [] }, ['tags "@b"']],
    [["~@a,~@b"], null, ['tags "~@a,~@b"']],
    [["@a", "~@a"], { include: ["a"], exclude: [] }, ['tags "~@a"']],
    ["@a", null, ['tags "@a"']],
  ])("names the tag entries of %j it cannot show", (tags, filter, unused) => {
    expect(readPom(project(JSON.stringify({ tags })))).toMatchObject({ filter, unused });
  });
  it("names an env or a thread count Grace cannot take", () => {
    expect(readPom(project('{"env":"x y"}'))).toMatchObject({ env: null, unused: ['env "x y"'] });
    expect(readPom(project('{"threads":64}'))).toMatchObject({
      threads: null,
      unused: ["threads 64"],
    });
    expect(readPom(project('{"threads":"4"}'))).toMatchObject({
      threads: null,
      unused: ['threads "4"'],
    });
  });
  it("reads an empty env as none given", () => {
    expect(readPom(project('{"env":""}'))).toMatchObject({ env: undefined, unused: [] });
  });
});

const MAVEN_DEV = "<project><properties><karate.env>dev</karate.env></properties></project>";

describe("pomStart", () => {
  it("takes pom.xml's karate.env without a karate-pom.json", () => {
    expect(pomStart(project(undefined, MAVEN_DEV))).toEqual({
      env: "dev",
      filter: null,
      threads: null,
    });
  });
  it("puts karate-pom.json's env first", () => {
    expect(pomStart(project('{"env":"qa"}', MAVEN_DEV)).env).toBe("qa");
  });
  it("keeps pom.xml out when karate-pom.json's env is unused", () => {
    expect(pomStart(project('{"env":"x y"}', MAVEN_DEV)).env).toBeNull();
  });
  it("reads a karate-pom.json that cannot be read as absent", () => {
    expect(pomStart(project("not json", MAVEN_DEV)).env).toBe("dev");
  });
  it("takes the filter and the thread count", () => {
    expect(pomStart(project('{"tags":["~@slow"],"threads":2}'))).toEqual({
      env: null,
      filter: { include: [], exclude: ["slow"] },
      threads: 2,
    });
  });
});
