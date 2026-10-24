// Shared by main and renderer: keep this file free of Node imports.
// Rules for the run environment and the run properties (docs/specs/2026-10-25-environment-switcher.md).

export const ENV_NAME = /^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$/;
export const PROPERTY_NAME = /^[A-Za-z_][A-Za-z0-9_.-]{0,199}$/;
export const MAX_VALUE_LENGTH = 10_000;

const MAX_NAME_LENGTH = 200;
const MAX_ROWS = 500;
const MAX_ENVS = 100;

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
}

/** A saved secret as the window sees it: never its value. */
export interface SecretRowView {
  name: string;
  secret: true;
  state: "set" | "unreadable";
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
  | { name: string; value: string; secret: boolean }
  | { name: string; secret: true; keep: true };

export interface PropertiesEdit {
  shared: RowEdit[];
  byEnv: Record<string, RowEdit[]>;
}

const parseRow = (x: unknown): RowEdit | null => {
  if (typeof x !== "object" || x === null) return null;
  const { name, value, secret, keep } = x as Record<string, unknown>;
  if (typeof name !== "string" || name.length > MAX_NAME_LENGTH) return null;
  if (keep !== undefined) return keep === true && secret === true ? { name, secret, keep } : null;
  if (typeof value !== "string" || value.length > MAX_VALUE_LENGTH) return null;
  return typeof secret === "boolean" ? { name, value, secret } : null;
};

const parseRows = (x: unknown): RowEdit[] | null => {
  if (!Array.isArray(x) || x.length > MAX_ROWS) return null;
  const rows = x.map(parseRow);
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
      const reasons = [
        propertyNameError(row.name),
        seen.has(row.name) ? "Duplicate name" : null,
        "keep" in row
          ? storedSecrets(scope, row.name)
            ? null
            : "No saved secret to keep"
          : propertyValueError(row.value),
      ];
      seen.add(row.name);
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
