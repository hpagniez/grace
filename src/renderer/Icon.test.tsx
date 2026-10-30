import { cleanup, render } from "@solidjs/testing-library";
import { Play } from "lucide";
import { afterEach, describe, expect, it } from "vitest";
import { Icon } from "./Icon";

afterEach(cleanup);

const svg = (ui: () => unknown) => render(ui as () => never).container.querySelector("svg")!;

describe("Icon", () => {
  it("is a decorative 16 px Lucide svg by default", () => {
    const el = svg(() => <Icon icon={Play} />);
    expect(el.getAttribute("aria-hidden")).toBe("true");
    expect(el.getAttribute("width")).toBe("16");
    expect(el.getAttribute("height")).toBe("16");
    expect(el.getAttribute("viewBox")).toBe("0 0 24 24");
    expect(el.getAttribute("fill")).toBe("none");
    expect(el.getAttribute("stroke")).toBe("currentColor");
    expect(el.getAttribute("stroke-width")).toBe("2");
    expect(el.getAttribute("stroke-linecap")).toBe("round");
    expect(el.getAttribute("stroke-linejoin")).toBe("round");
  });

  it("takes its size and class", () => {
    const el = svg(() => <Icon icon={Play} size={14} class="text-muted" />);
    expect(el.getAttribute("width")).toBe("14");
    expect(el.getAttribute("height")).toBe("14");
    expect(el.classList.contains("text-muted")).toBe(true);
  });

  it("draws one child per node of the icon", () => {
    const el = svg(() => <Icon icon={Play} />);
    expect(el.children).toHaveLength(Play.length);
    const [tag, attrs] = Play[0];
    expect(el.children[0].tagName.toLowerCase()).toBe(tag);
    expect(el.children[0].getAttribute("d")).toBe(attrs.d);
  });
});
