// A Maven project's default karate.env, read from its pom.xml (docs/specs/2026-11-04-pom-defaults.md R3).
import { readFileSync } from "node:fs";
import path from "node:path";
import { validEnvName } from "../../shared/env";

interface El {
  name: string;
  children: El[];
  text: string;
}

// a comment, CDATA, <?…?> or <!…>, an end tag, a start or empty tag (attributes ignored), text
const TOKEN =
  /<!--[\s\S]*?-->|<!\[CDATA\[([\s\S]*?)\]\]>|<[?!][^>]*>|<\/\s*([^\s>]+)\s*>|<([^\s/>!?]+)[^>]*?(\/)?>|([^<]+)/g;

/** The root element, or null when the elements are not well nested. */
function element(xml: string): El | null {
  const stack: El[] = [{ name: "", children: [], text: "" }];
  for (const [, cdata, end, start, empty, text] of xml.matchAll(TOKEN)) {
    const at = stack[stack.length - 1];
    if (start) {
      const el = { name: start, children: [], text: "" };
      at.children.push(el);
      if (!empty) stack.push(el);
    } else if (end) {
      if (stack.length === 1 || end !== at.name) return null;
      stack.pop();
    } else at.text += cdata ?? text ?? "";
  }
  return stack.length === 1 ? (stack[0].children[0] ?? null) : null;
}

const kids = (el: El | undefined, name: string) =>
  el?.children.filter((c) => c.name === name) ?? [];
const at = (el: El | undefined, ...names: string[]) =>
  names.reduce<El | undefined>((e, n) => kids(e, n)[0], el);

const PLUGINS = ["maven-surefire-plugin", "maven-failsafe-plugin"];

/** R3: Surefire's, then Failsafe's `karate.env` entry, else the `karate.env` property; null if none Grace takes. Never throws. */
export function mavenEnv(root: string): string | null {
  let xml: string;
  try {
    xml = new TextDecoder().decode(readFileSync(path.join(root, "pom.xml"))); // drops a BOM
  } catch {
    return null;
  }
  const project = element(xml);
  if (project?.name !== "project") return null;
  const props = new Map(
    (at(project, "properties")?.children ?? []).map((p) => [p.name, p.text.trim()]),
  );
  const lists = [
    at(project, "build", "plugins"),
    at(project, "build", "pluginManagement", "plugins"),
  ];
  const entry = PLUGINS.flatMap((id) =>
    lists.flatMap((l) => kids(l, "plugin").filter((p) => at(p, "artifactId")?.text.trim() === id)),
  )
    .map((p) => at(p, "configuration", "systemPropertyVariables", "karate.env"))
    .find((e) => e !== undefined);
  const raw = entry ? entry.text.trim() : props.get("karate.env");
  if (raw === undefined || raw.split("${").length > 2) return null;
  const ref = /\$\{([^}]*)\}/.exec(raw);
  const value = !ref
    ? raw
    : props.has(ref[1])
      ? raw.replace(ref[0], () => props.get(ref[1])!)
      : null;
  if (value === null || value.includes("${")) return null;
  return validEnvName(value.trim()) ? value.trim() : null;
}
