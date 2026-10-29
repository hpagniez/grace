import { cleanup, render, screen } from "@solidjs/testing-library";
import { History } from "lucide";
import { afterEach, describe, expect, it } from "vitest";
import { EmptyState } from "./EmptyState";

afterEach(cleanup);

describe("EmptyState", () => {
  it("shows a muted 24 px icon and the sentence", () => {
    const { container } = render(() => (
      <EmptyState icon={History} text="No runs yet." testid="history-empty" />
    ));
    const text = screen.getByTestId("history-empty");
    expect(text.textContent).toBe("No runs yet.");
    expect(text.className).toContain("text-muted");
    expect(text.className).toContain("text-sm");
    const icon = container.querySelector("svg")!;
    expect(icon.getAttribute("width")).toBe("24");
    expect(icon.getAttribute("aria-hidden")).toBe("true");
    expect(icon.classList.contains("text-muted")).toBe(true);
  });

  it("puts the action after the sentence", () => {
    render(() => (
      <EmptyState icon={History} text="Nothing.">
        <button type="button">Open</button>
      </EmptyState>
    ));
    const action = screen.getByRole("button", { name: "Open" });
    expect(screen.getByText("Nothing.").nextElementSibling).toBe(action);
  });

  it("has no test id unless given one", () => {
    render(() => <EmptyState icon={History} text="Nothing." />);
    expect(screen.getByText("Nothing.").hasAttribute("data-testid")).toBe(false);
  });
});
