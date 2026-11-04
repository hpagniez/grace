import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { mavenEnv } from "./maven";

const dirs: string[] = [];
/** A project folder whose pom.xml holds `inner` inside `<project>`; no pom.xml when undefined. */
function project(inner?: string, head = "<project>"): string {
  const d = mkdtempSync(path.join(tmpdir(), "kk-maven-"));
  dirs.push(d);
  if (inner !== undefined)
    writeFileSync(path.join(d, "pom.xml"), `<?xml version="1.0"?>\n${head}${inner}</project>\n`);
  return d;
}
afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true });
});

const property = (name: string, value: string) =>
  `<properties><${name}>${value}</${name}></properties>`;
const plugin = (id: string, entry: string) =>
  `<plugin><groupId>org.apache.maven.plugins</groupId><artifactId>${id}</artifactId>` +
  `<configuration><systemPropertyVariables><karate.env>${entry}</karate.env></systemPropertyVariables></configuration></plugin>`;
const surefire = (entry: string, where: "plugins" | "pluginManagement" = "plugins") => {
  const plugins = `<plugins>${plugin("maven-surefire-plugin", entry)}</plugins>`;
  return `<build>${where === "plugins" ? plugins : `<pluginManagement>${plugins}</pluginManagement>`}</build>`;
};

describe("mavenEnv", () => {
  it("takes the karate.env property alone", () => {
    expect(mavenEnv(project(property("karate.env", "dev")))).toBe("dev");
  });
  it("takes Surefire's entry as written", () => {
    expect(mavenEnv(project(surefire("qa")))).toBe("qa");
  });
  it("resolves Surefire's ${karate.env} from the properties", () => {
    expect(mavenEnv(project(property("karate.env", "dev") + surefire("${karate.env}")))).toBe(
      "dev",
    );
  });
  it("resolves one ${…} inside other text", () => {
    expect(mavenEnv(project(property("region", "eu") + surefire("qa-${region}")))).toBe("qa-eu");
  });
  it("gives none when Surefire's property is missing, without falling back", () => {
    expect(mavenEnv(project(property("karate.env", "dev") + surefire("${missing}")))).toBeNull();
  });
  it("takes Failsafe's entry", () => {
    const failsafe = `<build><plugins>${plugin("maven-failsafe-plugin", "${karate.env}")}</plugins></build>`;
    expect(mavenEnv(project(property("karate.env", "dev") + failsafe))).toBe("dev");
  });
  it("takes Surefire's entry in pluginManagement", () => {
    expect(mavenEnv(project(surefire("qa", "pluginManagement")))).toBe("qa");
  });
  it("puts Surefire before Failsafe", () => {
    const both = `<build><plugins>${plugin("maven-failsafe-plugin", "uat")}${plugin("maven-surefire-plugin", "qa")}</plugins></build>`;
    expect(mavenEnv(project(both))).toBe("qa");
  });
  it("leaves out an execution's configuration", () => {
    const execution =
      "<build><plugins><plugin><artifactId>maven-surefire-plugin</artifactId><executions><execution>" +
      "<configuration><systemPropertyVariables><karate.env>qa</karate.env></systemPropertyVariables></configuration>" +
      "</execution></executions></plugin></plugins></build>";
    expect(mavenEnv(project(execution))).toBeNull();
  });
  it("leaves out a profile's properties", () => {
    expect(
      mavenEnv(project(`<profiles><profile>${property("karate.env", "dev")}</profile></profiles>`)),
    ).toBeNull();
  });
  it("leaves out a comment", () => {
    expect(mavenEnv(project(`<!-- ${property("karate.env", "dev")} -->`))).toBeNull();
  });
  it("gives none for a name Grace does not take", () => {
    expect(mavenEnv(project(property("karate.env", "x y")))).toBeNull();
  });
  it("gives none when elements are not well nested", () => {
    expect(mavenEnv(project(`<properties><karate.env>dev</properties></karate.env>`))).toBeNull();
  });
  it("gives none for more than one ${", () => {
    expect(
      mavenEnv(project(property("a", "x") + property("b", "y") + surefire("${a}${b}"))),
    ).toBeNull();
  });
  it("gives none when the resolved value still holds ${", () => {
    expect(
      mavenEnv(project(property("karate.env", "${other}") + surefire("${karate.env}"))),
    ).toBeNull();
  });
  it("lets an empty Surefire entry decide", () => {
    expect(mavenEnv(project(property("karate.env", "dev") + surefire("")))).toBeNull();
  });
  it("reads a namespaced project with a BOM and Windows line ends", () => {
    const d = project();
    writeFileSync(
      path.join(d, "pom.xml"),
      '﻿<?xml version="1.0" encoding="UTF-8"?>\r\n' +
        '<project xmlns="http://maven.apache.org/POM/4.0.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">\r\n' +
        "  <properties>\r\n    <karate.env>dev</karate.env>\r\n  </properties>\r\n</project>\r\n",
    );
    expect(mavenEnv(d)).toBe("dev");
  });
  it("reads CDATA as text", () => {
    expect(mavenEnv(project(property("karate.env", "<![CDATA[dev]]>")))).toBe("dev");
  });
  it("gives none without a pom.xml", () => {
    expect(mavenEnv(project())).toBeNull();
  });
});
