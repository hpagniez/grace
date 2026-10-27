import { createHash } from "node:crypto";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { JAR, JRE, LICENSE, VERSIONS, verifySha256 } from "./fetch-vendor.mjs";

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

describe("pins", () => {
  it("pins each version once", () => {
    expect(VERSIONS).toEqual({ java: "21.0.12.1+1", karate: "2.1.4" });
  });

  it("builds the download URLs and file names from the versions", () => {
    expect(JRE.url).toBe(
      "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.12.1%2B1/OpenJDK21U-jre_x64_windows_hotspot_21.0.12.1_1.zip",
    );
    expect(JAR.url).toBe(
      "https://github.com/karatelabs/karate/releases/download/v2.1.4/karate-2.1.4.jar",
    );
    expect(JAR.file).toBe("karate-2.1.4.jar");
    expect(LICENSE.url).toBe("https://raw.githubusercontent.com/karatelabs/karate/v2.1.4/LICENSE");
    expect([JRE.sha256, JAR.sha256, LICENSE.sha256]).toEqual([
      "d35f31e712f0fcf6ac5a093edc90204fbff22f720ba3950bd09d331d5e621636",
      "1f743f4acc56526b39bd88c84bfe4491c46d5e94f4ab5a38c437c9400dbc2488",
      "28890ebdc037407a0e1168ce3b01880d5fbfbc6381f37629d25c070482ea0cea",
    ]);
  });
});
