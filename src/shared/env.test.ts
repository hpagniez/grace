import { describe, expect, it } from "vitest";
import {
  MAX_VALUE_LENGTH,
  RESERVED_KARATE_KEYS,
  editErrors,
  effectiveRows,
  envTagValues,
  parsePropertiesEdit,
  propertyNameError,
  propertyValueError,
  sortedUnique,
  validEnvName,
  type PropertiesEdit,
} from "./env";

const CANNOT_PASS = "This value cannot be passed to Java exactly.";

const plain = (name: string, value: string) => ({ name, value, secret: false });
const secret = (name: string, value: string) => ({ name, value, secret: true });
const keep = (name: string) => ({ name, secret: true as const, keep: true as const });
const edit = (shared: PropertiesEdit["shared"], byEnv: PropertiesEdit["byEnv"] = {}) => ({
  shared,
  byEnv,
});
const noSecrets = () => false;

describe("env names", () => {
  it("accepts ordinary names", () => {
    for (const n of ["qa", "prod-2", "eu.west_1", "1", "A"]) expect(validEnvName(n)).toBe(true);
    expect(validEnvName("a".repeat(64))).toBe(true);
  });

  it("rejects empty, path-like, spaced, long and dash-leading names", () => {
    for (const n of ["", "../x", "a b", "a".repeat(65), "-x", ".x", "_x", "é"])
      expect(validEnvName(n)).toBe(false);
  });
});

describe("property names", () => {
  it("reserves Karate and JVM names in any case", () => {
    for (const n of ["karate.env", "KARATE.OPTIONS", "java.home", "Sun.x", "user.dir", "JDK.x"])
      expect(propertyNameError(n)).toBe("Reserved name");
    for (const n of RESERVED_KARATE_KEYS)
      expect(propertyNameError(n.toUpperCase())).toBe("Reserved name");
  });

  it("knows the keys Karate 2.1.4 reads", () => {
    expect(RESERVED_KARATE_KEYS).toEqual(
      expect.arrayContaining([
        "karate.env",
        "karate.options",
        "karate.config.dir",
        "karate.output.dir",
        "karate.debug.port",
      ]),
    );
  });

  it("allows other names, including other karate.* ones", () => {
    for (const n of ["karate.mything", "timeout", "api.token", "_x", "a-b.c_d"])
      expect(propertyNameError(n)).toBeNull();
  });

  it("rejects names that do not match the pattern", () => {
    for (const n of ["", "1x", "a b", "a=b", "a".repeat(201), "é"])
      expect(propertyNameError(n)).toBe("Invalid name");
    expect(propertyNameError("a".repeat(200))).toBeNull();
  });
});

describe("value rules", () => {
  it("rejects too long values", () => {
    expect(propertyValueError("x".repeat(MAX_VALUE_LENGTH))).toBeNull();
    expect(propertyValueError("x".repeat(MAX_VALUE_LENGTH + 1))).toBe("Too long");
  });

  it("rejects line breaks", () => {
    expect(propertyValueError("a\nb")).toBe("No line breaks");
    expect(propertyValueError("a\r\nb")).toBe("No line breaks");
  });

  it("rejects anything that is not printable ASCII, with the exact copy", () => {
    for (const v of ["é", "\t", "a\u0000b", "\u007f", "日本", "\u2028"])
      expect(propertyValueError(v)).toBe(CANNOT_PASS);
  });

  it("accepts spaces, quotes, backslashes and percent signs", () => {
    expect(propertyValueError('x y "q" C:\\Users\\me %PATH%')).toBeNull();
    expect(propertyValueError("")).toBeNull();
  });
});

describe("effective rows", () => {
  const shared = [
    { name: "timeout", value: "5000" },
    { name: "a", value: "1" },
  ];
  const byEnv = {
    qa: [
      { name: "timeout", value: "9000" },
      { name: "b", value: "2" },
    ],
  };

  it("environment wins by name; shared order first, then environment-only rows", () => {
    expect(effectiveRows(shared, byEnv, "qa")).toEqual([
      { name: "timeout", value: "9000" },
      { name: "a", value: "1" },
      { name: "b", value: "2" },
    ]);
  });

  it("gives the shared rows for no environment or one without a list", () => {
    expect(effectiveRows(shared, byEnv, null)).toEqual(shared);
    expect(effectiveRows(shared, byEnv, "prod")).toEqual(shared);
    expect(effectiveRows(shared, byEnv, "constructor")).toEqual(shared);
  });
});

