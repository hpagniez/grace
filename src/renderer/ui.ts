/** Shared class strings besides the buttons' (spec R9.1). */

export const input =
  "h-7 rounded-md border border-control bg-surface px-2 text-sm text-ink placeholder:text-muted disabled:cursor-not-allowed disabled:opacity-40";

// A tag chip is 24 px (R3.1). Its state shows by shape as well as colour (R7.3).
const chipBase =
  "inline-flex items-center gap-1 rounded-sm border px-1.5 text-2xs font-medium transition-colors duration-(--duration-fast) ease-standard disabled:cursor-not-allowed disabled:opacity-40";
const chipPlain = "border-line bg-surface text-muted enabled:hover:bg-hover";
export const chip = `${chipBase} h-6 ${chipPlain}`;
/** A tag in a 24 px list row, 20 px so it clears the rows above and below. */
export const tagChip = `${chipBase} h-5 ${chipPlain}`;
export const chipIncluded = `${chipBase} h-6 border-accent bg-accent text-on-accent`;
export const chipExcluded = `${chipBase} h-6 border-control bg-surface text-ink line-through`;
/** A pressed filter chip (the HTTP groups): the selected fill and ink text, beside a check (R18.5). */
export const chipPressed = `${chipBase} h-6 border-line bg-selected text-ink`;

// A bottom panel tab is 32 px (R3.1); the active one has the 2 px accent line.
const tabBase =
  "inline-flex h-8 items-center gap-1.5 border-b-2 px-3 text-sm transition-colors duration-(--duration-fast) ease-standard";
export const tab = `${tabBase} border-transparent text-muted hover:text-ink`;
export const tabOn = `${tabBase} border-accent font-medium text-ink`;

export const badge = "inline-flex h-5 items-center gap-1 rounded-full px-2 text-2xs font-medium";

/** The tint of a badge, an error box or a notice: the status colour on its own solid tint (R1.4). */
export const TINT: Record<"passed" | "failed" | "skipped" | "running" | "cancelled", string> = {
  passed: "bg-passed-tint text-passed",
  failed: "bg-failed-tint text-failed",
  skipped: "bg-skipped-tint text-skipped",
  running: "bg-running-tint text-running",
  cancelled: "bg-cancelled-tint text-cancelled",
};

/** Error and code boxes. */
export const box = "rounded-lg bg-sunken p-2 font-mono text-xs";

/**
 * A modal <dialog> (R19.1). It enters over --duration-base with a fade and a scale from 0.98, and its
 * backdrop fades (R5.2). transition-discrete keeps both displayed while they fade out, over the
 * shorter --duration-fast. Checked in Chromium: the enter and the exit both animate.
 */
export const dialog = [
  "m-auto rounded-xl border border-line bg-raised p-5 text-ink shadow-float ease-standard",
  "scale-[0.98] opacity-0 transition-[opacity,scale,display,overlay] transition-discrete duration-(--duration-fast)",
  "open:scale-100 open:opacity-100 open:duration-(--duration-base)",
  "starting:open:scale-[0.98] starting:open:opacity-0",
  "backdrop:bg-(--backdrop) backdrop:opacity-0 backdrop:transition-[opacity,display,overlay] backdrop:transition-discrete backdrop:duration-(--duration-fast)",
  "open:backdrop:opacity-100 open:backdrop:duration-(--duration-base) starting:open:backdrop:opacity-0",
].join(" ");

/** The count in a tab or a button, e.g. the failed tests behind Re-run failed. */
export const countPill =
  "inline-flex h-4 min-w-5 items-center justify-center rounded-full bg-selected px-1.5 text-2xs font-medium text-muted tabular-nums";
