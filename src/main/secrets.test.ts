import { describe, expect, it } from "vitest";
import { fakeSecretBox, safeStorageBox } from "./secrets";

// Stands in for Electron's safeStorage; decryptString throws on anything it did not encrypt.
const storage = (available = true) => ({
  isEncryptionAvailable: () => available,
  encryptString: (plain: string) => Buffer.from(`enc:${plain}`),
  decryptString: (cipher: Buffer) => {
    const text = cipher.toString();
    if (!text.startsWith("enc:")) throw new Error("Error while decrypting the ciphertext");
    return text.slice(4);
  },
});

describe("fakeSecretBox", () => {
  it("round-trips and rejects corrupt input", () => {
    const box = fakeSecretBox();
    const cipher = box.encrypt("s3cret");
    expect(cipher).not.toContain("s3cret");
    expect(box.decrypt(cipher)).toBe("s3cret");
    expect(box.decrypt(box.encrypt(""))).toBe("");
    expect(box.decrypt("")).toBeNull();
    expect(box.decrypt("!!not base64!!")).toBeNull();
    expect(box.decrypt(Buffer.from("garbage").toString("base64"))).toBeNull();
  });

  it("reports whether it is available", () => {
    expect(fakeSecretBox().available()).toBe(true);
    expect(fakeSecretBox(false).available()).toBe(false);
  });
});

describe("safeStorageBox", () => {
  it("stores the base64 of the encrypted string and reads it back", () => {
    const box = safeStorageBox(storage());
    const cipher = box.encrypt("s3cret");
    expect(cipher).toBe(Buffer.from("enc:s3cret").toString("base64"));
    expect(box.decrypt(cipher)).toBe("s3cret");
    expect(box.decrypt(box.encrypt(""))).toBe("");
  });

  it("reports whether encryption is available", () => {
    expect(safeStorageBox(storage()).available()).toBe(true);
    expect(safeStorageBox(storage(false)).available()).toBe(false);
  });

  it("decrypt never throws", () => {
    const box = safeStorageBox(storage());
    expect(box.decrypt("!!not base64!!")).toBeNull();
    expect(box.decrypt("")).toBeNull();
    expect(box.decrypt(Buffer.from("garbage").toString("base64"))).toBeNull();
    const broken = safeStorageBox({
      ...storage(),
      decryptString: () => {
        throw new Error("boom");
      },
    });
    expect(broken.decrypt(box.encrypt("s3cret"))).toBeNull();
  });

  // Importing this file runs secrets.ts in plain Node, where Electron's main-process API does not exist.
  it("loads without Electron", () => {
    expect(typeof safeStorageBox).toBe("function");
  });
});
