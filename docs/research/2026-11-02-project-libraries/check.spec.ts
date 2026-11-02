// Opens each sample project in the built app with a real Maven or Gradle, runs it, and says what it saw (README.md).
import { execFileSync } from "node:child_process";
import { cpSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import {
  expect,
  launch,
  node,
  openProject,
  projectDataDir,
  tempDir,
  test,
} from "../../../e2e/helpers";

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
const sh = (cwd: string, file: string, args: string[]) =>
  execFileSync(file, args, {
    cwd,
    env: { ...process.env, JAVA_HOME: JDK },
    encoding: "utf8",
    shell: file.endsWith(".cmd"),
  });

/** What `mvn test` or the IDE does before the person opens the project: compiles src/test/java. */
const testCompile = (dir: string) =>
  sh(dir, path.join(MAVEN, "bin", "mvn.cmd"), ["-q", "-B", "test-compile"]);

/** The work suite's company repository, as its Nexus would serve acme-db 1.0, then its own build. */
function buildWorkSuite(dir: string) {
  const classes = path.join(tempDir("kk-acme-"), "classes");
  const javaFile = path.join(SAMPLES, "acme-db", "com", "acme", "db", "Db.java");
  sh(dir, path.join(JDK, "bin", "javac.exe"), ["-d", classes, "--release", "17", javaFile]);
  const repo = path.join(dir, "company-repo", "com", "acme", "acme-db", "1.0");
  mkdirSync(repo, { recursive: true });
  // a jar is a zip; System32's bsdtar makes one where a JDK ships no jar.exe (IntelliJ's)
  const tar = path.join(process.env.SystemRoot ?? "C:\\Windows", "System32", "tar.exe");
  sh(dir, tar, [
    "--format",
    "zip",
    "-c",
    "-f",
    path.join(repo, "acme-db-1.0.jar"),
    "-C",
    classes,
    "com",
  ]);
  writeFileSync(
    path.join(repo, "acme-db-1.0.pom"),
    '<project xmlns="http://maven.apache.org/POM/4.0.0"><modelVersion>4.0.0</modelVersion>' +
      "<groupId>com.acme</groupId><artifactId>acme-db</artifactId><version>1.0</version></project>",
  );
  testCompile(dir);
}

/** The helper gains a method and a scenario uses it, but nobody builds again: target/test-classes is stale. */
function staleHelper(dir: string) {
  testCompile(dir);
  const helper = path.join(dir, "src", "test", "java", "com", "example", "qa", "DbHelper.java");
  writeFileSync(
    helper,
    readFileSync(helper, "utf8").replace(
      "private DbHelper() {}",
      'private DbHelper() {}\n\n    public static int countOrders() throws SQLException {\n        return rows("select * from orders", List.of()).size();\n    }',
    ),
  );
  const feature = path.join(dir, "src", "test", "java", "orders", "orders.feature");
  writeFileSync(
    feature,
    `${readFileSync(feature, "utf8")}\n  Scenario: count with a method added since the last build\n    * match db.countOrders() == 1\n`,
  );
}

const KARATE1 = { "insert and read a user": "passed", "call a local mock": "passed" } as const;
const ORDERS = { "read with a parameter": "passed", "a failed transaction leaves nothing behind": "passed" } as const;

const CASES: {
  sample: string;
  name?: string;
  /** Each scenario and the status it ends with. */
  scenarios: Record<string, "passed" | "failed">;
  before?: (dir: string) => void;
  /** A jar the kept list must hold. */
  listed?: string;
  /** What the console must say. */
  says?: string;
}[] = [
  { sample: "karate1-maven", scenarios: KARATE1 },
  { sample: "karate15-maven", scenarios: KARATE1 },
  { sample: "karate1-gradle", scenarios: KARATE1 },
  {
    sample: "work-suite",
    scenarios: { "add a user through the company library": "passed" },
    before: buildWorkSuite,
  },
  {
    // the limit: Grace runs the plain command, so a profile typed only as `mvn test -Pwork` never reaches it
    sample: "work-suite",
    name: "work-suite, profile on the command line only",
    scenarios: { "add a user through the company library": "failed" },
    before: (dir) => {
      buildWorkSuite(dir);
      rmSync(path.join(dir, ".mvn"), { recursive: true });
    },
  },
  // Karate's database testing guide: a compiled helper and a test-scoped driver
  { sample: "db-helper", scenarios: ORDERS, before: testCompile, listed: "h2-2.2.224.jar" },
  { sample: "db-helper-karate1", scenarios: ORDERS, before: testCompile, listed: "h2-2.2.224.jar" },
  {
    sample: "db-helper",
    name: "db-helper, never compiled",
    scenarios: {
      "read with a parameter": "failed",
      "a failed transaction leaves nothing behind": "failed",
    },
    says: "Java.type: class not found: com.example.qa.DbHelper",
  },
  {
    sample: "db-helper",
    name: "db-helper, changed since its last build",
    scenarios: { ...ORDERS, "count with a method added since the last build": "failed" },
    before: staleHelper,
    says: ".countOrders is not a function",
  },
];

for (const c of CASES)
  test(c.name ?? c.sample, async () => {
    const dir = path.join(tempDir("kk-check-"), c.sample);
    cpSync(path.join(SAMPLES, c.sample), dir, { recursive: true });
    c.before?.(dir);
    const { app, win, userData } = await launch(undefined, {
      PATH: [path.join(MAVEN, "bin"), path.join(GRADLE, "bin"), process.env.PATH].join(";"),
      JAVA_HOME: JDK,
    });
    const opened = Date.now();
    await openProject(app, dir);
    const names = Object.keys(c.scenarios);
    for (const s of names) await expect(node(win, s)).toBeVisible({ timeout: 300_000 });
    console.log(`${c.name ?? c.sample}: listed and found in ${Date.now() - opened} ms`);
    await expect(win.getByTestId("notice")).toHaveCount(0);
    if (c.listed) {
      const kept = JSON.parse(
        readFileSync(path.join(projectDataDir(userData, dir), "libraries.json"), "utf8"),
      );
      expect(kept.jars.map((j: string) => path.basename(j))).toContain(c.listed);
    }
    await win.getByTestId("run-all-button").click();
    for (const s of names)
      await expect(node(win, s)).toHaveAttribute("data-status", c.scenarios[s], {
        timeout: 120_000,
      });
    // the statuses settle before Karate's last lines reach the console
    await expect(win.getByTestId("run-outcome")).toHaveText(/Finished|Run failed/, {
      timeout: 60_000,
    });
    if (c.says)
      await expect(win.getByTestId("console")).toContainText(c.says, { timeout: 30_000 });
    const printed = await win.getByTestId("console").innerText();
    if (c.says) {
      console.log(`  the console says: ${printed.split("\n").find((l) => l.includes(c.says!))?.trim()}`);
    }
    // a second logging binding or a second Netty would say so here
    for (const word of ["SLF4J", "Multiple bindings", "ClassNotFound", "NoSuchMethod", "LinkageError"])
      expect(printed, word).not.toContain(word);
  });
