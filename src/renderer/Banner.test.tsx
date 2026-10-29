import { cleanup, render, screen } from "@solidjs/testing-library";
import { afterEach, describe, expect, it } from "vitest";
import pkg from "../../package.json";
import vendor from "../../scripts/vendor.json";
import type { TestNode } from "../shared/model";
import { Banner } from "./Banner";
import { initialState, type AppState } from "./reduce";

afterEach(cleanup);

const node = (kind: TestNode["kind"], key: string, children: TestNode[] = []): TestNode => ({
  key,
  kind,
  name: key,
  file: key,
  line: kind === "scenario" ? 1 : null,
  refId: null,
  tags: [],
  children,
});
const tree = node("folder", "d:", [
  node("feature", "f:a", [node("scenario", "s:a1"), node("scenario", "s:a2")]),
  node("feature", "f:b", [node("scenario", "s:b1")]),
]);
const opened = (more: Partial<AppState> = {}): AppState => ({
  ...initialState(),
  project: { root: "D:\\work\\orders-api", layout: "standalone" },
  tree,
  ...more,
});
const text = (id: string) => screen.getByTestId(id).textContent;

describe("Banner", () => {
  it("names the pinned versions", () => {
    render(() => <Banner state={initialState()} />);
    expect(text("banner-versions")).toBe(
      `Grace ${pkg.version} · Karate ${vendor.karate.version} · Java ${vendor.jre.version}`,
    );
    const grace = screen.getByText("Grace").className;
    expect(grace).toContain("font-semibold");
    expect(grace).toContain("text-accent");
  });

  it("asks for a project with none open", () => {
    render(() => <Banner state={initialState()} />);
    expect(text("banner-project")).toBe("project  none");
    expect(screen.queryByTestId("banner-tests")).toBeNull();
    expect(text("console-state")).toBe("Open a Karate project folder");
  });

  it("is ready with a project open", () => {
    render(() => <Banner state={opened()} />);
    expect(text("banner-project")).toBe("project  D:\\work\\orders-api");
    expect(text("banner-tests")).toBe("tests    3 scenarios in 2 features");
    expect(text("console-state")).toBe("Ready. Press Run all, or ▶ on a test.");
    expect(screen.getByText("Ready.").className).toContain("text-passed");
  });

  it("says when it is finding tests or starting Karate", () => {
    render(() => <Banner state={opened({ busy: true })} />);
    expect(text("console-state")).toBe("Finding tests…");
    cleanup();
    const run = { ...initialState().run, active: true };
    render(() => <Banner state={opened({ busy: true, run })} />);
    expect(text("console-state")).toBe("Starting Karate…");
  });

  it("ends the state line with a cursor that screen readers skip and reduced motion stills", () => {
    render(() => <Banner state={opened()} />);
    const cursor = screen.getByTestId("console-cursor");
    expect(cursor.getAttribute("aria-hidden")).toBe("true");
    expect(cursor.className).toContain("animate-blink");
    expect(cursor.className).toContain("motion-reduce:animate-none");
  });

  it("drops the state line once the console has output", () => {
    render(() => <Banner state={opened({ console: ["x"] })} />);
    expect(screen.queryByTestId("console-state")).toBeNull();
    expect(text("banner-tests")).toBe("tests    3 scenarios in 2 features");
  });

  it("a long project path stays on its own line", () => {
    const root = `D:\\${"deep\\".repeat(60)}project`;
    render(() => <Banner state={opened({ project: { root, layout: "standalone" } })} />);
    const project = screen.getByTestId("banner-project");
    expect(project.textContent).toBe(`project  ${root}`);
    expect(project.nextElementSibling).toBe(screen.getByTestId("banner-tests"));
  });
});
