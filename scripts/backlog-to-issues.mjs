// One-off (ADR 0014): turns the rows of docs/features.md into GitHub issues, then rewrites the file as an ID map.
// `node scripts/backlog-to-issues.mjs` prints the plan and writes nothing; add `--apply` to create the issues.
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

const REPO = "hpagniez/grace";
const PROJECT = "Grace";
const BLOB = `https://github.com/${REPO}/blob/develop/docs/`;
const FEATURES = new URL("../docs/features.md", import.meta.url);
const MARKER = /Old backlog ID: (F\d+)\s*$/;

/** Runs `gh` without a shell and returns stdout; throws on a non-zero exit. Copy of `execRun` in bump-pr.mjs, which does not export it. */
const execRun = (cmd, args, input) =>
  execFileSync(cmd, args, { encoding: "utf8", input, maxBuffer: 64 << 20 });

/**
 * @typedef {{ id: string, title: string, feature: string, priority: "must" | "should" | "could", status: string, surface: string, spec: string, notes: string }} Row
 */

/**
 * The table rows of docs/features.md, each with its `### <id>` section from Idea notes.
 * @param {string} markdown
 * @returns {Row[]}
 */
export function parseBacklog(markdown) {
  const notes = new Map();
  const sections = markdown.split(/^### (F\d+)\b.*$/m);
  for (let i = 1; i < sections.length; i += 2) notes.set(sections[i], sections[i + 1].trim());
  return markdown
    .split("\n")
    .filter((line) => /^\| F\d+ \|/.test(line))
    .map((line) => {
      const [id, feature, priority, status, surface, spec] = line
        .split("|")
        .slice(1, -1)
        .map((cell) => cell.trim());
      return {
        id,
        title: feature.split(": ")[0].replaceAll("`", ""),
        feature,
        priority: /** @type {Row["priority"]} */ (priority),
        status,
        surface,
        spec,
        notes: notes.get(id) ?? "",
      };
    });
}

/** Links that work outside the repo: same-page anchors lose their link, relative paths point at `develop` on GitHub. */
export function absoluteLinks(md) {
  return md.replace(/\[([^\]]*)\]\(([^)\s]*)\)/g, (link, text, href) => {
    if (href.startsWith("#")) return text;
    return /^https?:/.test(href) ? link : `[${text}](${new URL(href, BLOB)})`;
  });
}

/** @param {Row} row */
export function issueBody(row) {
  const parts = [
    row.feature,
    `**Karate surface:** ${row.surface}`,
    `**Spec:** ${row.spec === "—" ? "none" : row.spec}`,
  ];
  if (row.notes) parts.push(`## Notes\n\n${row.notes}`);
  parts.push(`Old backlog ID: ${row.id}`);
  return absoluteLinks(parts.join("\n\n"));
}

/**
 * The new docs/features.md: old ID to issue.
 * @param {Row[]} rows
 * @param {Map<string, number>} numbers
 */
export function frozenBacklog(rows, numbers) {
  return [
    "# Features",
    "",
    "The backlog moved to GitHub issues on 2026-10-28 ([ADR 0014](decisions/0014-backlog-in-github-issues-and-a-project.md)). This table maps the old IDs, which older specs, plans and notes use, to their issues.",
    "",
    "| ID | Issue |",
    "|---|---|",
    ...rows.map((r) => {
      const n = numbers.get(r.id);
      return `| ${r.id} | [#${n}](https://github.com/${REPO}/issues/${n}) ${r.title} |`;
    }),
    "",
  ].join("\n");
}

/**
 * Creates the labels and the issues that do not exist yet (found by their `Old backlog ID:` line, so a second run resumes), then closes the done ones.
 * Without `apply` it only reads, and prints what it would create.
 * @param {{ apply: boolean }} o
 * @param {(cmd: string, args: string[], input?: string) => string} run
 * @returns {Map<string, number>} old ID to issue number
 */
export function migrate({ apply }, run = execRun) {
  const rows = parseBacklog(readFileSync(FEATURES, "utf8"));
  const numbers = new Map();
  const existing = JSON.parse(
    run("gh", [
      "issue",
      "list",
      "--repo",
      REPO,
      "--state",
      "all",
      "--limit",
      "1000",
      "--json",
      "number,body",
    ]),
  );
  for (const issue of existing) {
    const id = MARKER.exec(issue.body)?.[1];
    if (id) numbers.set(id, issue.number);
  }
  if (apply)
    for (const p of ["must", "should", "could"])
      run("gh", ["label", "create", p, "--repo", REPO, "--force"]);
  for (const row of rows) {
    if (apply && !numbers.has(row.id)) {
      const url = run(
        "gh",
        [
          "issue",
          "create",
          "--repo",
          REPO,
          "--title",
          row.title,
          "--label",
          row.priority,
          "--project",
          PROJECT,
          "--body-file",
          "-",
        ],
        issueBody(row),
      ).trim();
      const number = Number(url.split("/").pop());
      if (!Number.isInteger(number)) throw new Error(`${row.id}: no issue URL from gh: ${url}`);
      numbers.set(row.id, number);
    }
    console.log(
      numbers.has(row.id) ? `${row.id} #${numbers.get(row.id)}` : `${row.id} would create`,
    );
  }
  if (apply)
    for (const row of rows)
      if (row.status === "done")
        run("gh", [
          "issue",
          "close",
          String(numbers.get(row.id)),
          "--reason",
          "completed",
          "--repo",
          REPO,
        ]);
  return numbers;
}

function main() {
  const flags = process.argv.slice(2);
  if (flags.some((f) => f !== "--apply")) {
    console.error("usage: node scripts/backlog-to-issues.mjs [--apply]");
    process.exit(2);
  }
  const apply = flags.length > 0;
  const numbers = migrate({ apply });
  if (apply)
    writeFileSync(FEATURES, frozenBacklog(parseBacklog(readFileSync(FEATURES, "utf8")), numbers));
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    main();
  } catch (e) {
    console.error(e.message);
    process.exit(1);
  }
}
