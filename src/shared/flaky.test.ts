import { describe, expect, it } from "vitest";
import {
  FLAKY_TITLE,
  flakyCounts,
  flakyHeading,
  orderEnvs,
  threadsLabel,
  type FlakyEnv,
} from "./flaky";

const env = (name: string | null): FlakyEnv => ({ env: name, passed: 1, failed: 1, runs: [] });

describe("flaky labels", () => {
  it("words the mark", () => {
    expect(FLAKY_TITLE).toBe("Flaky: passed and failed with the same project files");
  });

  it("heads a box with its environment, or says there is none", () => {
    expect(flakyHeading("qa")).toBe("Flaky on qa");
    expect(flakyHeading(null)).toBe("Flaky with no environment");
  });

  it("counts the runs, with singular for 1", () => {
    expect(flakyCounts(1, 1)).toBe("Passed 1 time, failed 1 time with the same project files");
    expect(flakyCounts(3, 2)).toBe("Passed 3 times, failed 2 times with the same project files");
    expect(flakyCounts(1, 2)).toBe("Passed 1 time, failed 2 times with the same project files");
  });

  it("words the thread count", () => {
    expect(threadsLabel(1)).toBe("1 thread");
    expect(threadsLabel(8)).toBe("8 threads");
    expect(threadsLabel(null)).toBe("—");
  });
});

describe("orderEnvs", () => {
  const names = (envs: FlakyEnv[]) => envs.map((e) => e.env);

  it("puts the current environment first, then none, then the rest by name", () => {
    const envs = [env("dev"), env(null), env("qa")];
    expect(names(orderEnvs(envs, "qa"))).toEqual(["qa", null, "dev"]);
    expect(names(orderEnvs(envs, null))).toEqual([null, "dev", "qa"]);
    expect(names(orderEnvs([env("qa"), env("dev"), env("alpha")], "prod"))).toEqual([
      "alpha",
      "dev",
      "qa",
    ]);
  });

  it("leaves its input as it was", () => {
    const envs = [env("dev"), env(null), env("qa")];
    orderEnvs(envs, "qa");
    expect(names(envs)).toEqual(["dev", null, "qa"]);
  });
});
