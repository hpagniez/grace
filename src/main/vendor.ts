import { join } from "node:path";

export interface VendorPaths {
  java: string;
  jar: string;
  jreDir: string;
}

/** base = `<repo>/vendor` in dev and tests, `process.resourcesPath` when packaged. */
export function vendorPaths(base: string): VendorPaths {
  const jreDir = join(base, "jre");
  return { java: join(jreDir, "bin", "java.exe"), jar: join(base, "karate.jar"), jreDir };
}
