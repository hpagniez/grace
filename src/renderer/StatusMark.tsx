import {
  Circle,
  CircleCheck,
  CircleDashed,
  CircleMinus,
  CircleSlash,
  CircleX,
  LoaderCircle,
  type IconNode,
} from "lucide";
import type { Status } from "../shared/model";
import { Icon } from "./Icon";
import { STATUS_LABEL } from "./nodes";

/** What a test is, plus the two outcomes that are not a test's status (spec R4.4). */
export type MarkStatus = Status | "skipped" | "cancelled";

export const MARK: Record<MarkStatus, { icon: IconNode; text: string; label: string }> = {
  notRun: { icon: Circle, text: "text-skipped", label: STATUS_LABEL.notRun },
  pending: { icon: CircleDashed, text: "text-skipped", label: STATUS_LABEL.pending },
  running: { icon: LoaderCircle, text: "text-running", label: STATUS_LABEL.running },
  passed: { icon: CircleCheck, text: "text-passed", label: STATUS_LABEL.passed },
  failed: { icon: CircleX, text: "text-failed", label: STATUS_LABEL.failed },
  interrupted: { icon: CircleSlash, text: "text-cancelled", label: STATUS_LABEL.interrupted },
  cancelled: { icon: CircleSlash, text: "text-cancelled", label: "Cancelled" },
  skipped: { icon: CircleMinus, text: "text-skipped", label: "Skipped" },
};

/**
 * A status by shape, colour and accessible name (spec R7.1): never by colour alone. `decorative` hides
 * it from assistive technology, for a mark that sits next to the word that names the status.
 */
export function StatusMark(props: { status: MarkStatus; size?: 14 | 16; decorative?: boolean }) {
  const mark = () => MARK[props.status];
  return (
    <span
      role={props.decorative ? undefined : "img"}
      aria-label={props.decorative ? undefined : mark().label}
      aria-hidden={props.decorative ? "true" : undefined}
      data-status={props.status}
      class={`inline-flex shrink-0 ${mark().text}`}
    >
      <Icon
        icon={mark().icon}
        size={props.size ?? 14}
        class={props.status === "running" ? "animate-spin" : undefined}
      />
    </span>
  );
}
