/** Encrypts and decrypts secret run properties. A seam over Electron's safeStorage, so Node tests need no Electron. */
export interface SecretBox {
  available(): boolean;
  /** Base64 ciphertext. */
  encrypt(plain: string): string;
  /** The plain text, or null if the ciphertext cannot be read. Never throws. */
  decrypt(cipher: string): string | null;
}

type SafeStorageLike = Pick<
  typeof import("electron").safeStorage,
  "isEncryptionAvailable" | "encryptString" | "decryptString"
>;

/**
 * The real box. Usable only after `app.whenReady()`. Electron is required here, not imported at the top,
 * so Node-only tests can load this file: outside Electron, importing "electron" yields a path string
 * (and tries to download the binary if it is missing).
 */
export function safeStorageBox(
  storage: SafeStorageLike = require("electron").safeStorage,
): SecretBox {
  return {
    available: () => storage.isEncryptionAvailable(),
    encrypt: (plain) => storage.encryptString(plain).toString("base64"),
    decrypt: (cipher) => {
      try {
        return storage.decryptString(Buffer.from(cipher, "base64"));
      } catch {
        return null;
      }
    },
  };
}

const FAKE_PREFIX = "fake:";

/** For tests only: base64 of a prefixed copy, so ciphertext differs from plain text and corrupt input is detectable. */
export function fakeSecretBox(available = true): SecretBox {
  return {
    available: () => available,
    encrypt: (plain) => Buffer.from(FAKE_PREFIX + plain).toString("base64"),
    decrypt: (cipher) => {
      const text = Buffer.from(cipher, "base64").toString();
      return text.startsWith(FAKE_PREFIX) ? text.slice(FAKE_PREFIX.length) : null;
    },
  };
}
