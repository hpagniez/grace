import { createHash } from "node:crypto";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { verifySha256 } from "./fetch-vendor.mjs";

const dir = mkdtempSync(join(tmpdir(), "kk-sha-"));
const file = join(dir, "a.bin");
writeFileSync(file, "hello");
const actual = createHash("sha256").update("hello").digest("hex");

describe("verifySha256", () => {
  it("resolves when the hash matches", async () => {
    await expect(verifySha256(file, actual)).resolves.toBeUndefined();
  });

  it("rejects naming the file and both hashes on mismatch", async () => {
    const wrong = "0".repeat(64);
    const err = await verifySha256(file, wrong).catch((e: Error) => e);
    expect(err).toBeInstanceOf(Error);
    expect((err as Error).message).toContain(file);
    expect((err as Error).message).toContain(wrong);
    expect((err as Error).message).toContain(actual);
  });
});
