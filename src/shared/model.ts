// Shared by main and renderer: keep this file free of Node imports.

export type Layout = "maven" | "gradle" | "standalone";
export type NodeKind = "folder" | "feature" | "outline" | "scenario" | "row";
export type Status = "notRun" | "pending" | "running" | "passed" | "failed" | "interrupted";

export interface TestNode {
  /** `d:<file>` folder, `f:<file>` feature, `o:<file>#<section>` outline, `s:<file>#<section>[.<n>]` scenario or row. */
  key: string;
  kind: NodeKind;
  name: string;
  /** Folder or feature path relative to the project, '/' separators; '' for the root. */
  file: string;
  /** Line to run: scenario line, outline line, or row line; null for folders and features. */
  line: number | null;
  /** Karate refId (`[section:line]`, `[section.example:line]`); scenarios and rows only, null otherwise. */
  refId: string | null;
  /** Without '@'. */
  tags: string[];
  children: TestNode[];
}
