import type { IconNode } from "lucide";

/** Whether `svg` draws the Lucide `icon`: every attribute of its shapes is in the markup. */
export const drawn = (svg: Element | null | undefined, icon: IconNode) =>
  !!svg &&
  icon.every(([, attrs]) => Object.values(attrs).every((v) => svg.innerHTML.includes(String(v))));
