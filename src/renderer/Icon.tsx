import type { IconNode } from "lucide";

/** The children of the svg: Lucide's icon data is static and ours, so it is safe as markup. */
const markup = (icon: IconNode) =>
  icon
    .map(
      ([tag, attrs]) =>
        `<${tag} ${Object.entries(attrs)
          .filter(([, v]) => v !== undefined)
          .map(([k, v]) => `${k}="${v}"`)
          .join(" ")}/>`,
    )
    .join("");

/** A Lucide icon (spec R4). Decorative: a button that holds only an icon needs an aria-label. */
export function Icon(props: { icon: IconNode; size?: 12 | 14 | 16 | 24; class?: string }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      width={props.size ?? 16}
      height={props.size ?? 16}
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
      class={props.class ? `shrink-0 ${props.class}` : "shrink-0"}
      innerHTML={markup(props.icon)}
    />
  );
}
