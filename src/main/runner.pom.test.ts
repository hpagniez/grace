import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { cleanUp, record, setup, T, temps } from "../../test/runner";
import { NO_FILTER } from "../shared/tags";
import {
  emptyEnvSettings,
  loadProjectFilter,
  saveProjectEnv,
  saveProjectFilter,
  saveProjectThreads,
  savedControls,
} from "./settings";

afterEach(cleanUp);

/** Writes `files` (relative path → content) into a fresh temp project. */
function write(files: Record<string, string>, root = mkdtempSync(path.join(tmpdir(), "kk-pom-"))) {
  if (!temps.includes(root)) temps.push(root);
  for (const [f, text] of Object.entries(files)) {
    mkdirSync(path.dirname(path.join(root, f)), { recursive: true });
    writeFileSync(path.join(root, f), text);
  }
  return root;
}

const FEATURE =
  "Feature: a\n@smoke\nScenario: fast\n  * print 'fast'\n@smoke @slow\nScenario: slow\n  * print 'slow'\nScenario: plain\n  * print 'plain'\n";
/** A standalone project whose karate-pom.json holds `pom`. */
const withPom = (pom: object, root?: string) =>
  write({ "a.feature": FEATURE, "karate-pom.json": JSON.stringify(pom) }, root);
const none = { env: false, filter: false, threads: false };

describe("Runner starts from the pom (docs/specs/2026-11-04-pom-defaults.md)", () => {
  it("karate-pom.json starts the controls, and a run passes them", T, async () => {
    const { runner, all, last, dataDir } = setup();
    const root = withPom({ env: "qa", tags: ["@smoke", "~@slow"], threads: 3 });
    await runner.open(root);
    expect(last("environment").view.env).toBe("qa");
    expect(last("filter").filter).toEqual({
      include: ["smoke"],
      exclude: ["slow"],
      hideNonMatching: false,
    });
    expect(last("threads").count).toBe(3);
    expect(all("notice")).toEqual([]);
    await runner.run("d:");
    const { runId } = last("runStarted");
    expect(record(dataDir, root, runId)).toMatchObject({
      env: "qa",
      threads: 3,
      filter: { include: ["smoke"], exclude: ["slow"] },
    });
    expect(savedControls(dataDir, root)).toEqual(none);
  });

  it("what the person saved wins", T, async () => {
    const { runner, last, dataDir } = setup();
    const root = withPom({ env: "qa", tags: ["@smoke"], threads: 3 });
    saveProjectEnv(dataDir, root, { ...emptyEnvSettings(), env: "dev" });
    saveProjectFilter(dataDir, root, NO_FILTER);
    saveProjectThreads(dataDir, root, 1);
    await runner.open(root);
    expect(last("environment").view.env).toBe("dev");
    expect(last("filter").filter).toEqual(NO_FILTER);
    expect(last("threads").count).toBe(1);
  });

  it("each control decides on its own", T, async () => {
    const { runner, last, dataDir } = setup();
    const root = withPom({ env: "qa", threads: 3 });
    saveProjectThreads(dataDir, root, 2);
    await runner.open(root);
    expect(last("environment").view.env).toBe("qa");
    expect(last("threads").count).toBe(2);
  });

  it("picking no environment is saved", T, async () => {
    const first = setup();
    const root = withPom({ env: "qa" });
    await first.runner.open(root);
    await first.runner.setEnv(null);
    await first.runner.dispose();
    const second = setup(first.dataDir);
    await second.runner.open(root);
    expect(second.last("environment").view.env).toBeNull();
  });

  it("saving run properties does not save the pom's environment", T, async () => {
    const first = setup();
    const root = withPom({ env: "qa" });
    await first.runner.open(root);
    expect(
      first.runner.saveProperties({
        shared: [{ name: "a", value: "1", secret: false }],
        byEnv: {},
      }),
    ).toBe(true);
    expect(savedControls(first.dataDir, root)?.env).toBe(false);
    await first.runner.dispose();
    withPom({ env: "dev" }, root);
    const second = setup(first.dataDir);
    await second.runner.open(root);
    expect(second.last("environment").view.env).toBe("dev");
  });

  it("a pom filter whose tags no test has is not saved", T, async () => {
    const { runner, last, dataDir } = setup();
    const root = withPom({ tags: ["~@gone"] });
    await runner.open(root);
    expect(last("filter").filter).toEqual(NO_FILTER);
    expect(savedControls(dataDir, root)?.filter).toBe(false);
  });

  it("a change in the filter bar saves the whole filter shown", T, async () => {
    const { runner, dataDir } = setup();
    const root = withPom({ tags: ["@smoke"] });
    await runner.open(root);
    const shown = { include: ["smoke"], exclude: [], hideNonMatching: true };
    runner.setFilter(shown);
    expect(loadProjectFilter(dataDir, root)).toEqual(shown);
  });

  it("Refresh applies an edited pom to the controls with nothing saved", T, async () => {
    const { runner, last } = setup();
    const root = withPom({ env: "qa", threads: 3 });
    await runner.open(root);
    runner.setThreads(2);
    withPom({ env: "dev", threads: 4 }, root);
    await runner.refresh();
    expect(last("environment").view.env).toBe("dev");
    expect(last("threads").count).toBe(2);
  });

  it("Refresh keeps the controls in force when settings.json cannot be read", T, async () => {
    const { runner, last, dataDir } = setup();
    const root = withPom({ env: "qa" });
    await runner.open(root);
    runner.setThreads(2);
    const settings = path.join(dataDir, "settings.json");
    rmSync(settings);
    mkdirSync(settings); // reading it now fails with EISDIR
    withPom({ env: "dev", threads: 4 }, root);
    await runner.refresh();
    expect(last("environment").view.env).toBe("qa");
    expect(last("threads").count).toBe(2);
  });

  it("Refresh drops a value the pom no longer gives", T, async () => {
    const { runner, last } = setup();
    const root = withPom({ env: "qa" });
    await runner.open(root);
    withPom({}, root);
    await runner.refresh();
    expect(last("environment").view.env).toBeNull();
  });

  it("the notice names only what Grace does not use", T, async () => {
    const { runner, last } = setup();
    await runner.open(withPom({ env: "qa", tags: ["~@slow"], threads: 2, dryRun: true }));
    expect(last("notice").message).toBe(
      "Grace does not use these karate-pom.json settings: dryRun.",
    );
  });

  it("Surefire passes a Maven project's karate.env", T, async () => {
    const { runner, last } = setup();
    await runner.open(
      write({
        "src/test/java/a.feature": FEATURE,
        "pom.xml":
          "<project><properties><karate.env>dev</karate.env></properties><build><plugins><plugin>" +
          "<artifactId>maven-surefire-plugin</artifactId><configuration><systemPropertyVariables>" +
          "<karate.env>${karate.env}</karate.env></systemPropertyVariables></configuration>" +
          "</plugin></plugins></build></project>",
      }),
    );
    expect(last("environment").view.env).toBe("dev");
  });
});
