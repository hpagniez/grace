import type { JSX } from "@solidjs/web";
import type { IconNode } from "lucide";
import { Icon } from "./Icon";

/** An empty list or panel: one icon, one sentence and, when there is one, the action that fills it (spec R6.5). */
export function EmptyState(props: {
  icon: IconNode;
  text: string;
  testid?: string;
  children?: JSX.Element;
}) {
  return (
    <div class="flex flex-col items-center gap-2 p-6 text-center">
      <Icon icon={props.icon} size={24} class="text-muted" />
      <p data-testid={props.testid} class="text-sm text-muted">
        {props.text}
      </p>
      {props.children}
    </div>
  );
}
