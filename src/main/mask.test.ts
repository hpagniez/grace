import { describe, expect, it } from "vitest";
import { makeMasker, secretForms } from "./mask";

describe("secretForms", () => {
  it("lists the value, its URL-encoded form and its JSON-escaped form", () => {
    expect(secretForms('a b"c')).toEqual(['a b"c', "a%20b%22c", 'a b\\"c']);
  });

  it("drops forms equal to one already listed", () => {
    expect(secretForms("token")).toEqual(["token"]);
  });

  it("has no forms for an empty value", () => {
    expect(secretForms("")).toEqual([]);
  });
});

describe("makeMasker", () => {
  it("masks every occurrence", () => {
    const mask = makeMasker(["s3cret"]);
    expect(mask("a s3cret b s3cret\nx s3cret")).toBe("a **** b ****\nx ****");
  });

  it("masks URL-encoded and JSON-escaped forms", () => {
    const mask = makeMasker(['a b"c']);
    expect(mask('plain a b"c, url a%20b%22c, json a b\\"c')).toBe(
      "plain ****, url ****, json ****",
    );
  });

  it("masks values with regex metacharacters literally", () => {
    const mask = makeMasker(["$&", ".*", "a\b"]);
    expect(mask("pre $& mid .* post a\b end abc")).toBe("pre **** mid **** post **** end abc");
  });

  it("masks the longest form first", () => {
    expect(makeMasker(["abc", "abcdef"])("xabcdefx")).toBe("x****x");
  });

  it("changes nothing without values", () => {
    expect(makeMasker([])("some s3cret text")).toBe("some s3cret text");
  });

  it("ignores empty values", () => {
    const mask = makeMasker(["", "s3cret"]);
    expect(mask("a s3cret b")).toBe("a **** b");
    expect(makeMasker([""])("a b")).toBe("a b");
  });
});
