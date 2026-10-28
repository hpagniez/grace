import { readFileSync } from "node:fs";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  absoluteLinks,
  frozenBacklog,
  issueBody,
  migrate,
  parseBacklog,
} from "./backlog-to-issues.mjs";

const rows = parseBacklog(readFileSync(new URL("../docs/features.md", import.meta.url), "utf8"));
const row = (id: string) => rows.find((r) => r.id === id)!;
const DOCS = "https://github.com/hpagniez/grace/blob/develop/docs/";

describe("parseBacklog", () => {
  it("reads the 45 rows of docs/features.md in order", () => {
    expect(rows.map((r) => r.id)).toEqual(
      Array.from({ length: 45 }, (_, i) => `F${String(i + 1).padStart(2, "0")}`),
    );
    expect(rows.filter((r) => r.status === "done")).toHaveLength(25);
    expect(rows.filter((r) => r.status === "idea")).toHaveLength(20);
    expect(rows.every((r) => ["must", "should", "could"].includes(r.priority))).toBe(true);
  });

  it("takes the title up to the first colon, or the whole cell", () => {
    expect(row("F11").title).toBe("HTTP inspector");
    expect(row("F02").title).toBe("Run a feature, scenario or single example row");
    for (const r of rows) {
      expect(r.title).not.toContain("`");
      expect(r.title.length).toBeLessThanOrEqual(80);
    }
  });

  it("keeps the other cells and the idea notes of the row", () => {
    expect(row("F11").spec).toContain("specs/2026-10-25-http-inspector.md");
    expect(row("F13").spec).toBe("—");
    expect(row("F01").surface).toContain("dry run");
    expect(row("F44").notes.startsWith("Measurements, causes and estimates")).toBe(true);
    expect(row("F44").notes).not.toContain("### F45");
    expect(row("F01").notes).toBe("");
  });
});

describe("absoluteLinks", () => {
  it("points repo links at develop on GitHub", () => {
    expect(absoluteLinks("[spec](specs/a.md)")).toBe(`[spec](${DOCS}specs/a.md)`);
    expect(absoluteLinks("[x](inventory/k.md#19-a)")).toBe(`[x](${DOCS}inventory/k.md#19-a)`);
    expect(absoluteLinks("[x](../AGENTS.md)")).toBe(
      "[x](https://github.com/hpagniez/grace/blob/develop/AGENTS.md)",
    );
  });

  it("drops same-page anchors to their text and leaves web links alone", () => {
    expect(absoluteLinks("See [notes](#f11-http-inspector)")).toBe("See notes");
    expect(absoluteLinks("[d](https://docs.karatelabs.io/)")).toBe(
      "[d](https://docs.karatelabs.io/)",
    );
  });
});

describe("issueBody", () => {
  it("holds the feature, surface, spec and notes, ending with the old ID", () => {
    const body = issueBody(row("F11"));
    expect(body).toContain("**Karate surface:**");
    expect(body).toContain(`**Spec:** [spec](${DOCS}specs/2026-10-25-http-inspector.md)`);
    expect(body).toContain("## Notes");
    expect(body.trimEnd().endsWith("Old backlog ID: F11")).toBe(true);
    expect(body).not.toContain("](specs/");
    expect(body).not.toContain("](#");
  });

  it("says none for a row without a spec and skips empty notes", () => {
    const body = issueBody(row("F13"));
    expect(body).toContain("**Spec:** none");
    expect(body).not.toContain("## Notes");
  });
});

/** A fake `gh`: records every call and answers `issue list` and `issue create`. */
function fakeGh(existing: { number: number; body: string }[] = []) {
  const calls: { line: string; input?: string }[] = [];
  let created = 0;
  const run = (cmd: string, args: string[], input?: string) => {
    const line = [cmd, ...args].join(" ");
    calls.push({ line, input });
    if (line.startsWith("gh issue list")) return JSON.stringify(existing);
    if (line.startsWith("gh issue create"))
      return `https://github.com/hpagniez/grace/issues/${41 + created++}\n`;
    return "";
  };
  return { run, calls, lines: (re: RegExp) => calls.map((c) => c.line).filter((l) => re.test(l)) };
}

