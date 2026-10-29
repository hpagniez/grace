import { cleanup, render, screen } from "@solidjs/testing-library";
import { afterEach, describe, expect, it } from "vitest";
import { Notice } from "./Notice";

afterEach(cleanup);

describe("Notice", () => {
  it("a warning is a status, tinted cancelled", () => {
    render(() => <Notice kind="warning" text="Folder not found" testid="notice" />);
    const box = screen.getByRole("status");
    expect(box).toBe(screen.getByTestId("notice"));
    expect(box.textContent).toBe("Folder not found");
    expect(box.className).toContain("bg-cancelled-tint");
    expect(box.className).toContain("text-cancelled");
    expect(box.querySelector("svg")!.getAttribute("aria-hidden")).toBe("true");
    expect(screen.getByText("Folder not found").className).toContain("text-ink");
  });

  it("an error is an alert, tinted failed", () => {
    render(() => <Notice kind="error" text="Karate failed" />);
    const box = screen.getByRole("alert");
    expect(box.textContent).toBe("Karate failed");
    expect(box.className).toContain("bg-failed-tint");
    expect(box.className).toContain("text-failed");
    expect(box.hasAttribute("data-testid")).toBe(false);
  });

  it("draws a different icon per kind", () => {
    const shape = (kind: "warning" | "error") => {
      const { container } = render(() => <Notice kind={kind} text="x" />);
      const html = container.querySelector("svg")!.innerHTML;
      cleanup();
      return html;
    };
    expect(shape("warning")).not.toBe(shape("error"));
  });
});
