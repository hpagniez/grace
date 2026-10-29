import { cleanup, render, screen } from "@solidjs/testing-library";
import { CircleAlert, Info, Lock, TriangleAlert } from "lucide";
import { afterEach, describe, expect, it } from "vitest";
import { drawn } from "../../test/drawn";
import { FieldMessage } from "./FieldMessage";

afterEach(cleanup);

describe("FieldMessage (spec R20.2)", () => {
  it.each([
    ["hint", Info, "text-muted"],
    ["warning", TriangleAlert, "text-cancelled"],
    ["error", CircleAlert, "text-failed"],
  ] as const)("a %s is 12 px text in its tone with its own icon", (kind, icon, tone) => {
    render(() => (
      <FieldMessage kind={kind} testid="msg" id="msg-id">
        Words
      </FieldMessage>
    ));
    const box = screen.getByTestId("msg");
    expect(box.id).toBe("msg-id");
    expect(box.textContent).toBe("Words");
    for (const c of ["text-xs", tone]) expect(box.className).toContain(c);
    const svg = box.querySelector("svg")!;
    expect(drawn(svg, icon)).toBe(true);
    expect(svg.getAttribute("width")).toBe("14");
    expect(svg.getAttribute("aria-hidden")).toBe("true");
  });

  it("takes another icon, and a class for its place", () => {
    render(() => (
      <FieldMessage kind="hint" icon={Lock} testid="msg" class="w-full">
        Words
      </FieldMessage>
    ));
    const box = screen.getByTestId("msg");
    expect(drawn(box.querySelector("svg"), Lock)).toBe(true);
    expect(box.className).toContain("w-full");
    expect(box.hasAttribute("id")).toBe(false);
  });

  it("wraps long text beside its icon instead of overflowing", () => {
    render(() => (
      <FieldMessage kind="error" testid="msg">
        {"x".repeat(300)}
      </FieldMessage>
    ));
    const text = screen.getByTestId("msg").querySelector("span")!;
    expect(text.className).toContain("min-w-0");
    expect(text.className).toContain("break-words");
  });
});