describe("migrate", () => {
  let log: ReturnType<typeof vi.spyOn>;
  beforeEach(() => {
    log = vi.spyOn(console, "log").mockImplementation(() => {});
  });
  afterEach(() => log.mockRestore());

  it("writes nothing to GitHub without apply", () => {
    const f = fakeGh();
    migrate({ apply: false }, f.run);
    expect(f.lines(/^gh (issue (create|close)|label)/)).toEqual([]);
    expect(f.lines(/^gh issue list/)).toHaveLength(1);
    expect(log.mock.calls).toHaveLength(45);
  });

  it("creates the labels and the 45 issues, then closes the done ones", () => {
    const f = fakeGh();
    const numbers = migrate({ apply: true }, f.run);

    expect(f.lines(/^gh label create/).sort()).toEqual([
      "gh label create could --repo hpagniez/grace --force",
      "gh label create must --repo hpagniez/grace --force",
      "gh label create should --repo hpagniez/grace --force",
    ]);

    const creates = f.calls.filter((c) => c.line.startsWith("gh issue create"));
    expect(creates).toHaveLength(45);
    rows.forEach((r, i) => {
      const line = creates[i].line;
      expect(line).toContain("--repo hpagniez/grace");
      expect(line).toContain(`--title ${r.title}`);
      expect(line).toContain(`--label ${r.priority}`);
      expect(line).toContain("--project Grace");
      expect(line.endsWith("--body-file -")).toBe(true);
      expect(creates[i].input).toBe(issueBody(r));
    });

    const lines = f.calls.map((c) => c.line);
    const closes = lines.filter((l) => l.startsWith("gh issue close"));
    const done = rows.filter((r) => r.status === "done");
    expect(closes).toHaveLength(25);
    expect(closes).toEqual(
      done.map(
        (r) => `gh issue close ${numbers.get(r.id)} --reason completed --repo hpagniez/grace`,
      ),
    );
    const lastCreate = lines.findLastIndex((l) => l.startsWith("gh issue create"));
    expect(lines.findIndex((l) => l.startsWith("gh issue close"))).toBeGreaterThan(lastCreate);

    expect(numbers.size).toBe(45);
    expect(numbers.get("F01")).toBe(41);
    expect(numbers.get("F45")).toBe(85);
    expect(log).toHaveBeenCalledWith("F01 #41");
  });

  it("skips a row whose issue exists and keeps its number", () => {
    const f = fakeGh([{ number: 7, body: `${issueBody(row("F01"))}\n` }]);
    const numbers = migrate({ apply: true }, f.run);

    expect(f.lines(/^gh issue create/)).toHaveLength(44);
    expect(f.calls.some((c) => c.input?.endsWith("Old backlog ID: F01"))).toBe(false);
    expect(numbers.get("F01")).toBe(7);
    expect(numbers.get("F02")).toBe(41);
    expect(f.lines(/^gh issue close 7 /)).toHaveLength(1);
  });
});

describe("frozenBacklog", () => {
  it("maps every old ID to its issue and names ADR 0014", () => {
    const numbers = new Map(rows.map((r, i) => [r.id, 41 + i]));
    const md = frozenBacklog(rows, numbers);
    expect(md).toContain(
      "| F44 | [#84](https://github.com/hpagniez/grace/issues/84) Faster CI without dropping a test |",
    );
    expect(md).toContain("(decisions/0014-backlog-in-github-issues-and-a-project.md)");
    expect(md.startsWith("# Features\n")).toBe(true);
    expect(md.split("\n").filter((l) => /^\| F\d+ /.test(l))).toHaveLength(45);
  });
});
