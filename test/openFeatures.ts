import { fireEvent, screen } from "@solidjs/testing-library";
import { flush } from "solid-js";

/**
 * Opens every closed feature of the tree by its chevron (#224), the lowest first: opening one then never pushes
 * another out of the drawn rows.
 */
export function openFeatures() {
  const top = (b: HTMLElement) => parseFloat(b.closest<HTMLElement>("[role=treeitem]")!.style.top);
  for (;;) {
    const closed = screen.queryAllByRole("button", { name: /^Expand .+\.feature$/ });
    if (closed.length === 0) return;
    fireEvent.click(closed.reduce((a, b) => (top(b) > top(a) ? b : a)));
    flush();
  }
}
