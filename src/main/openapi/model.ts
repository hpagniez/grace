// What the walk (walk.ts) hands the generator (generate.ts): a spec with every `$ref` followed.

/** A schema, whatever OpenAPI version it came from. A schema reached again through its own `$ref` is the same object: a cycle is a cyclic graph. */
export interface Schema {
  /** `type` without `"null"`. */
  types: string[];
  /** `nullable: true` (3.0) or `"null"` among the types (3.1). */
  nullable: boolean;
  properties: Record<string, Schema> | null;
  required: string[];
  items: Schema | null;
  allOf: Schema[];
  oneOf: Schema[];
  anyOf: Schema[];
  format: string | null;
  minimum: number | null;
  readOnly: boolean;
  writeOnly: boolean;
  /** In the order of R5.5: `example`, the items of `examples`, `const`, `default`, `enum[0]`. */
  samples: unknown[];
}

export interface Param {
  name: string;
  in: "query" | "header" | "path" | "cookie" | "querystring";
  required: boolean;
  /** `example`, then the `value` of each of `examples`. */
  samples: unknown[];
  schema: Schema | null;
}

/** The media type the generator uses: the first JSON one, else the first. */
export interface Body {
  mediaType: string;
  json: boolean;
  /** `example`, then the `value` of each of `examples`. */
  samples: unknown[];
  schema: Schema | null;
}

export interface Operation {
  /** Lower case for the fixed methods; the key as written for an `additionalOperations` one (`COPY`). */
  method: string;
  /** As the spec writes it. */
  path: string;
  /** The first tag. */
  tag: string | null;
  summary: string | null;
  /** The path item's and the operation's, the operation's winning on the same name and `in`. */
  params: Param[];
  body: Body | null;
  /** The lowest `2xx` code, else `"2XX"`, else `null`. */
  success: { code: string; body: Body | null } | null;
  /** Scheme names of the operation's `security`, else the root's. */
  security: string[];
}

export interface Api {
  title: string;
  version: string;
  /** The first server with its variables replaced and made absolute; `null` when the spec has none. */
  server: string | null;
  /** The root's tags in order, then the tags operations use that the root lacks. */
  tags: { name: string; description: string | null }[];
  operations: Operation[];
}
