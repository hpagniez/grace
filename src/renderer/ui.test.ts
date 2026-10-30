import { describe, expect, it } from "vitest";
import { dialog } from "./ui";

describe("dialog", () => {
  // While a closed dialog fades out its box is still drawn: it must not take a click meant for the page.
  it("takes no pointer events unless it is open", () => {
    const classes = dialog.split(/\s+/);
    expect(classes).toContain("pointer-events-none");
    expect(classes).toContain("open:pointer-events-auto");
  });
});
