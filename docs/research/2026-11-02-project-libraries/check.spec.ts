// Opens each sample project in the built app with a real Maven or Gradle, runs it, and says what it saw (README.md).
import { execFileSync } from "node:child_process";
import { cpSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { expect, launch, node, openProject, tempDir, test } from "../../../e2e/helpers";

const need = (name: string) => {
  const v = process.env[name];
  if (!v) throw new Error(`set ${name} (README.md)`);
  return v;
};
const MAVEN = need("CHECK_MAVEN_HOME");
const GRADLE = need("CHECK_GRADLE_HOME");
const JDK = need("CHECK_JDK");
const SAMPLES = path.join(__dirname, "samples");

/** Runs `file` in `cwd` with the JDK; throws with its output if it fails. */
const sh = (cwd: string, file: string, args: string[], env: NodeJS.ProcessEnv = {}) =>
  execFileSync(file, args, {
    cwd,
    env: { ...process.env, JAVA_HOME: JDK, ...env },
    encoding: "utf8",
    shell: file.endsWith(".cmd"),
  });

/** What the person's own build did before they open the project in Grace. */
function buildWorkSuite(dir: string) {
  // the company's repository, as its Nexus would serve acme-db 1.0
  const classes = path.join(tempDir("kk-acme-"), "classes");
  const javaFile = path.join(SAMPLES, "acme-db", "com", "acme", "db", "Db.java");
  sh(dir, path.join(JDK, "bin", "javac.exe"), ["-d", classes, "--release", "17", javaFile]);
  const repo = path.join(dir, "company-repo", "com", "acme", "acme-db", "1.0");
  mkdirSync(repo, { recursive: true });
  // a jar is a zip; System32's bsdtar makes one where a JDK ships no jar.exe (IntelliJ's)
  const tar = path.join(process.env.SystemRoot ?? "C:\\Windows", "System32", "tar.exe");
  sh(dir, tar, ["--format", "zip", "-c", "-f", path.join(repo, "acme-db-1.0.jar"), "-C", classes, "com"]);
  writeFileSync(
    path.join(repo, "acme-db-1.0.pom"),
    '<project xmlns="http://maven.apache.org/POM/4.0.0"><modelVersion>4.0.0</modelVersion>' +
      "<groupId>com.acme</groupId><artifactId>acme-db</artifactId><version>1.0</version></project>",
  );
  // the helper class, compiled as `mvn test` or the IDE would have
  sh(dir, path.join(MAVEN, "bin", "mvn.cmd"), ["-q", "-B", "test-compile"]);
}

const CASES: {
  sample: string;
  name?: string;
  scenarios: string[];
  before?: (dir: string) => void;
  /** The status each scenario ends with; "passed" unless the case shows a limit. */
  status?: "passed" | "failed";
}[] = [
  { sample: "karate1-maven", scenarios: ["insert and read a user", "call a local mock"] },
  { sample: "karate15-maven", scenarios: ["insert and read a user", "call a local mock"] },
  { sample: "karate1-gradle", scenarios: ["insert and read a user", "call a local mock"] },
  {
    sample: "work-suite",
    scenarios: ["add a user through the company library"],
    before: buildWorkSuite,
  },
  {
    // the limit: Grace runs the plain command, so a profile typed only as `mvn test -Pwork` never reaches it
    sample: "work-suite",
    name: "work-suite, profile on the command line only",
    scenarios: ["add a user through the company library"],
    before: (dir) => {
      buildWorkSuite(dir);
      rmSync(path.join(dir, ".mvn"), { recursive: true });
    },
    status: "failed",
  },
];

for (const c of CASES)
  test(c.name ?? c.sample, async () => {
    const dir = path.join(tempDir("kk-check-"), c.sample);
    cpSync(path.join(SAMPLES, c.sample), dir, { recursive: true });
    c.before?.(dir);
    const { app, win } = await launch(undefined, {
      PATH: [path.join(MAVEN, "bin"), path.join(GRADLE, "bin"), process.env.PATH].join(";"),
      JAVA_HOME: JDK,
    });
    const opened = Date.now();
    await openProject(app, dir);
    for (const s of c.scenarios) await expect(node(win, s)).toBeVisible({ timeout: 300_000 });
    console.log(`${c.name ?? c.sample}: listed and found in ${Date.now() - opened} ms`);
    await expect(win.getByTestId("notice")).toHaveCount(0);
    await win.getByTestId("run-all-button").click();
    for (const s of c.scenarios)
      await expect(node(win, s)).toHaveAttribute("data-status", c.status ?? "passed", {
        timeout: 120_000,
      });
    const printed = await win.getByTestId("console").innerText();
    // a second logging binding or a second Netty would say so here
    for (const word of ["SLF4J", "Multiple bindings", "ClassNotFound", "NoSuchMethod", "LinkageError"])
      expect(printed, word).not.toContain(word);
  });
