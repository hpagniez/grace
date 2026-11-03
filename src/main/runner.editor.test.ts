import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import type { TestNode } from "../shared/model";
import { cleanUp, T, setup, fixture } from "../../test/runner";

afterEach(cleanUp);

const flat = (n: TestNode): TestNode[] => [n, ...n.children.flatMap(flat)];

describe("editorFile (open-in-editor R3)", () => {
  it("resolves a tree path through the classpath, and keeps an absolute path", T, async () => {
    const { runner, last } = setup();
    const root = fixture("maven");
    await runner.open(root);
    const feature = flat(last("project").tree!).find(
      (n) => n.kind === "feature" && n.file.endsWith("users.feature"),
    )!;
    const file = path.resolve(root, "src/test/java/examples/users.feature");
    expect(runner.editorFile(feature.file)).toBe(file);
    expect(runner.editorFile(file)).toBe(file);
    expect(runner.editorFile("../x.feature")).toBeNull();
  });

  it("opens nothing with no project open", () => {
    expect(setup().runner.editorFile("examples/users.feature")).toBeNull();
  });
});
