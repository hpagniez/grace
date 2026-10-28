import { createHash } from "node:crypto";
import { spawn } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import {
  cleanUp,
  vendor,
  T,
  setup,
  fixture,
  printed,
  byName,
  until,
  javaCommandLines,
  holdSlow,
} from "../../test/runner";
import type { PropertiesEdit, RowEdit } from "../shared/env";
import { fakeSecretBox } from "./secrets";
import { loadProjectEnv, saveProjectEnv } from "./settings";

afterEach(cleanUp);

describe("Runner properties", () => {
  const SECRET = "s3cret-token-123";
  const plain = (name: string, value: string): RowEdit => ({ name, value, secret: false });
  const secret = (name: string, value: string): RowEdit => ({ name, value, secret: true });
  const keep = (name: string): RowEdit => ({ name, secret: true, keep: true });
  const argFile = (dataDir: string, root: string) =>
    path.join(
      dataDir,
      "projects",
      createHash("sha1").update(root).digest("hex").slice(0, 12),
      "run-args.txt",
    );

  it("shared and env properties, env wins", T, async () => {
    const { runner, events, last } = setup();
    await runner.open(fixture("env"));
    runner.saveProperties({
      shared: [plain("timeout", "5000")],
      byEnv: { qa: [plain("timeout", "9000")] },
    });
    await runner.setEnv("qa");
    await runner.run(byName(last("project").tree!, "always").key);
    expect(printed(events).some((l) => l.includes("timeout = 9000"))).toBe(true);
    const from = events.length;
    await runner.setEnv(null);
    await runner.run(byName(last("project").tree!, "always").key);
    expect(printed(events.slice(from)).some((l) => l.includes("timeout = 5000"))).toBe(true);
  });

  it("a secret reaches the run and is masked", T, async () => {
    const { runner, events, all, last } = setup();
    const root = fixture("env");
    // a failing match puts the value in the scenario's error too
    writeFileSync(
      path.join(root, "tests", "leak.feature"),
      [
        "Feature: leak",
        "",
        "Scenario: leak",
        "  * print 'api.token =', karate.properties['api.token']",
        "  * match karate.properties['api.token'] == 'other'",
        "",
      ].join("\n"),
    );
    await runner.open(root);
    await runner.setEnv("qa");
    runner.saveProperties({ shared: [], byEnv: { qa: [secret("api.token", SECRET)] } });
    expect(last("environment").view.properties.byEnv.qa).toEqual([
      { name: "api.token", secret: true, state: "set" },
    ]);
    await runner.run("f:tests/leak.feature");
    expect(last("runFinished").summary).toMatchObject({ scenariosFailed: 1 });
    expect(printed(events).some((l) => l.includes("api.token = ****"))).toBe(true);
    const failed = all("scenarios")
      .flatMap((e) => e.updates)
      .find((u) => u.status === "failed");
    expect(failed?.error).toContain("****");
    const logs = all("steps").flatMap((e) => e.steps.map((s) => s.log ?? ""));
    expect(logs.some((l) => l.includes("api.token = ****"))).toBe(true);
    expect(JSON.stringify(events)).not.toContain(SECRET);
  });

  it("settings never hold the plain secret", T, async () => {
    const { runner, dataDir } = setup();
    const root = fixture("env");
    await runner.open(root);
    runner.saveProperties({ shared: [secret("api.token", SECRET)], byEnv: {} });
    expect(readFileSync(path.join(dataDir, "settings.json"), "utf8")).not.toContain(SECRET);
    expect(loadProjectEnv(dataDir, root).properties.shared).toEqual([
      { name: "api.token", secret: fakeSecretBox().encrypt(SECRET) },
    ]);
  });

  it("properties it cannot save are applied with a notice", T, async () => {
    const { runner, events, last, dataDir } = setup();
    await runner.open(fixture("env"));
    rmSync(path.join(dataDir, "settings.json")); // open saved it: replace it with a folder
    mkdirSync(path.join(dataDir, "settings.json")); // writing a file there fails
    expect(runner.saveProperties({ shared: [plain("timeout", "5000")], byEnv: {} })).toBe(true); // applied, though not saved
    expect(last("notice").message).toContain("The properties could not be saved");
    expect(last("environment").view.properties.shared).toEqual([
      { name: "timeout", value: "5000" },
    ]);
    await runner.run(byName(last("project").tree!, "always").key);
    expect(printed(events).some((l) => l.includes("timeout = 5000"))).toBe(true);
  });

  it("the secret is not on the command line", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const root = fixture("env");
    const release = holdSlow(root);
    await runner.open(root);
    runner.saveProperties({ shared: [secret("api.token", SECRET)], byEnv: {} });
    const saved = loadProjectEnv(dataDir, root);
    const run = runner.run(byName(last("project").tree!, "slow").key);
    await until(() => all("runStarted").length > 0);
    const lines = javaCommandLines(root);
    expect(lines).toHaveLength(1);
    expect(lines[0]).toContain(`@${argFile(dataDir, root)}`);
    expect(lines[0]).not.toContain(SECRET);
    // a save while busy is refused
    expect(runner.saveProperties({ shared: [], byEnv: {} })).toBe(false);
    expect(last("notice").message).toBe("A run is already in progress");
    expect(loadProjectEnv(dataDir, root)).toEqual(saved);
    release();
    await run;
    expect(last("runFinished").outcome).toBe("finished");
  });

  it("no argument file is left after a run, a cancel, or a failed spawn", T, async () => {
    const paths = { ...vendor };
    const { runner, events, all, last, dataDir } = setup(undefined, undefined, paths);
    const root = fixture("env");
    const file = argFile(dataDir, root);
    const key = (name: string) => byName(last("project").tree!, name).key;
    await runner.open(root);
    runner.saveProperties({ shared: [plain("timeout", "5000")], byEnv: {} });

    await runner.run(key("always"));
    expect(last("runFinished").outcome).toBe("finished");
    expect(existsSync(file)).toBe(false);

    const slow = runner.run(key("slow"));
    await until(() => all("runStarted").length === 2);
    expect(readFileSync(file, "latin1")).toBe('"-Dtimeout=5000"\r\n');
    await runner.cancel();
    await slow;
    expect(last("runFinished").outcome).toBe("cancelled");
    expect(existsSync(file)).toBe(false);

    const pidFile = path.join(dataDir, "running.pid");
    mkdirSync(pidFile); // writing a file there fails
    try {
      await runner.run(key("always"));
      expect(last("notice").message).toContain("EISDIR");
      expect(existsSync(file)).toBe(false);
    } finally {
      rmSync(pidFile, { recursive: true, force: true });
    }

    // the pre-run dry run is spawned before run() first awaits: only the run gets the missing Java
    const from = events.length;
    const failing = runner.run(key("always"));
    paths.java = path.join(dataDir, "missing", "java.exe");
    await failing;
    expect(last("runFinished").outcome).toBe("failed");
    expect(printed(events.slice(from)).some((l) => l.includes("ENOENT"))).toBe(true);
    expect(existsSync(file)).toBe(false);
  });

  it("an argument file another process holds is a notice, not an error", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const root = fixture("env");
    await runner.open(root);
    runner.saveProperties({ shared: [plain("timeout", "5000")], byEnv: {} });
    const run = runner.run(byName(last("project").tree!, "slow").key);
    await until(() => all("runStarted").length > 0);
    // without delete sharing, as an antivirus or an indexer can, until its input closes
    const holder = spawn("powershell", [
      "-NoProfile",
      "-NonInteractive",
      "-Command",
      `$f = [IO.File]::Open('${argFile(dataDir, root)}', 'Open', 'Read', 'ReadWrite'); 'held'; [Console]::In.ReadLine(); $f.Close()`,
    ]);
    const exited = new Promise((r) => holder.on("exit", r));
    try {
      await new Promise<void>((resolve, reject) => {
        holder.stdout.on("data", (d) => String(d).includes("held") && resolve());
        void exited.then(() => reject(new Error("the holder exited")));
      });
      await run;
      expect(last("runFinished").outcome).toBe("finished");
      expect(last("notice").message).toContain("could not be deleted");
    } finally {
      holder.stdin.end();
      await exited;
    }
  });

  it("an unreadable secret blocks the run", T, async () => {
    const first = setup();
    const root = fixture("env");
    await first.runner.open(root);
    first.runner.saveProperties({ shared: [secret("api.token", SECRET)], byEnv: {} });
    await first.runner.dispose();
    const settings = path.join(first.dataDir, "settings.json");
    const cipher = fakeSecretBox().encrypt(SECRET);
    writeFileSync(settings, readFileSync(settings, "utf8").replace(cipher, "Zm9v"));

    const { runner, all, last } = setup(first.dataDir);
    await runner.open(root);
    expect(last("environment").view.properties.shared).toEqual([
      { name: "api.token", secret: true, state: "unreadable" },
    ]);
    await runner.run("d:");
    expect(last("notice").message).toBe(
      "Secret api.token (All environments) can't be read. Replace it in Properties.",
    );
    expect(all("runStarted")).toHaveLength(0);
    // refused before the pre-run dry run: no JVM started; discovery still works
    expect(all("project")).toHaveLength(1);
    expect(last("project").tree).not.toBeNull();

    // with secret storage unavailable, a secret of the environment cannot be used either
    saveProjectEnv(first.dataDir, root, {
      env: "qa",
      properties: { shared: [], byEnv: { qa: [{ name: "api.token", secret: cipher }] } },
    });
    const off = setup(first.dataDir, fakeSecretBox(false));
    await off.runner.open(root);
    await off.runner.run("d:");
    expect(off.last("notice").message).toBe(
      "Secret api.token (qa) can't be read. Replace it in Properties.",
    );
    expect(off.all("runStarted")).toHaveLength(0);
    expect(off.all("project")).toHaveLength(1);
  });

  it("keep copies the stored ciphertext, a rename needs a new value", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const root = fixture("env");
    await runner.open(root);
    runner.saveProperties({
      shared: [secret("api.token", SECRET), plain("timeout", "5000")],
      byEnv: {},
    });
    const cipher = fakeSecretBox().encrypt(SECRET);
    const applied = runner.saveProperties({
      shared: [keep("api.token"), plain("timeout", "6000")],
      byEnv: { qa: [] },
    });
    expect(applied).toBe(true);
    expect(all("notice")).toHaveLength(0);
    expect(loadProjectEnv(dataDir, root).properties).toEqual({
      shared: [
        { name: "api.token", secret: cipher },
        { name: "timeout", value: "6000" },
      ],
      byEnv: { qa: [] },
    });
    expect(last("environment").view.properties).toEqual({
      shared: [
        { name: "api.token", secret: true, state: "set" },
        { name: "timeout", value: "6000" },
      ],
      byEnv: { qa: [] },
    });

    const saved = loadProjectEnv(dataDir, root);
    const views = all("environment").length;
    const cases: [PropertiesEdit, string][] = [
      [{ shared: [keep("api.key")], byEnv: {} }, "All environments: api.key"],
      [{ shared: [], byEnv: { qa: [keep("api.token")] } }, "qa: api.token"],
      [{ shared: [keep("timeout")], byEnv: {} }, "All environments: timeout"],
    ];
    for (const [edit, row] of cases) {
      expect(runner.saveProperties(edit)).toBe(false);
      expect(last("notice").message).toBe(`Properties not saved: ${row}: No saved secret to keep`);
    }
    expect(loadProjectEnv(dataDir, root)).toEqual(saved);
    expect(all("environment")).toHaveLength(views);
  });

  const savedSettings = (dataDir: string) =>
    JSON.parse(readFileSync(path.join(dataDir, "settings.json"), "utf8"));

  it("reserved names and bad values are refused", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const root = fixture("env");
    expect(runner.saveProperties({ shared: [], byEnv: {} })).toBe(false); // no project
    await runner.open(root);
    const views = all("environment").length;
    const cases: [PropertiesEdit, string][] = [
      [{ shared: [plain("karate.options", "-t @x")], byEnv: {} }, "karate.options: Reserved name"],
      [{ shared: [plain("JAVA.home", "x")], byEnv: {} }, "JAVA.home: Reserved name"],
      [{ shared: [plain("a", "x\ny")], byEnv: {} }, "a: No line breaks"],
      [
        { shared: [secret("a", "é")], byEnv: {} },
        "a: This value cannot be passed to Java exactly.",
      ],
      [{ shared: [plain("a", "1"), plain("a", "2")], byEnv: {} }, "a: Duplicate name"],
    ];
    for (const [edit, error] of cases) {
      expect(runner.saveProperties(edit)).toBe(false);
      expect(last("notice").message).toBe(`Properties not saved: All environments: ${error}`);
    }
    expect(all("environment")).toHaveLength(views);
    expect(savedSettings(dataDir)).not.toHaveProperty("projects"); // open saved the recent list only

    const off = setup(undefined, fakeSecretBox(false));
    await off.runner.open(root);
    expect(off.runner.saveProperties({ shared: [secret("api.token", SECRET)], byEnv: {} })).toBe(
      false,
    );
    expect(off.last("notice").message).toBe("Secret storage is not available on this system");
    expect(savedSettings(off.dataDir)).not.toHaveProperty("projects");
    // plain rows still save
    expect(off.runner.saveProperties({ shared: [plain("timeout", "1")], byEnv: {} })).toBe(true);
    expect(loadProjectEnv(off.dataDir, root).properties.shared).toEqual([
      { name: "timeout", value: "1" },
    ]);
  });

  it("saved rows today's rules reject are named in one notice on open", T, async () => {
    const root = fixture("env");
    const dataDir = mkdtempSync(path.join(tmpdir(), "kk-data-"));
    const properties = {
      shared: [
        { name: "timeout", value: "5000" },
        { name: "java.home", value: "C:/jdk" },
      ],
      byEnv: {
        qa: [{ name: "greeting", secret: "Zm9v", value: "h\u00e9llo" }, { secret: "Zm9v" }],
      },
    };
    writeFileSync(
      path.join(dataDir, "settings.json"),
      JSON.stringify({ projects: { [root]: { properties } } }),
    );
    const { runner, all, last } = setup(dataDir);
    await runner.open(root);
    await runner.refresh();
    expect(all("notice").map((n) => n.message)).toEqual([
      "Some saved properties were not loaded and are kept in settings.json: All environments: java.home, qa: greeting, qa: (no name)",
    ]);
    expect(last("environment").view.properties).toEqual({
      shared: [{ name: "timeout", value: "5000" }],
      byEnv: { qa: [] },
    });
  });

  it("no properties, no @ argument", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const root = fixture("env");
    holdSlow(root); // the cancel ends the run
    await runner.open(root);
    // rows of another environment are not passed
    runner.saveProperties({ shared: [], byEnv: { qa: [plain("timeout", "9000")] } });
    const run = runner.run(byName(last("project").tree!, "slow").key);
    await until(() => all("runStarted").length > 0);
    const lines = javaCommandLines(root);
    expect(lines).toHaveLength(1);
    expect(lines[0]).not.toContain("run-args.txt");
    expect(lines[0]).not.toMatch(/\s"?@/);
    expect(existsSync(argFile(dataDir, root))).toBe(false);
    await runner.cancel();
    await run;
  });
});
