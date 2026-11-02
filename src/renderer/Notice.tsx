import { CircleAlert, CircleCheck, TriangleAlert } from "lucide";
import { Icon } from "./Icon";

const KIND = {
  success: { tone: "bg-passed-tint text-passed", icon: CircleCheck, role: "status" },
  warning: { tone: "bg-cancelled-tint text-cancelled", icon: TriangleAlert, role: "status" },
  error: { tone: "bg-failed-tint text-failed", icon: CircleAlert, role: "alert" },
} as const;

/** A tinted message with its icon: a success or a warning is a status, an error an alert (spec R20.1). */
export function Notice(props: {
  kind: "success" | "warning" | "error";
  text: string;
  testid?: string;
}) {
  const kind = () => KIND[props.kind];
  return (
    <div
      role={kind().role}
      data-testid={props.testid}
      class={`flex items-start gap-2 rounded-lg px-3 py-2 text-sm ${kind().tone}`}
    >
      <Icon icon={kind().icon} class="mt-0.5" />
      <span class="min-w-0 break-words text-ink">{props.text}</span>
    </div>
  );
}
