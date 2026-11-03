import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { NO_POM, readPom } from "./pom";

const dirs: string[] = [];
function project(pom?: string): string {
  const d = mkdtempSync(path.join(tmpdir(), "kk-pom-"));
  dirs.push(d);
  if (pom !== undefined) writeFileSync(path.join(d, "karate-pom.json"), pom);
  return d;
}
afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true });
});

const broken = (error: string) => ({
  paths: null,
  configDir: null,
  junitXml: false,
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
      unused: ["env", "dryRun"],
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
      unused: ["env", "output.dir", "output.html"],
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
});
