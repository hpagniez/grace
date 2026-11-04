// Shared by main and renderer: keep this file free of Node imports.
// Rules for the run environment and the run properties (docs/specs/2026-10-25-environment-switcher.md).

export const ENV_NAME = /^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$/;
export const PROPERTY_NAME = /^[A-Za-z_][A-Za-z0-9_.-]{0,199}$/;
export const MAX_VALUE_LENGTH = 10_000;

export const MAX_ROWS = 500;
export const MAX_ENVS = 100;
const MAX_NAME_LENGTH = 200;

/** System properties Karate 2.1.4 reads (found by scanning the bundled jar); a run property must not replace them. */
export const RESERVED_KARATE_KEYS: readonly string[] = [
  "karate.env",
  "karate.options",
  "karate.config.dir",
  "karate.output.dir",
  "karate.debug.port",
  "karate.js.slotFrames",
];

const RESERVED_EXACT = new Set(
  [
    ...RESERVED_KARATE_KEYS,
    "user.dir",
    "user.home",
    "file.encoding",
    "native.encoding",
    "stdout.encoding",
    "stderr.encoding",
    "line.separator",
    "path.separator",
  ].map((k) => k.toLowerCase()),
);
const RESERVED_PREFIXES = ["java.", "jdk.", "sun."];

export const validEnvName = (name: string): boolean => ENV_NAME.test(name);

/** Variables Grace removes from Karate's environment, as they override the app's values; a run property must not set them. */
export const STRIPPED_ENV_VARS: readonly string[] = [
  "KARATE_OPTIONS",
  "KARATE_ENV",
  "KARATE_CONFIG_DIR",
  "JAVA_TOOL_OPTIONS",
  "JDK_JAVA_OPTIONS",
  "_JAVA_OPTIONS",
];
const ENV_VAR_NAME = /^[A-Za-z_][A-Za-z0-9_]{0,199}$/;

export function envVarNameError(name: string): string | null {
  if (!ENV_VAR_NAME.test(name)) return "Invalid name";
  return STRIPPED_ENV_VARS.includes(name.toUpperCase()) ? "Reserved name" : null;
}

/** The name rule of a row's kind: an environment variable's, or a system property's. */
export const nameError = (name: string, envVar: boolean): string | null =>
  envVar ? envVarNameError(name) : propertyNameError(name);

/**
 * Two rows of a list clash when these intersect: the same name, or two environment variables
 * that differ only in letter case, which Windows sees as one.
 */
export const nameKeys = (name: string, envVar: boolean): string[] =>
  envVar ? [name, `\0${name.toUpperCase()}`] : [name];

/** An imported key's variable name: the prefix as typed, then the key in upper case, other characters as `_`. */
export const envVarFromKey = (prefix: string, key: string): string =>
  prefix + key.toUpperCase().replace(/[^A-Z0-9_]/g, "_");

/** What an import prefix may hold. */
export const PREFIX = /^[A-Za-z0-9_]*$/;

export function propertyNameError(name: string): string | null {
  if (!PROPERTY_NAME.test(name)) return "Invalid name";
  const lower = name.toLowerCase();
  return RESERVED_EXACT.has(lower) || RESERVED_PREFIXES.some((p) => lower.startsWith(p))
    ? "Reserved name"
    : null;
}

/** Java reads the argument file in the Windows ANSI code page; only printable ASCII is the same in all of them. */
export function propertyValueError(value: string): string | null {
  if (value.length > MAX_VALUE_LENGTH) return "Too long";
  if (/[\r\n]/.test(value)) return "No line breaks";
  return /[^\x20-\x7e]/.test(value) ? "This value cannot be passed to Java exactly." : null;
}

export interface PlainRow {
  name: string;
  value: string;
  /** Set on the Karate process's environment rather than passed as `-D`. */
  envVar?: boolean;
}

/** A saved secret as the window sees it: never its value. */
export interface SecretRowView {
  name: string;
  secret: true;
  state: "set" | "unreadable";
  envVar?: boolean;
}

export type RowView = PlainRow | SecretRowView;

export interface PropertyListsView {
  shared: RowView[];
  byEnv: Record<string, RowView[]>;
}

export interface EnvView {
  env: string | null;
  suggestions: string[];
  /** Environments that have a karate-config-<env>.js overlay file. */
  overlays: string[];
  properties: PropertyListsView;
  secretsAvailable: boolean;
}

