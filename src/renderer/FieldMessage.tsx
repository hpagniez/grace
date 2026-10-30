import type { JSX } from "@solidjs/web";
import { CircleAlert, Info, TriangleAlert, type IconNode } from "lucide";
import { Icon } from "./Icon";

const KIND = {
  hint: { tone: "text-muted", icon: Info },
  warning: { tone: "text-cancelled", icon: TriangleAlert },
  error: { tone: "text-failed", icon: CircleAlert },
} as const;

/**
 * An inline message under a field (spec R20.2): 12 px with its icon, muted for a hint, cancelled for
 * a warning, failed for an error. `id` is what the field's aria-describedby points at.
 */
export function FieldMessage(props: {
  kind: keyof typeof KIND;
  testid: string;
  id?: string;
  icon?: IconNode;
  class?: string;
  children: JSX.Element;
}) {
  const kind = () => KIND[props.kind];
  return (
    <p
      id={props.id}
      data-testid={props.testid}
      class={`flex items-start gap-1 text-xs ${kind().tone} ${props.class ?? ""}`}
    >
      <Icon icon={props.icon ?? kind().icon} size={14} class="mt-px" />
      <span class="min-w-0 break-words">{props.children}</span>
    </p>
  );
}
