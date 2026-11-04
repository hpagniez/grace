import { describe, expect, it } from "vitest";
import { importPlan } from "./importPlan";

const keys = (pairs: Record<string, string>) =>
  Object.entries(pairs).map(([key, value]) => ({ key, value }));
const file = keys({ "db.user": "u", "db.password": "p", "9lives": "x", empty: "" });
const existing = [{ name: "APP_DB_USER", envVar: true }];

describe("importPlan", () => {
  it("adds, replaces and skips, as variables with no prefix", () => {
    expect(importPlan(file, { envVar: true, prefix: "", existing })).toEqual({
      rows: [
        { key: "db.user", name: "DB_USER", value: "u" },
        { key: "db.password", name: "DB_PASSWORD", value: "p" },
      ],
      adds: 2,
      replaces: [],
      skips: [
        { key: "9lives", reason: "Invalid name" },
        { key: "empty", reason: "Empty value" },
      ],
    });
  });

  it("a prefix names the variables and finds the row they replace", () => {
    expect(importPlan(file, { envVar: true, prefix: "APP_", existing })).toEqual({
      rows: [
        { key: "db.user", name: "APP_DB_USER", value: "u" },
        { key: "db.password", name: "APP_DB_PASSWORD", value: "p" },
        { key: "9lives", name: "APP_9LIVES", value: "x" },
      ],
      adds: 2,
      replaces: ["APP_DB_USER"],
      skips: [{ key: "empty", reason: "Empty value" }],
    });
  });

  it("replaces a variable that differs only in case, not a property", () => {
    const one = keys({ "db.user": "u" });
    const plan = (e: { name: string; envVar: boolean }) =>
      importPlan(one, { envVar: true, prefix: "APP_", existing: [e] }).replaces;
    expect(plan({ name: "App_Db_User", envVar: true })).toEqual(["APP_DB_USER"]);
    expect(plan({ name: "app_db_user", envVar: false })).toEqual([]);
  });

  it("skips a key that maps to the name of an earlier one", () => {
    const plan = importPlan(keys({ "db.user": "a", db_user: "b" }), {
      envVar: true,
      prefix: "APP_",
      existing: [],
    });
    expect(plan.rows).toEqual([{ key: "db.user", name: "APP_DB_USER", value: "a" }]);
    expect(plan.skips).toEqual([{ key: "db_user", reason: "Same name as db.user" }]);
  });

  it("as system properties, a key keeps its name and their rule applies", () => {
    const plan = importPlan(file, { envVar: false, prefix: "IGNORED_", existing });
    expect(plan.rows.map((r) => r.name)).toEqual(["db.user", "db.password"]);
    expect(plan.skips[0]).toEqual({ key: "9lives", reason: "Invalid name" });
    expect(
      importPlan(keys({ "java.home": "x" }), { envVar: false, prefix: "", existing: [] }).skips,
    ).toEqual([{ key: "java.home", reason: "Reserved name" }]);
  });

  it("skips a value the value rules refuse, and a reserved variable", () => {
    const plan = importPlan(keys({ greeting: "héllo", java_tool_options: "x" }), {
      envVar: true,
      prefix: "",
      existing: [],
    });
    expect(plan.skips).toEqual([
      { key: "greeting", reason: "This value cannot be passed to Java exactly." },
      { key: "java_tool_options", reason: "Reserved name" },
    ]);
  });
});