/** A row sent by the window: a new value, or `keep` for a saved secret whose value stays. */
export type RowEdit =
  | { name: string; value: string; secret: boolean; envVar?: boolean }
  | { name: string; secret: true; keep: true; envVar?: boolean };

export interface PropertiesEdit {
  shared: RowEdit[];
  byEnv: Record<string, RowEdit[]>;
}

const parseRow = (x: unknown): RowEdit | null => {
  if (typeof x !== "object" || x === null) return null;
  const { name, value, secret, keep, envVar } = x as Record<string, unknown>;
  if (typeof name !== "string" || name.length > MAX_NAME_LENGTH) return null;
  if (envVar !== undefined && typeof envVar !== "boolean") return null;
  const kind = envVar === undefined ? {} : { envVar };
  if (keep !== undefined)
    return keep === true && secret === true ? { name, secret, keep, ...kind } : null;
  if (typeof value !== "string" || value.length > MAX_VALUE_LENGTH) return null;
  return typeof secret === "boolean" ? { name, value, secret, ...kind } : null;
};

const parseRows = (x: unknown): RowEdit[] | null => {
  if (!Array.isArray(x) || x.length > MAX_ROWS) return null;
  const rows = Array.from(x, parseRow); // not x.map: it skips holes in a sparse array
  return rows.every((r) => r !== null) ? rows : null;
};

/** Validates the shape of untrusted input (IPC); the rules of each row are `editErrors`. */
export function parsePropertiesEdit(x: unknown): PropertiesEdit | null {
  if (typeof x !== "object" || x === null) return null;
  const { shared, byEnv } = x as Record<string, unknown>;
  if (typeof byEnv !== "object" || byEnv === null || Array.isArray(byEnv)) return null;
  const sharedRows = parseRows(shared);
  const envs = Object.keys(byEnv);
  if (!sharedRows || envs.length > MAX_ENVS || !envs.every(validEnvName)) return null;
  const out: PropertiesEdit = { shared: sharedRows, byEnv: {} };
  for (const env of envs) {
    const rows = parseRows((byEnv as Record<string, unknown>)[env]);
    if (!rows) return null;
    out.byEnv[env] = rows;
  }
  return out;
}

/** Every broken rule of the edit, as `<scope>: <name>: <reason>`. Scope `null` is "All environments". */
export function editErrors(
  e: PropertiesEdit,
  storedSecrets: (scope: string | null, name: string) => boolean,
): string[] {
  const errors: string[] = [];
  const check = (scope: string | null, rows: RowEdit[]) => {
    const seen = new Set<string>();
    for (const row of rows) {
      const keys = nameKeys(row.name, row.envVar === true);
      const reasons = [
        nameError(row.name, row.envVar === true),
        keys.some((k) => seen.has(k)) ? "Duplicate name" : null,
        "keep" in row
          ? storedSecrets(scope, row.name)
            ? null
            : "No saved secret to keep"
          : propertyValueError(row.value),
      ];
      for (const k of keys) seen.add(k);
      for (const reason of reasons)
        if (reason) errors.push(`${scope ?? "All environments"}: ${row.name}: ${reason}`);
    }
  };
  check(null, e.shared);
  for (const [env, rows] of Object.entries(e.byEnv)) check(env, rows);
  return errors;
}

/** R4: environment rows win by name; shared order first, then rows only the environment has. */
export function effectiveRows<T extends { name: string }>(
  shared: T[],
  byEnv: Record<string, T[]>,
  env: string | null,
): T[] {
  const own = env !== null && Object.hasOwn(byEnv, env) ? byEnv[env] : [];
  const wins = new Map(own.map((r) => [r.name, r]));
  const inShared = new Set(shared.map((r) => r.name));
  return [...shared.map((r) => wins.get(r.name) ?? r), ...own.filter((r) => !inShared.has(r.name))];
}

/** Environment names in `env=` and `envnot=` tags (written without `@`), e.g. `env=dev,qa`. */
export const envTagValues = (tags: string[]): string[] =>
  tags.flatMap((t) => (/^(?:env|envnot)=(.*)$/.exec(t)?.[1] ?? "").split(",").filter(validEnvName));

export const sortedUnique = (lists: string[][]): string[] => [...new Set(lists.flat())].sort();

/** A key of an imported `.properties` file, with its value. */
export interface ImportEntry {
  key: string;
  value: string;
}

/** Main's answer to Import from file…: the file's name and keys, a message, or null when no file was picked. */
export type ImportResult = { file: string; entries: ImportEntry[] } | { error: string } | null;