describe("envTagValues", () => {
  it("splits values of env and envnot tags", () => {
    expect(envTagValues(["env=dev,qa", "envnot=prod", "smoke", "env"])).toEqual([
      "dev",
      "qa",
      "prod",
    ]);
  });

  it("drops invalid names and ignores look-alike tags", () => {
    expect(envTagValues(["env=a b,,../x,ok", "environment=x", "envnot="])).toEqual(["ok"]);
  });
});

describe("sortedUnique", () => {
  it("merges, de-duplicates and sorts", () => {
    expect(sortedUnique([["qa", "dev"], ["prod", "qa"], []])).toEqual(["dev", "prod", "qa"]);
  });
});

describe("parsePropertiesEdit", () => {
  const ok = { shared: [plain("a", "1")], byEnv: { qa: [secret("t", "x"), keep("k")] } };

  it("accepts a well-formed edit and keeps only known fields", () => {
    expect(parsePropertiesEdit(ok)).toEqual(ok);
    expect(parsePropertiesEdit({ shared: [{ ...plain("a", "1"), extra: 1 }], byEnv: {} })).toEqual({
      shared: [plain("a", "1")],
      byEnv: {},
    });
  });

  it("rejects bad shapes", () => {
    const bad: unknown[] = [
      null,
      "x",
      [],
      {},
      { shared: {}, byEnv: {} },
      { shared: [], byEnv: [] },
      { shared: [], byEnv: { qa: {} } },
      { shared: [{ value: "1", secret: false }], byEnv: {} },
      { shared: [{ name: "a", value: 1, secret: false }], byEnv: {} },
      { shared: [{ name: "a", value: "1" }], byEnv: {} },
      { shared: [{ name: "a", secret: false, keep: true }], byEnv: {} },
      { shared: [{ name: "a", secret: true, keep: false }], byEnv: {} },
      { shared: [plain("a", "x".repeat(MAX_VALUE_LENGTH + 1))], byEnv: {} },
      { shared: [plain("a".repeat(201), "1")], byEnv: {} },
      { shared: [], byEnv: { "../x": [] } },
      JSON.parse('{"shared":[],"byEnv":{"__proto__":[]}}'),
      { shared: Array.from({ length: 501 }, (_, i) => plain(`p${i}`, "1")), byEnv: {} },
      { shared: [], byEnv: { qa: Array.from({ length: 501 }, (_, i) => plain(`p${i}`, "1")) } },
      {
        shared: [],
        byEnv: Object.fromEntries(Array.from({ length: 101 }, (_, i) => [`e${i}`, []])),
      },
    ];
    for (const x of bad) expect(parsePropertiesEdit(x)).toBeNull();
  });

  it("accepts the limits themselves", () => {
    const shared = Array.from({ length: 500 }, (_, i) =>
      plain(`p${i}`, "x".repeat(MAX_VALUE_LENGTH)),
    );
    const byEnv = Object.fromEntries(Array.from({ length: 100 }, (_, i) => [`e${i}`, []]));
    expect(parsePropertiesEdit({ shared, byEnv })).not.toBeNull();
  });
});

describe("editErrors", () => {
  it("is empty for a valid edit", () => {
    expect(editErrors(edit([plain("a", "1")], { qa: [secret("t", "x")] }), noSecrets)).toEqual([]);
  });

  it("reports duplicates and reserved names per scope", () => {
    const e = edit([plain("a", "1"), plain("a", "2"), plain("karate.env", "x")], {
      qa: [plain("1bad", "x"), plain("timeout", "1"), plain("timeout", "2")],
    });
    expect(editErrors(e, noSecrets)).toEqual([
      "All environments: a: Duplicate name",
      "All environments: karate.env: Reserved name",
      "qa: 1bad: Invalid name",
      "qa: timeout: Duplicate name",
    ]);
  });

  it("reports value problems, secret values included", () => {
    const e = edit([plain("a", "é"), secret("b", "x\ny")], {
      qa: [plain("c", "x".repeat(10_001))],
    });
    expect(editErrors(e, noSecrets)).toEqual([
      `All environments: a: ${CANNOT_PASS}`,
      "All environments: b: No line breaks",
      "qa: c: Too long",
    ]);
  });

  it("keep needs a stored secret of the same name in the same scope", () => {
    const stored = new Set(["null|token", "qa|api"]);
    const has = (scope: string | null, name: string) => stored.has(`${scope}|${name}`);
    const e = edit([keep("token"), keep("api")], { qa: [keep("api"), keep("token")] });
    expect(editErrors(e, has)).toEqual([
      "All environments: api: No saved secret to keep",
      "qa: token: No saved secret to keep",
    ]);
  });
});
